# Alloy — developer task runner.
#
#   just              list every recipe
#   just gate         run the full local quality gate (mirrors CI)
#   just run --script scripts/hello.rhai
#   just test engine
#
# Thin wrappers over cargo + pnpm. `just` replaces the old Makefile: this is a
# command runner, not a compiler recipe, and `just` says so honestly (no phony
# targets, no implicit file rules). Source of truth stays lefthook.yml,
# package.json and .github/workflows/ci.yml.

set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

cargo := env_var_or_default("CARGO", "cargo")
pnpm  := env_var_or_default("PNPM", "pnpm")

# Clippy invocation used everywhere (matches lefthook.yml / CI / package.json).
clippy_flags := "--workspace --all-targets --all-features -- -D warnings"
# Coverage gate: line coverage of `engine`, threshold from roadmap §5.
cov_pkg := "engine"
cov_min := "85"

# Show this help (default recipe).
_default:
    @just --list --unsorted

# --- build / run --------------------------------------------------------

# Debug build of the whole workspace
build:
    {{cargo}} build --workspace

# Optimised build of the whole workspace
release:
    {{cargo}} build --workspace --release

# Fast type-check, all targets
check:
    {{cargo}} check --workspace --all-targets

# Run the alloy binary   (just run --help)
run *args:
    {{cargo}} run -p alloy -- {{args}}

# Run the bundled example muscle script through alloy
script:
    {{cargo}} run -p alloy -- --script scripts/hello.rhai

# --- tests ------------------------------------------------------------

# Run tests   (just test engine "-- name"  →  scoped;  just test  →  workspace)
test crate="" *args="":
    {{cargo}} test {{ if crate == "" { "--workspace" } else { "-p " + crate } }} {{args}}

# Run the RuntimeEngine port conformance suites (Mock + Rhai)
conformance:
    {{cargo}} test -p engine -p rhai-runtime

# --- formatting / lint ---------------------------------------------

# Auto-format Rust + Markdown
fmt:
    {{cargo}} fmt --all
    {{pnpm}} format:md

# Verify formatting without writing (Rust + Markdown)
fmt-check:
    {{cargo}} fmt --all --check
    {{pnpm}} format:check

# Clippy (warnings = errors) + markdownlint
lint:
    {{cargo}} clippy {{clippy_flags}}
    {{pnpm}} lint:md

# Apply clippy + rustfmt autofixes to the working tree
fix:
    {{cargo}} clippy --workspace --all-targets --all-features --fix --allow-dirty --allow-staged
    {{cargo}} fmt --all

# --- quality gates -------------------------------------------------

# Full local gate (CI minus the 3-OS matrix, cargo-fuzz's 10min/target runs, and benches)
gate: fmt-check lint check test deny coverage arch layering css-conformance unsafe-audit
    @echo "✓ all local gates passed"

# Alias for `gate`
ci: gate

# unsafe-by-threat-surface audit (ADR-0018). Blocking since v0.5 Phase P —
# see ci/unsafe_audit.sh for the forbid-only sweep + direct-dependency scan.
unsafe-audit:
    @command -v cargo-geiger >/dev/null || {{cargo}} install cargo-geiger --locked
    ./ci/unsafe_audit.sh

# Hook-dispatch overhead vs. the committed baseline (PRD-001 N-01, <10us).
hook-benchmark:
    ./ci/hook_benchmark.sh

# ADR-0018 row-1 decoders under cargo-fuzz, 10 min/target (requires nightly
# and cargo-fuzz — not part of `just gate`, CI-only otherwise).
fuzz target="":
    @command -v cargo-fuzz >/dev/null || {{cargo}} install cargo-fuzz --locked
    {{ if target == "" { "for t in inflate png_decode css_parse; do cargo +nightly fuzz run $t -- -max_total_time=600; done" } else { "cargo +nightly fuzz run " + target + " -- -max_total_time=600" } }}

# Supply-chain audit: licenses, advisories, bans, sources
deny:
    @command -v cargo-deny >/dev/null || { echo "cargo-deny not found — run: just setup"; exit 1; }
    {{cargo}} deny check

# Just the security-advisory half of `deny` (fast)
audit:
    @command -v cargo-deny >/dev/null || { echo "cargo-deny not found — run: just setup"; exit 1; }
    {{cargo}} deny check advisories

# Line coverage of the engine crate, and of css/network/window/html's domain/
# (v0.5 Phase P); both fail under the roadmap threshold.
coverage:
    @command -v cargo-llvm-cov >/dev/null || { echo "cargo-llvm-cov not found — run: just setup"; exit 1; }
    {{cargo}} llvm-cov --package {{cov_pkg}} --all-features --summary-only --fail-under-lines {{cov_min}}
    {{cargo}} llvm-cov --package css --package network --package window --package html --all-features \
        --ignore-filename-regex '(/application/|/infrastructure/)' \
        --summary-only --fail-under-lines {{cov_min}}

# Write an HTML coverage report under target/llvm-cov/html
coverage-html:
    @command -v cargo-llvm-cov >/dev/null || { echo "cargo-llvm-cov not found — run: just setup"; exit 1; }
    {{cargo}} llvm-cov --package {{cov_pkg}} --all-features --html
    @echo "report: target/llvm-cov/html/index.html"

# CSS + HTML support manifests: MANIFEST.md, the registries and the parser
# must agree in every direction (relatório §2.8:350-354). No bless path by design.
css-conformance:
    {{cargo}} test -p css --test manifest_runner
    {{cargo}} test -p html --test manifest_runner

# Architecture gate: layers, dependencies, `tracing` + no-`unwrap` (arch-lint)
arch:
    @command -v arch-lint >/dev/null || { echo "arch-lint not found — run: just setup"; exit 1; }
    arch-lint check

# Prove core/engine links no script interpreter (ADR-0002 / ADR-0011 item 2) and
# that core/runtime/rhai names no domain crate (v0.5 report §2.12 — the R split).
# Renamed from `no-engine` in v0.5 Phase P (kept as an alias below) — it now
# covers every subsystem's layering rule, not only the engine's.
layering:
    @{{cargo}} tree -p engine --edges normal --prefix none
    @if {{cargo}} tree -p engine --edges normal --prefix none \
        | grep -Eiq '^(rhai|boa_engine|rquickjs|deno_core|v8|mlua|rlua) '; then \
        echo "✗ core/engine linked a script interpreter"; exit 1; \
    else \
        echo "✓ core/engine is interpreter-free"; \
    fi
    @if {{cargo}} tree -p rhai-runtime --edges normal --prefix none | grep -Eq '^dom '; then \
        echo "✗ core/runtime/rhai depends on core/dom — the bridge belongs in rhai-bindings"; exit 1; \
    else \
        echo "✓ core/runtime/rhai is domain-crate free"; \
    fi
    @if {{cargo}} tree -p css --edges normal --prefix none | grep -Eiq '^(engine|rhai|rhai-runtime|rhai-bindings) '; then \
        echo "✗ core/css linked the engine or a script runtime"; exit 1; \
    else \
        echo "✓ core/css is engine/rhai free"; \
    fi
    @if {{cargo}} tree -p html --edges normal --prefix none | grep -Eiq '^(engine|rhai|rhai-runtime|rhai-bindings) '; then \
        echo "✗ core/html linked the engine or a script runtime"; exit 1; \
    else \
        echo "✓ core/html is engine/rhai free"; \
    fi
    @if {{cargo}} tree -p network --edges normal --prefix none | grep -Eiq '^(engine|rhai|rhai-runtime|rhai-bindings|dom|css|graphics) '; then \
        echo "✗ core/network linked the engine, a script runtime or another subsystem"; exit 1; \
    else \
        echo "✓ core/network is engine/rhai/subsystem free"; \
    fi
    @if {{cargo}} tree -p window --edges normal --prefix none | grep -Eiq '^(engine|rhai|rhai-runtime|rhai-bindings|dom|css|graphics|network) '; then \
        echo "✗ core/window linked the engine, a script runtime or another subsystem"; exit 1; \
    else \
        echo "✓ core/window is engine/rhai/subsystem free"; \
    fi
    @if {{cargo}} tree -p window --no-default-features --edges normal --prefix none | grep -Eiq '^(winit|softbuffer) '; then \
        echo "✗ core/window --no-default-features still links winit/softbuffer"; exit 1; \
    else \
        echo "✓ core/window --no-default-features (no-window) links neither winit nor softbuffer"; \
    fi

# Alias for `layering` (pre-v0.5-Phase-P name).
no-engine: layering

# --- web-tests server -------------------------------------------------

# Start Caddy local test server via Docker Compose
serve-tests:
    docker compose -f web-tests/docker-compose.yml up -d
    @echo "✓ Caddy web test server running at http://localhost:8080"

# Stop Caddy local test server
stop-tests:
    docker compose -f web-tests/docker-compose.yml down
    @echo "✓ Caddy web test server stopped"

# Run smoke test verifying Caddy endpoints and alloy rendering
test-web:
    ./scripts/smoke_web_tests.sh

# --- misc -------------------------------------------------------------

# Dependency tree for the whole workspace
tree:
    {{cargo}} tree

# Build API docs and open them
doc:
    {{cargo}} doc --workspace --no-deps --open

# Refresh Cargo.lock (it is versioned — commit the change deliberately)
update:
    {{cargo}} update

# Install dev tooling: pnpm deps, rust components, cargo-deny, cargo-llvm-cov, arch-lint, git hooks
setup:
    {{pnpm}} install --frozen-lockfile
    rustup component add rustfmt clippy llvm-tools-preview
    @command -v cargo-deny     >/dev/null || {{cargo}} install --locked cargo-deny
    @command -v cargo-llvm-cov >/dev/null || {{cargo}} install --locked cargo-llvm-cov
    @command -v arch-lint      >/dev/null || {{cargo}} install --locked arch-lint-cli
    {{pnpm}} exec lefthook install

# (Re)install the lefthook git hooks
hooks:
    {{pnpm}} exec lefthook install

# Remove cargo build artifacts
clean:
    {{cargo}} clean

# clean + drop node_modules
distclean: clean
    rm -rf node_modules

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

# Full local gate (CI minus the 3-OS matrix)
gate: fmt-check lint check test deny coverage arch
    @echo "✓ all local gates passed"

# Alias for `gate`
ci: gate

# Supply-chain audit: licenses, advisories, bans, sources
deny:
    @command -v cargo-deny >/dev/null || { echo "cargo-deny not found — run: just setup"; exit 1; }
    {{cargo}} deny check

# Just the security-advisory half of `deny` (fast)
audit:
    @command -v cargo-deny >/dev/null || { echo "cargo-deny not found — run: just setup"; exit 1; }
    {{cargo}} deny check advisories

# Line coverage of the engine crate; fails under the roadmap threshold
coverage:
    @command -v cargo-llvm-cov >/dev/null || { echo "cargo-llvm-cov not found — run: just setup"; exit 1; }
    {{cargo}} llvm-cov --package {{cov_pkg}} --all-features --summary-only --fail-under-lines {{cov_min}}

# Write an HTML coverage report under target/llvm-cov/html
coverage-html:
    @command -v cargo-llvm-cov >/dev/null || { echo "cargo-llvm-cov not found — run: just setup"; exit 1; }
    {{cargo}} llvm-cov --package {{cov_pkg}} --all-features --html
    @echo "report: target/llvm-cov/html/index.html"

# Architecture gate: layers, dependencies, `tracing` + no-`unwrap` (arch-lint)
arch:
    @command -v arch-lint >/dev/null || { echo "arch-lint not found — run: just setup"; exit 1; }
    arch-lint check

# Prove core/engine links no script interpreter (ADR-0002 / ADR-0011 item 2) and
# that core/runtime/rhai names no domain crate (v0.5 report §2.12 — the R split)
no-engine:
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

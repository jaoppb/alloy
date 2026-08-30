# Alloy — developer task runner.
#
#   make            list every target
#   make gate       run the full local quality gate (mirrors CI)
#   make run ARGS="--script scripts/hello.rhai"
#   make test CRATE=engine
#
# Thin wrappers over cargo + pnpm; the source of truth stays lefthook.yml,
# package.json and .github/workflows/ci.yml.

MAKEFLAGS += --no-print-directory
SHELL := /bin/bash
.SHELLFLAGS := -eu -o pipefail -c

CARGO ?= cargo
PNPM  ?= pnpm

# Clippy invocation used everywhere (matches lefthook.yml / CI / package.json).
CLIPPY_FLAGS := --workspace --all-targets --all-features -- -D warnings
# Coverage gate: line coverage of `engine`, threshold from roadmap §5.
COV_PKG := engine
COV_MIN := 85

# Pass-through knobs:
#   make run  ARGS="--version"
#   make test CRATE=engine  ARGS="-- conformance"
ARGS  ?=
CRATE ?=
crate_flag := $(if $(CRATE),-p $(CRATE),--workspace)

.DEFAULT_GOAL := help

.PHONY: help build release check run script test conformance \
        fmt fmt-check lint fix \
        gate ci deny audit coverage coverage-html no-engine \
        tree doc update setup hooks clean distclean

help: ## Show this help
	@echo "Alloy — make targets:"
	@echo
	@awk 'BEGIN{FS=":.*## "} /^[a-zA-Z0-9_-]+:.*## /{printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2}' $(MAKEFILE_LIST) | sort

# --- build / run --------------------------------------------------------

build: ## Debug build of the whole workspace
	$(CARGO) build --workspace

release: ## Optimised build of the whole workspace
	$(CARGO) build --workspace --release

check: ## Fast type-check, all targets
	$(CARGO) check --workspace --all-targets

run: ## Run the alloy binary   (make run ARGS="--help")
	$(CARGO) run -p alloy -- $(ARGS)

script: ## Run the bundled example muscle script through alloy
	$(CARGO) run -p alloy -- --script scripts/hello.rhai

# --- tests ------------------------------------------------------------

test: ## Run tests   (make test CRATE=engine ARGS="-- name")
	$(CARGO) test $(crate_flag) $(ARGS)

conformance: ## Run the RuntimeEngine port conformance suites (Mock + Rhai)
	$(CARGO) test -p engine -p rhai-runtime

# --- formatting / lint ---------------------------------------------

fmt: ## Auto-format Rust + Markdown
	$(CARGO) fmt --all
	$(PNPM) format:md

fmt-check: ## Verify formatting without writing (Rust + Markdown)
	$(CARGO) fmt --all --check
	$(PNPM) format:check

lint: ## Clippy (warnings = errors) + markdownlint
	$(CARGO) clippy $(CLIPPY_FLAGS)
	$(PNPM) lint:md

fix: ## Apply clippy + rustfmt autofixes to the working tree
	$(CARGO) clippy --workspace --all-targets --all-features --fix --allow-dirty --allow-staged
	$(CARGO) fmt --all

# --- quality gates -------------------------------------------------

gate: fmt-check lint check test deny coverage no-engine ## Full local gate (CI minus the 3-OS matrix)
	@echo "✓ all local gates passed"

ci: gate ## Alias for `gate`

deny: ## Supply-chain audit: licenses, advisories, bans, sources
	@command -v cargo-deny >/dev/null || { echo "cargo-deny not found — run: make setup"; exit 1; }
	$(CARGO) deny check

audit: ## Just the security-advisory half of `deny` (fast)
	@command -v cargo-deny >/dev/null || { echo "cargo-deny not found — run: make setup"; exit 1; }
	$(CARGO) deny check advisories

coverage: ## Line coverage of the engine crate; fails under the roadmap threshold
	@command -v cargo-llvm-cov >/dev/null || { echo "cargo-llvm-cov not found — run: make setup"; exit 1; }
	$(CARGO) llvm-cov --package $(COV_PKG) --all-features --summary-only --fail-under-lines $(COV_MIN)

coverage-html: ## Write an HTML coverage report under target/llvm-cov/html
	@command -v cargo-llvm-cov >/dev/null || { echo "cargo-llvm-cov not found — run: make setup"; exit 1; }
	$(CARGO) llvm-cov --package $(COV_PKG) --all-features --html
	@echo "report: target/llvm-cov/html/index.html"

no-engine: ## Prove core/engine links no script interpreter (ADR-0002 / ADR-0011)
	@$(CARGO) tree -p engine --edges normal --prefix none
	@if $(CARGO) tree -p engine --edges normal --prefix none \
		| grep -Eiq '^(rhai|boa_engine|rquickjs|deno_core|v8|mlua|rlua) '; then \
		echo "✗ core/engine linked a script interpreter"; exit 1; \
	else \
		echo "✓ core/engine is interpreter-free"; \
	fi

# --- misc -------------------------------------------------------------

tree: ## Dependency tree for the whole workspace
	$(CARGO) tree

doc: ## Build API docs and open them
	$(CARGO) doc --workspace --no-deps --open

update: ## Refresh Cargo.lock (it is versioned — commit the change deliberately)
	$(CARGO) update

setup: ## Install dev tooling: pnpm deps, rust components, cargo-deny, cargo-llvm-cov, git hooks
	$(PNPM) install --frozen-lockfile
	rustup component add rustfmt clippy llvm-tools-preview
	@command -v cargo-deny    >/dev/null || $(CARGO) install --locked cargo-deny
	@command -v cargo-llvm-cov >/dev/null || $(CARGO) install --locked cargo-llvm-cov
	$(PNPM) exec lefthook install

hooks: ## (Re)install the lefthook git hooks
	$(PNPM) exec lefthook install

clean: ## Remove cargo build artifacts
	$(CARGO) clean

distclean: clean ## clean + drop node_modules
	rm -rf node_modules

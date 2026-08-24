# ADR-0006: Cargo Workspace Modular Crate Structure

- **Status**: Accepted
- **Deciders**: Architecture Team
- **Date**: 2026-08-22 (Updated: 2026-08-23)

---

## Context and Problem Statement

Alloy's codebase must balance compile times, modularity, unit testability, and clear separation of concerns across its
browser domains. How should Cargo crates, package names, and workspace members be organized?

---

## Decision Drivers

- Fast incremental compilation times.
- Clear module ownership: Domain data, web content execution, engine abstractions, and concrete backends must be
  separated.
- Ergonomic internal crate naming (bare names in private workspace).
- Independent testing of individual subsystems without requiring the entire browser stack.

---

## Considered Options

- **Option 1**: Fine-grained Cargo workspace with bare package names (`dom`, `html`, `css`, `graphics`, `window`,
  `network`, `js`, `engine`, `rhai-runtime`, `devtools`, `extension`).
- **Option 2**: Prefixed package names (`alloy-dom`, `alloy-html`, etc.) for every crate.
- **Option 3**: Single monolithic crate with internal Rust modules (`mod dom`, `mod html`, etc.).

---

## Decision Outcome

Chosen option: **Option 1 (Fine-grained workspace with bare package names)**.

### Workspace Layout & Crate Responsibilities

```text
alloy/
├── Cargo.toml                 # Workspace root: explicit members + the core/runtime/* glob
├── core/
│   ├── engine/                # Package: engine (Abstract RuntimeEngine traits & Capability sandbox)
│   ├── runtime/               # Script engine backends (no manifest of its own)
│   │   └── rhai/              # Package: rhai-runtime (Rhai engine backend for browser muscle)
│   ├── js/                    # Package: js (Web content ECMAScript runtime & DOM script binding)
│   ├── dom/                   # Package: dom (Pure DOM Node tree and mutations)
│   ├── html/                  # Package: html (HTML5 tokenization & tree construction)
│   ├── css/                   # Package: css (CSS styling, selectors, rules & cascade)
│   ├── graphics/              # Package: graphics (DisplayList, Vulkano & OpenGL renderers)
│   ├── window/                # Package: window (OS windowing & event pump abstraction)
│   └── network/               # Package: network (HTTP/TLS client, URL parsing & cache)
├── devtools/                  # Package: devtools (Introspection server & hot-reload orchestration)
└── extension/                 # Package: extension (Extensions host & WebExtensions API bridge)
```

Workspace members are listed explicitly rather than via a bare `core/*` glob: that glob also matches the manifest-less
`core/runtime/` directory and aborts every Cargo command. New script backends are picked up by the `core/runtime/*`
glob.

### Script Execution Separation

1. **Web Content JavaScript (`core/js`)**: Responsible for standard Web APIs and execution of untrusted `<script>` tags
   found on web pages.
2. **Browser Muscle Engine (`core/engine` + `core/runtime/rhai`)**: Responsible for internal browser automation, UI
   layout orchestration, event routing, and hot-swappable user customization scripts.

### Consequences

- **Positive**:
    - Bare package names provide concise, ergonomic internal dependency paths (`dom = { path = "../dom" }`).
    - Clean separation between Web content scripting (`core/js`) and browser subsystem customization (`core/engine` +
      `core/runtime/rhai`).
    - Fast incremental builds and fine-grained unit testing across all crates.
- **Negative**:
    - Managing individual `Cargo.toml` manifests across 11 workspace crates.

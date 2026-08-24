# PRD-001: Alloy Core System & Modular Architecture

- **Status**: Accepted
- **Author**: Core Architecture Team
- **Date**: 2026-08-22
- **Target Release**: v0.1.0-alpha

---

## 1. Executive Summary

Alloy is an infinitely malleable, modular web browser engineered with a high-performance Rust core and driven by
swappable, hot-reloadable scripting engines (starting with Rhai). Rather than a monolithic application, Alloy functions
as a browser construction kit where all domain state (DOM, CSS, Layout, Network, Windows) is represented as memory-safe
Rust structures, while behavior, orchestration, and pipelines are defined in user-editable scripts.

---

## 2. Problem Statement & Motivation

Traditional browsers (Chromium, Gecko, WebKit) are monolithic and tightly coupled. Customizing core behaviors (such as
network routing, rendering passes, tab lifecycle, or devtools inspection) requires modifying millions of lines of C++
code, recompiling, and managing complex build systems.

Power users, developers, and researchers lack a browser architecture that enables:

1. **Granular malleability**: Modifying individual subsystem behaviors (e.g. swapping layout algorithms or intercepting
   requests) without rebuilding the binary.
2. **Deterministic safety**: Ensuring scripting bugs or user experiments cannot crash the core host or corrupt memory.
3. **Engine agnosticism**: Scripting behavior across different language engines without changing domain data
   representations.

---

## 3. Goals and Non-Goals

### 3.1 Goals

- **Fractal Modularity**: Every major subsystem (`dom`, `html`, `css`, `graphics`, `window`, `network`, `devtools`,
  `extension`) is an independent Cargo crate exposing pure domain types and contracts.
- **Skeleton & Muscle Pattern**: Rust manages data structures, memory allocation, concurrency, and hardware I/O; dynamic
  scripts define behavioral policies and execution flow.
- **Script Replacement**: Users can override default subsystem scripts at startup or during execution.
- **Hot-Reloading**: Script modifications on disk or via DevTools update active subsystems with zero process downtime.
- **Fault-Tolerant Isolation**: Malformed or crashing scripts are trapped at capability sandboxes and gracefully fall
  back to safe defaults.

### 3.2 Non-Goals

- Full initial compatibility with 100% of Web Standards (CSS3 animations, complex WebGL2) in v0.1.
- Replacing Rust domain structs with dynamic interpreted objects.
- Unsandboxed direct hardware or arbitrary OS execution from untrusted user scripts.

---

## 4. System Personas & Use Cases

| Persona                   | Primary Goal                                | Usage in Alloy                                                                          |
| ------------------------- | ------------------------------------------- | --------------------------------------------------------------------------------------- |
| **End-User / Power User** | Tailor browser workflow and UI              | Swapping Rhai scripts for tabs, keyboard shortcuts, bookmarking, and window layouts.    |
| **Security Researcher**   | Inspect and sanitize web traffic & DOM      | Intercepting network requests, altering DOM sanitization pipelines, inspecting ASTs.    |
| **Engine Developer**      | Experiment with rendering / parsing engines | Swapping the HTML parser or CSS cascade resolver with custom Rhai/Wasm implementations. |
| **AI Agent / Automator**  | Headless automated web workflows            | Running Alloy in headless mode with programmable script hooks.                          |

---

## 5. Functional Requirements

### 5.1 Modular Workspace Crates

- `core/dom`: Node trees, Element attributes, Event targets, traversal APIs.
- `core/html`: HTML tokenization, parser state machines, tree construction.
- `core/css`: CSS tokenizer, selectors, rule sets, cascade resolver.
- `core/graphics`: 2D draw commands, display lists, Vulkan/OpenGL backends.
- `core/window`: Platform windowing abstraction, event pumps, input handling.
- `core/network`: Request/response structs, protocol handlers, connection pools.
- `core/js`: Web content JavaScript runtime & DOM script execution.
- `core/engine`: Generic runtime engine traits, execution context, capability sandboxing.
- `core/runtime/rhai`: Concrete Rhai script engine backend for browser subsystem muscle.
- `devtools`: Introspection server, event logs, AST inspect, script hot-reload trigger.
- `extension`: WebExtension and native extension host bridge.

### 5.2 Dynamic Hook Lifecycle

Subsystems expose standard lifecycle hooks invoked through the engine trait:

1. `on_init()`: Initialize subsystem state.
2. `on_event(event: Event)`: Handle incoming events (I/O, input, DOM mutations).
3. `on_process(state: &mut DomainState)`: Execute transformation logic.
4. `on_reload()`: Rebind scope after a hot reload.

---

## 6. Non-Functional Requirements

- **Performance**: Rust-to-Engine invocation overhead must be minimal (<10μs per event hook).
- **Memory Safety**: Zero unsafe memory operations exposed to script runtimes.
- **Reliability**: 100% crash isolation between independent subsystem scripts.
- **Testability**: Every domain crate must be testable with and without script engines attached.
- **SPDD Compliance**: All functional increments must have corresponding SPDD Prompts (`spdd/prompt/*.md`).

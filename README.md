# 🌐 Alloy

> **The infinitely malleable, fully modular web browser.** > Built with a high-performance Rust core and driven by Rhai
> scripting.

Alloy is not just another web browser; it is a browser construction kit. By strictly dividing the architecture between
rigid, memory-safe interfaces (Rust) and flexible, hot-swappable behavioral logic (Rhai), Alloy allows users to rewrite
how their browser works on the fly, without ever recompiling the core.

From the network stack down to the UI rendering, every module in Alloy contains smaller, replaceable sub-modules. If you
don't like how tab management works, write a new script. If you want a completely different rendering pipeline, swap the
module.

---

## ✨ Key Features

- **🦀 Rust Core:** All base interfaces, structs, enums, and heavy computational boundaries are defined in Rust,
  ensuring memory safety, concurrency, and raw performance.
- **📜 Rhai Scripted Behavior:** Every piece of logic, routing, UI behavior, and module interaction is handled by
  [Rhai](https://rhai.rs/) scripts.
- **🧩 Fractal Modularity:** Modules aren't just top-level plugins. Every system in Alloy is made of smaller,
  replaceable Rhai modules. Swap out the entire UI, or just swap out the URL bar logic.
- **🛠️ Ultimate Customization:** Create a minimalist terminal-based browser, a heavy power-user GUI, or a headless
  scraper, all using the exact same Alloy core.

---

## 🏗️ Architecture Overview

Alloy uses a **Skeleton and Muscle** architectural pattern:

1. **The Skeleton (Rust):** Defines the strict contracts. It provides the `structs` that hold state, the `enums` that
   define events, and the `traits` that modules must implement. It also handles the heavy lifting (e.g., interfacing
   with the GPU or OS-level network sockets).
2. **The Muscle (Rhai):** Hooks into the Rust interfaces. When a network request is made, a Rust event fires, but a Rhai
   script decides _what to do_ with that event.

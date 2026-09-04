//! # `window` — the `WindowSystem` / `Presenter` ports
//!
//! A native window and its event queue on one side (`WindowSystem`, mechanism,
//! owns the main thread), a place to blit pixels on the other (`Presenter`,
//! mechanism, may live on a render worker) — `PRD-010`, `ADR-0011`. Policy
//! (when to repaint, how to react to a resize) is Phase M's `.rhai` muscle,
//! not this crate.
//!
//! This crate names **no** engine type, no `rhai`, no `dom`, no `css`, no
//! `graphics`. It depends on `thiserror` and — only in the `winit-system`
//! build (the default) — on `winit` and `softbuffer`. The `no-engine` CI job
//! holds that line, and `no-window` (`--no-default-features`) proves it links
//! neither `winit` nor `softbuffer` at all.
//!
//! ## Layout (`ADR-0010` §1)
//!
//! - [`domain`] — zero-I/O value objects: [`WindowEvent`] / [`PointerButton`],
//!   [`SurfaceSize`] / [`ScaleFactor`] / [`PhysicalPosition`], [`FrameView`],
//!   [`WindowAttributes`] / [`WindowTitle`] / [`WindowId`], [`KeyCode`], and
//!   the typed [`WindowError`] / [`WindowOperation`].
//! - [`application`] — the two ports ([`WindowSystem`], [`Presenter`],
//!   [`PumpStatus`]) and the [`conformance`] suite.
//! - [`infrastructure`] — the `winit` + `softbuffer` adapter
//!   ([`WinitSystem`], [`SoftbufferPresenter`], the `winit`-event mapping) and
//!   the headless reference ([`HeadlessWindowSystem`], [`RecordingPresenter`]).
//!
//! [`FrameView`] is deliberately not `graphics::Framebuffer`: naming that type
//! here would put `core/graphics` in this crate's dependency graph. A caller
//! that already depends on both crates (later, `alloy`) builds a `FrameView`
//! from a `graphics::Framebuffer` in one borrow, right before calling
//! [`Presenter::present`] — the seam that keeps the two ports decoupled.
//!
//! ## Threading (`ADR-0019`)
//!
//! The window event loop is the **sole owner of the main thread**.
//! [`WindowSystem`] is deliberately **not** `Send + Sync` — [`WinitSystem`]
//! must be constructed on the main thread on macOS and Windows, and stay
//! there. [`Presenter`] **is** `Send`: [`SoftbufferPresenter`] may be handed
//! to a render worker. Blocking I/O (`core/network`'s `HttpTransport`, DNS,
//! file reads) never runs on this loop's thread — that is Phase I4's
//! `alloy/src/application/event_loop.rs`, not this crate.
//!
//! ## `unsafe` (`ADR-0018`)
//!
//! This crate is `#![forbid(unsafe_code)]`. `winit` and `softbuffer` are the
//! row-3 nominal exception — platform windowing/surface FFI with no
//! `unsafe`-free alternative across Linux/macOS/Windows, processing no
//! attacker-controlled bytes — recorded in `unsafe-allowlist.toml`.
//!
//! ## Contract record
//!
//! This crate is the `WindowSystem` / `Presenter` port under the `ADR-0011`
//! Replaceable Port Contract, and freezes at integration point `I4`.
//! `docs/architecture/window-system-port-contract.md` records the state of
//! all seven items from that point on. A change after the freeze also needs a
//! migration note in `PRD-010`.

#![forbid(unsafe_code)]
// Every fallible function here documents its failures through the typed
// `WindowError` variant it returns; a prose `# Errors` section on each would
// restate the enum. Same call, same reason, as `core/dom/src/lib.rs:24`,
// `core/css/src/lib.rs:44` and `core/network/src/lib.rs`.
#![allow(clippy::missing_errors_doc)]

pub mod application;
pub mod domain;
pub mod infrastructure;

/// The observable version of this port's event and attribute vocabulary.
///
/// `ADR-0011` item 3. Bumped on any change a backend or a caller could
/// notice; **frozen at I4**, after which a change also needs a migration note
/// in `PRD-010`.
pub const PORT_SCHEMA_VERSION: u32 = 1;

pub use application::conformance;
pub use application::{Presenter, PumpStatus, WindowSystem};
pub use domain::attributes::{WindowAttributes, WindowId, WindowTitle};
pub use domain::error::{WindowError, WindowOperation};
pub use domain::event::{PointerButton, WindowEvent};
pub use domain::frame::FrameView;
pub use domain::key::KeyCode;
pub use domain::surface::{PhysicalPosition, ScaleFactor, SurfaceSize};
pub use infrastructure::{HeadlessWindowSystem, RecordingPresenter};

#[cfg(feature = "winit-system")]
pub use infrastructure::{SoftbufferPresenter, WinitSystem};

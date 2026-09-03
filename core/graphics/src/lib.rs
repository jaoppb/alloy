//! # `graphics` — declarative display lists and the render-backend port
//!
//! The **Skeleton-side** rasterization seam (`ADR-0003`): subsystems describe
//! *what* to paint as an immutable [`DisplayList`], and a [`RenderBackend`]
//! decides *how*. Layout never issues a draw call, and the backend never sees a
//! DOM node or a style rule — which is what makes the GPU tiers of `F12`
//! swappable without layout noticing (`PRD-005:62-63`, roadmap point `I6`).
//!
//! This crate names no engine type and has no script bridge. Making a display
//! list scriptable is `core/runtime/rhai`'s job at `I2b`, exactly as making a
//! DOM node scriptable was at `I1` (v0.3 report decision 2.1). Its only
//! dependencies are `thiserror` and, from `F4b`, `ttf-parser`.
//!
//! ## Layout (`ADR-0010` §1)
//!
//! - [`domain`] — zero-I/O value objects: [`Au`] / [`Px`], [`Point`] / [`Size`]
//!   / [`Rect`] / [`SurfaceSize`], [`Color`] / [`Opacity`], [`FontId`] /
//!   [`GlyphId`] / [`GlyphInstance`], [`ImageId`], [`BackendTier`],
//!   [`CommandIndex`] / [`CommandKind`], and the typed [`GraphicsError`].
//! - `application` — the ports themselves and the sanitizing builder (`F4a`).
//! - `infrastructure` — the tier cascade, the CPU rasterizer, font discovery
//!   and the PNG encoder (`F4a`/`F4b`).
//!
//! ## Determinism (`ADR-0016`)
//!
//! Every *computed* length is an [`Au`] — 1/64 px, integer arithmetic — so a
//! golden image matches byte for byte on Linux, macOS and Windows. [`Px`] is the
//! author-input type and crosses into [`Au`] through exactly one function,
//! [`Au::from_px`], which is therefore the single place `NaN` can be caught.
//!
//! ## Contract record
//!
//! This crate is the `RenderBackend` port under the `ADR-0011` Replaceable Port
//! Contract, and freezes at `F4` (`ADR-0011:121`).
//! `docs/architecture/render-backend-port-contract.md` records the state of all
//! seven items.

#![forbid(unsafe_code)]
// Every fallible function here documents its failures through the typed
// `GraphicsError` variant it returns; a prose `# Errors` section on each would
// restate the enum. Same call, same reason, as `core/dom/src/lib.rs:24`.
#![allow(clippy::missing_errors_doc)]

pub mod application;
pub mod domain;
pub mod infrastructure;

/// The observable version of this port's boundary aggregates.
///
/// `ADR-0011` item 3. Bumped on any change a backend or a producer could
/// notice; frozen at `F4`, after which a change also needs a migration note in
/// `PRD-005`.
pub const PORT_SCHEMA_VERSION: u32 = 1;

pub use application::conformance;
pub use application::{DisplayListBuilder, PxRect, RenderBackend};
pub use domain::{
    color::{Color, Opacity},
    command::DisplayCommand,
    command_index::CommandIndex,
    command_kind::CommandKind,
    display_list::DisplayList,
    error::{CommandRejection, FrameOperation, FrameState, GraphicsError},
    font::{FontId, GlyphId, GlyphInstance, GlyphRun},
    framebuffer::{BYTES_PER_PIXEL, Framebuffer},
    geometry::{Point, Rect, Size, SurfaceSize},
    image::ImageId,
    path::{Path, PathSegment, Stroke},
    tier::BackendTier,
    unit::{AU_PER_PX, Au, Px},
};
pub use infrastructure::cascade::{
    BackendPreference, BackendSelection, FORCE_TIER_VARIABLE, select_backend, select_backend_with,
};
#[cfg(feature = "software-backend")]
pub use infrastructure::software::SoftwareCpuBackend;
pub use infrastructure::{golden, png};

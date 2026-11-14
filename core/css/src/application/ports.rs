//! The replaceable ports of `PRD-007`: [`CascadeResolver`], [`LayoutEngine`],
//! [`TextMeasurer`] and — since issue #83 — [`NaturalSizes`].
//!
//! All four are object-safe and `Send + Sync`: every method speaks only this
//! crate's own types (plus the shared `graphics` units), so `&dyn` handles work
//! directly and `ADR-0011` item 2 is satisfied without a companion trait — the
//! same shape as `graphics`'s `RenderBackend`
//! (`core/graphics/src/application/ports.rs`).
//!
//! ## Whole-tree granularity is mandated
//!
//! `PRD-007:51` and `:78`: the unit of exchange is the **whole tree**. No
//! per-node callback crosses the seam — a naive per-node FFI seam would blow the
//! `<10μs` per-hook budget of `PRD-001:96`. A resolver takes a `&DomSnapshot`
//! and returns a `StyledTree`; a layout engine takes a `&StyledTree` and returns
//! a `LayoutBoxTree`. Nothing finer.
//!
//! [`TextMeasurer`] and [`NaturalSizes`] are the two lookups a layout engine
//! makes *while* it lays the tree out: Rust-to-Rust, injected into the engine
//! when it is built, never crossing a script boundary — so the whole-tree rule
//! does not apply to them (ADR-0028).

use crate::domain::dom_snapshot::{DomSnapshot, SnapshotId};
use crate::domain::error::CssError;
use crate::domain::layout_box_tree::LayoutBoxTree;
use crate::domain::replaced::ReplacedResource;
use crate::domain::styled_tree::StyledTree;
use crate::domain::stylesheet_set::StyleSheetSet;
use crate::domain::text::{ComputedText, TextMetrics, TextRun};
use crate::domain::viewport::ViewportConstraints;

/// Resolves the cascade: a DOM snapshot plus the stylesheets in, a computed
/// style per node out (`PRD-007:42-52`).
///
/// Pure and deterministic — identical inputs produce an identical
/// [`StyledTree`].
pub trait CascadeResolver: Send + Sync {
    /// Compute a style for every node of `dom` under `sheets`.
    fn resolve(&self, dom: &DomSnapshot, sheets: &StyleSheetSet) -> Result<StyledTree, CssError>;
}

/// Lays out a styled tree into boxes with resolved geometry
/// (`PRD-007:54-61`).
///
/// Pure and deterministic — the same [`StyledTree`] and the same
/// [`ViewportConstraints`] yield a byte-identical [`LayoutBoxTree`].
pub trait LayoutEngine: Send + Sync {
    /// Lay `styled` out within `constraints`.
    fn layout(
        &self,
        styled: &StyledTree,
        constraints: &ViewportConstraints,
    ) -> Result<LayoutBoxTree, CssError>;
}

/// Measures a run of text under a computed text style.
///
/// Consumed by the layout engine's inline formatting context from B4; the port
/// exists now so the contract is born whole.
pub trait TextMeasurer: Send + Sync {
    /// The extent of `run` set in `style`.
    fn measure(&self, run: &TextRun, style: &ComputedText) -> Result<TextMetrics, CssError>;
}

impl<T: TextMeasurer + ?Sized> TextMeasurer for &T {
    fn measure(&self, run: &TextRun, style: &ComputedText) -> Result<TextMetrics, CssError> {
        (**self).measure(run, style)
    }
}

impl<T: TextMeasurer + ?Sized> TextMeasurer for std::sync::Arc<T> {
    fn measure(&self, run: &TextRun, style: &ComputedText) -> Result<TextMetrics, CssError> {
        (**self).measure(run, style)
    }
}

/// What has become of the resource behind a replaced element (issue #83,
/// ADR-0028).
///
/// Asked by the layout engine for each replaced element it lays out, keyed by
/// the element's [`SnapshotId`]: the engine never sees where decoded images
/// are kept, and whoever keeps them never sees layout. Answering
/// [`ReplacedResource::Unavailable`] for a node it knows nothing about is the
/// correct default — that is what a page with no fetched resource is.
pub trait NaturalSizes: Send + Sync {
    /// The resource state of the replaced element `node`.
    fn resource_of(&self, node: SnapshotId) -> ReplacedResource;
}

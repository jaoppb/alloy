//! # `css` — the style-cascade and layout ports
//!
//! The **policy-heavy** stages of the render pipeline
//! `HtmlStream → DomTree → StyledTree → LayoutBoxTree → DisplayList`
//! (`ADR-0010:114-117`): CSS *parsing* stays native Rust (arriving in B1), but
//! *cascade resolution* and *layout* are exposed as replaceable ports so an
//! engine developer can substitute a custom specificity/inheritance resolver or
//! a custom layout algorithm — in Rust or as a `.rhai` adapter driven through
//! `RuntimeEngine` — without touching `core/dom`, `core/graphics`, or any
//! consumer (`PRD-007:12-16`).
//!
//! This crate names no engine type and has no script bridge: the `.rhai`
//! cascade adapter of `PRD-007` §3.4 is `core/runtime/rhai-bindings`' job, the
//! same way making a DOM node scriptable was at roadmap point I1. Its only
//! dependencies are `dom`, `thiserror`, and — for the shared fixed-point units
//! `Au` / `Px` / `Color` / `Rect` alone (`ADR-0016`) — `graphics`.
//!
//! ## Layout (`ADR-0010` §1)
//!
//! - [`domain`] — the four boundary aggregates ([`DomSnapshot`],
//!   [`StyleSheetSet`], [`StyledTree`], [`LayoutBoxTree`]) plus
//!   [`ViewportConstraints`]; the value objects ([`Length`], [`CssColor`],
//!   [`SnapshotId`], [`SourceSpan`]); the computed-value enums ([`Display`],
//!   [`CssStage`], [`Origin`], [`SnapshotNodeKind`]); and the typed [`CssError`].
//! - [`application`] — the three ports ([`CascadeResolver`], [`LayoutEngine`],
//!   [`TextMeasurer`]), the explicit [`snapshot`] mapping (`dom::DomTree →
//!   DomSnapshot`), and the [`conformance`] suite.
//! - [`infrastructure`] — the placeholder Rust adapters ([`UaCascade`],
//!   [`BlockLayout`], [`MonospaceMetrics`]) that B2/B4 replace, and the port
//!   mocks ([`MockCascadeResolver`] and friends).
//!
//! ## Contract record
//!
//! This crate is the `CascadeResolver` / `LayoutEngine` / `TextMeasurer` port
//! under the `ADR-0011` Replaceable Port Contract. The boundary aggregates and
//! [`PORT_SCHEMA_VERSION`] **freeze at integration point I3** (end of B4);
//! `docs/architecture/style-cascade-port-contract.md` records the state of all
//! seven items from that point on. A change after the freeze also needs a
//! migration note in `PRD-007`.

#![forbid(unsafe_code)]
// Every fallible function documents its failures through the typed `CssError`
// variant it returns; a prose `# Errors` section on each would restate the
// enum. Same call, same reason, as `core/dom/src/lib.rs:24`.
#![allow(clippy::missing_errors_doc)]

pub mod application;
pub mod domain;
pub mod infrastructure;

/// The observable version of this port's boundary aggregates.
///
/// `ADR-0011` item 3. Bumped on any change a resolver, a layout engine or a
/// producer could notice; **frozen at I3** (end of B4), after which a change
/// also needs a migration note in `PRD-007`.
pub const PORT_SCHEMA_VERSION: u32 = 1;

/// The CSS properties this crate can resolve to a computed value.
///
/// The single canonical registry the cascade honours — `ComputedStyle` carries
/// exactly these fields. `core/css/tests/manifest_runner.rs` asserts this list
/// and `core/css/tests/data/MANIFEST.md` agree in both directions; B1 grows the
/// list property by property as the parser lands.
pub const SUPPORTED_PROPERTIES: [&str; 6] = [
    "display",
    "color",
    "background-color",
    "margin",
    "padding",
    "font-size",
];

/// The selector forms this crate can match against a [`DomSnapshot`].
///
/// Empty in B0 — there is no selector engine yet (that is B1). The
/// manifest-consistency check still runs over it, so the mechanism is real from
/// the first slice.
pub const SUPPORTED_SELECTORS: [&str; 0] = [];

pub use application::conformance;
pub use application::ports::{CascadeResolver, LayoutEngine, TextMeasurer};
pub use application::snapshot::snapshot;
pub use domain::color::CssColor;
pub use domain::computed::{ComputedStyle, Display, LengthEdges};
pub use domain::dom_snapshot::{
    AttributeList, ChildIds, DomSnapshot, NodeRef, SnapshotId, SnapshotNodeKind,
};
pub use domain::error::{CssError, CssStage, SourceSpan};
pub use domain::layout_box_tree::{EdgeSizes, LayoutBox, LayoutBoxTree};
pub use domain::length::Length;
pub use domain::styled_tree::{StyledNode, StyledTree};
pub use domain::stylesheet_set::{DeclarationBlock, Origin, StyleRule, StyleSheetSet};
pub use domain::text::{ComputedText, TextMetrics, TextRun};
pub use domain::viewport::ViewportConstraints;
pub use infrastructure::cascade::UaCascade;
pub use infrastructure::layout::BlockLayout;
pub use infrastructure::mock::{MockCascadeResolver, MockLayoutEngine, MockTextMeasurer};
pub use infrastructure::text_metrics::MonospaceMetrics;

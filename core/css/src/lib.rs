//! # `css` — the style-cascade and layout ports
//!
//! The **policy-heavy** stages of the render pipeline
//! `HtmlStream → DomTree → StyledTree → LayoutBoxTree → DisplayList`
//! (`ADR-0010:114-117`): CSS *parsing* stays native Rust (`PRD-007:11-13`), but
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
//!   [`Identifier`], [`Specificity`], [`SnapshotId`], [`SourceSpan`]); the
//!   selector family ([`SelectorList`], [`ComplexSelector`],
//!   [`CompoundSelector`], [`Combinator`], [`PseudoClass`]); the declaration and
//!   media vocabulary ([`Declaration`], [`MediaQuery`]); the computed-value
//!   enums ([`Display`], [`CssStage`], [`Origin`], [`SnapshotNodeKind`]); and
//!   the typed [`CssError`].
//! - [`application`] — the three ports ([`CascadeResolver`], [`LayoutEngine`],
//!   [`TextMeasurer`]), the explicit [`snapshot`] mapping (`dom::DomTree →
//!   DomSnapshot`), selector [`matches`]ing, [`collect_style_sheets`], and the
//!   [`conformance`] suite.
//! - [`infrastructure`] — the hand-written CSS Syntax Level 3 parser
//!   ([`parse_stylesheet`], [`parse_inline_style`], [`tokenize`]), the
//!   placeholder Rust adapters ([`UaCascade`], [`BlockLayout`],
//!   [`MonospaceMetrics`]) that B2/B4 replace, and the port mocks
//!   ([`MockCascadeResolver`] and friends).
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
pub const PORT_SCHEMA_VERSION: u32 = 2;

/// The CSS properties this crate can parse and resolve to a computed value.
///
/// The single canonical registry: the parser **drops** a declaration whose
/// property is not on this list (with a [`ParseNote`]), and
/// `infrastructure/cascade/values.rs` applies exactly these.
/// `core/css/tests/manifest_runner.rs` asserts this list, the fourteen-row
/// `## Properties` table of `core/css/tests/data/MANIFEST.md`, and what
/// [`parse_stylesheet`] actually accepts all agree — in every direction.
pub const SUPPORTED_PROPERTIES: [&str; 14] = [
    "display",
    "color",
    "background-color",
    "margin",
    "margin-top",
    "margin-right",
    "margin-bottom",
    "margin-left",
    "padding",
    "padding-top",
    "padding-right",
    "padding-bottom",
    "padding-left",
    "font-size",
];

/// The selector and `@media` forms this crate parses and matches against a
/// [`DomSnapshot`] — the "Dentro" column of `relatório §2.8:342-345`, verbatim.
///
/// Written in the `E` / `F` element notation the CSS specifications use, so one
/// entry names one grammatical form rather than one example. Everything outside
/// this list is **refused** by the parser, never accepted and ignored;
/// `core/css/tests/manifest_runner.rs` proves both halves of that sentence.
pub const SUPPORTED_SELECTORS: [&str; 19] = [
    "E",
    "*",
    ".class",
    "#id",
    "[attr]",
    "[attr=value]",
    "E, F",
    "E F",
    "E > F",
    "E + F",
    "E ~ F",
    ":hover",
    ":active",
    ":focus",
    ":first-child",
    ":last-child",
    ":nth-child()",
    "@media (min-width)",
    "@media (max-width)",
];

pub use application::collect_sheets::collect_style_sheets;
pub use application::conformance;
pub use application::matching::{matches, strongest_match};
pub use application::ports::{CascadeResolver, LayoutEngine, TextMeasurer};
pub use application::snapshot::snapshot;
pub use domain::color::CssColor;
pub use domain::computed::{ComputedStyle, Display, LengthEdges};
pub use domain::declaration::{Declaration, DeclarationBlock, DeclarationValue, Importance};
pub use domain::dom_snapshot::{
    AttributeList, ChildIds, DomSnapshot, NodeRef, SnapshotId, SnapshotNodeKind,
};
pub use domain::error::{CssError, CssStage, SourceSpan};
pub use domain::identifier::Identifier;
pub use domain::layout_box_tree::{EdgeSizes, LayoutBox, LayoutBoxTree};
pub use domain::length::Length;
pub use domain::media::{MediaCondition, MediaFeature, MediaQuery};
pub use domain::parse_notes::{ParseNote, ParseNotes};
pub use domain::selector::{
    AttributeMatch, AttributeSelector, AttributeSelectors, Combinator, ComplexSelector,
    CompoundSelector, IdentifierList, NthFormula, PseudoClass, PseudoClasses, SelectorList,
    SelectorStep, TypeSelector,
};
pub use domain::specificity::Specificity;
pub use domain::styled_tree::{StyledNode, StyledTree};
pub use domain::stylesheet_set::{InlineStyles, Origin, StyleRule, StyleSheetSet};
pub use domain::text::{ComputedText, TextMetrics, TextRun};
pub use domain::viewport::ViewportConstraints;
pub use infrastructure::cascade::UaCascade;
pub use infrastructure::layout::BlockLayout;
pub use infrastructure::mock::{MockCascadeResolver, MockLayoutEngine, MockTextMeasurer};
pub use infrastructure::parser::{
    SpannedToken, Token, TokenStream, parse_inline_style, parse_stylesheet, tokenize,
};
pub use infrastructure::text_metrics::MonospaceMetrics;

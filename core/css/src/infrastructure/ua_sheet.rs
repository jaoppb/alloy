//! [`UaCascade`] — the [`CascadeResolver`] of B1: a hard-coded user-agent base
//! per tag, with the document's own parsed rules cascaded on top.
//!
//! B0 shipped this adapter ignoring its `StyleSheetSet` entirely. B1 is the
//! phase that makes the aggregate *do* something (`plano:430-431`): the UA
//! defaults below are the base, `infrastructure/cascade/author_rules.rs` applies
//! every rule that selects the node in `(origin, specificity, source order)`,
//! and the node's `style=` block lands last.
//!
//! Still stubbed, and B2's (`plano:435-443`): `!important` is parsed but does
//! not yet win, the user origin has no source, and the UA defaults live in Rust
//! rather than in a real `assets/ua.css` parsed by B1's own parser.
//! Inheritance covers exactly the two properties the placeholder
//! [`crate::BlockLayout`] and the painter read (`color`, `font-size`).

use crate::application::ports::CascadeResolver;
use crate::domain::computed::display::Display;
use crate::domain::computed::edges::LengthEdges;
use crate::domain::computed::style::ComputedStyle;
use crate::domain::dom_snapshot::{DomSnapshot, NodeRef, SnapshotNodeKind};
use crate::domain::error::CssError;
use crate::domain::length::Length;
use crate::domain::styled_tree::StyledTree;
use crate::domain::stylesheet_set::StyleSheetSet;
use crate::infrastructure::cascade::author_rules::apply_author_rules;

/// The user-agent cascade.
#[derive(Clone, Copy, Debug, Default)]
pub struct UaCascade;

impl UaCascade {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl CascadeResolver for UaCascade {
    fn resolve(&self, dom: &DomSnapshot, sheets: &StyleSheetSet) -> Result<StyledTree, CssError> {
        Ok(StyledTree::recompute_in_document_order(
            dom,
            |node_ref, parent| cascade_style(node_ref, parent, dom, sheets),
        ))
    }
}

/// One node's finished style: the UA base, then the author rules that select
/// it, then its `style=` block.
fn cascade_style(
    node_ref: NodeRef<'_>,
    parent: Option<&ComputedStyle>,
    dom: &DomSnapshot,
    sheets: &StyleSheetSet,
) -> ComputedStyle {
    let base = ua_style(node_ref, parent);
    apply_author_rules(base, node_ref, dom, sheets)
}

/// The computed style for one node: inherit from the parent, then apply the UA
/// rule for its tag.
fn ua_style(node_ref: NodeRef<'_>, parent: Option<&ComputedStyle>) -> ComputedStyle {
    let base = parent.map_or_else(ComputedStyle::initial, ComputedStyle::inheriting_from);
    node_ref.tag().map_or_else(
        || style_for_non_element(base, node_ref.kind()),
        |tag| style_for_tag(base, tag),
    )
}

const fn style_for_non_element(base: ComputedStyle, kind: SnapshotNodeKind) -> ComputedStyle {
    match kind {
        SnapshotNodeKind::Document | SnapshotNodeKind::Element => base.with_display(Display::Block),
        SnapshotNodeKind::Text => base.with_display(Display::Inline),
        SnapshotNodeKind::Comment => base.with_display(Display::None),
    }
}

fn style_for_tag(base: ComputedStyle, tag: &str) -> ComputedStyle {
    match tag {
        "head" | "style" | "script" | "title" | "meta" | "link" | "base" => {
            base.with_display(Display::None)
        }
        "span" | "a" | "em" | "strong" | "b" | "i" | "code" | "small" | "label" => {
            base.with_display(Display::Inline)
        }
        "h1" => heading(base, 2.00),
        "h2" => heading(base, 1.50),
        "h3" => heading(base, 1.17),
        "h4" => heading(base, 1.00),
        "h5" => heading(base, 0.83),
        "h6" => heading(base, 0.67),
        "p" => block_with_margin(base, LengthEdges::vertical(Length::Pixels(16.0))),
        "body" => block_with_margin(base, LengthEdges::uniform(Length::Pixels(8.0))),
        "blockquote" => block_with_margin(
            base,
            LengthEdges::new(
                Length::Pixels(16.0),
                Length::Pixels(40.0),
                Length::Pixels(16.0),
                Length::Pixels(40.0),
            ),
        ),
        _ => base.with_display(Display::Block),
    }
}

const fn block_with_margin(base: ComputedStyle, margin: LengthEdges) -> ComputedStyle {
    base.with_display(Display::Block).with_margin(margin)
}

/// A heading: block, `em`-relative font-size and a `0.67em` vertical margin —
/// the shape of the classic UA rules, kept minimal.
const fn heading(base: ComputedStyle, font_size_em: f32) -> ComputedStyle {
    base.with_display(Display::Block)
        .with_font_size(Length::Em(font_size_em))
        .with_margin(LengthEdges::vertical(Length::Em(0.67)))
}

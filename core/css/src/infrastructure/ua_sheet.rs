//! [`UaCascade`] — a placeholder [`CascadeResolver`] whose rules are hard-coded
//! in Rust.
//!
//! Origins and `!important` are stubbed to user-agent-only; the author
//! `StyleSheetSet` is ignored. Inheritance covers exactly the two properties
//! the placeholder [`crate::BlockLayout`] and the painter read (`color`,
//! `font-size`). B2 replaces this with a real three-origin cascade over parsed
//! rules.

use crate::application::ports::CascadeResolver;
use crate::domain::computed::display::Display;
use crate::domain::computed::edges::LengthEdges;
use crate::domain::computed::style::ComputedStyle;
use crate::domain::dom_snapshot::{DomSnapshot, NodeRef, SnapshotNodeKind};
use crate::domain::error::CssError;
use crate::domain::length::Length;
use crate::domain::styled_tree::StyledTree;
use crate::domain::stylesheet_set::StyleSheetSet;

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
    fn resolve(&self, dom: &DomSnapshot, _sheets: &StyleSheetSet) -> Result<StyledTree, CssError> {
        Ok(StyledTree::recompute_in_document_order(dom, ua_style))
    }
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

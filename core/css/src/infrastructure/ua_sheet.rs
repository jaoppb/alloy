//! [`UaCascade`] — the [`CascadeResolver`] of B1/B2: the embedded user-agent
//! sheet, cascaded through the same machinery as the document's own rules.
//!
//! B1 shipped the UA defaults as a hard-coded Rust function per tag and said
//! so in this file's doc comment: "the UA defaults live in Rust rather than
//! in a real `assets/ua.css` parsed by B1's own parser." B2 is that move
//! (`plano:435-443`): [`UaCascade::new`] parses `assets/ua.css`
//! (`crate::infrastructure::ua_sheet::UA_SHEET_SOURCE`) once into an
//! `Origin::UserAgent`-tagged [`StyleSheetSet`], stored on the struct, and
//! [`UaCascade::resolve`] merges it with whatever `StyleSheetSet` it is
//! handed **before** a single [`apply_author_rules`] pass decides every
//! node's style. One merged pass — rather than a UA pass followed by an
//! author pass — is also what makes `!important` order correctly *across*
//! origins (CSS Cascade L4 §4.2): a UA `!important` declaration and an author
//! declaration now compete in the very same sort, never in two.
//!
//! Which properties inherit is [`ComputedStyle::inheriting_from`]'s; the
//! CSS-wide keywords `initial` / `inherit` are
//! `infrastructure/cascade/values.rs`'s (B2, same phase). What a node hands
//! its children beyond its style — its custom properties and its computed
//! font size — travels in an [`InheritedContexts`] side table built alongside
//! the styled tree, because neither belongs in the style aggregate.

use std::collections::BTreeMap;

use crate::application::ports::CascadeResolver;
use crate::domain::computed::display::Display;
use crate::domain::computed::style::ComputedStyle;
use crate::domain::dom_snapshot::{DomSnapshot, NodeRef, SnapshotId, SnapshotNodeKind};
use crate::domain::error::CssError;
use crate::domain::styled_tree::StyledTree;
use crate::domain::stylesheet_set::{Origin, StyleSheetSet};
use crate::infrastructure::cascade::author_rules::{InheritedContext, apply_author_rules};
use crate::infrastructure::parser::parse_stylesheet;

/// The embedded user-agent stylesheet's CSS text (`core/css/assets/ua.css`) —
/// real CSS, not Rust, per `plano:435-443`.
const UA_SHEET_SOURCE: &str = include_str!("../../assets/ua.css");

/// The user-agent cascade: the parsed UA sheet, merged ahead of whatever
/// [`StyleSheetSet`] [`UaCascade::resolve`] is handed.
#[derive(Clone, Debug)]
pub struct UaCascade {
    ua_rules: StyleSheetSet,
}

impl UaCascade {
    /// Parses `assets/ua.css` once.
    ///
    /// The embedded sheet is a startup-time invariant this crate owns end to
    /// end: it ships inside the compiled binary (`include_str!`), so a parse
    /// failure here means the workspace itself cannot build a working
    /// `core/css`, not that a caller supplied bad input. That is exactly the
    /// "genuinely impossible state" `.expect()` carve-out `CLAUDE.md`
    /// documents for `core/dom` and `core/engine` — there is no caller-facing
    /// `Result` to push a failure like this into.
    #[must_use]
    pub fn new() -> Self {
        let ua_rules = parse_ua_sheet();
        Self { ua_rules }
    }

    /// The UA sheet and `sheets`, in one list — so a single
    /// [`apply_author_rules`] pass sorts every declaration by `(precedence,
    /// specificity, source order)` regardless of which one it came from.
    fn combined_with(&self, sheets: &StyleSheetSet) -> StyleSheetSet {
        let mut combined = self.ua_rules.clone();
        combined.absorb(sheets.clone());
        combined
    }
}

// `assets/ua.css` is `include_str!`-embedded, so a parse failure here is a
// defect in this crate's own shipped asset, never caller input — the
// "genuinely impossible state" `.expect()` carve-out `CLAUDE.md` documents
// for `core/dom` / `core/engine`.
#[allow(clippy::expect_used)]
fn parse_ua_sheet() -> StyleSheetSet {
    parse_stylesheet(UA_SHEET_SOURCE, Origin::UserAgent)
        .expect("core/css/assets/ua.css ships with this crate and must always parse")
}

impl Default for UaCascade {
    fn default() -> Self {
        Self::new()
    }
}

impl CascadeResolver for UaCascade {
    fn resolve(&self, dom: &DomSnapshot, sheets: &StyleSheetSet) -> Result<StyledTree, CssError> {
        let combined = self.combined_with(sheets);
        let mut contexts = InheritedContexts::new();
        Ok(StyledTree::recompute_in_document_order(
            dom,
            |node_ref, parent| {
                let inherited = contexts.of_parent(node_ref);
                let (style, context) = cascade_style(node_ref, parent, inherited, dom, &combined);
                contexts.record(node_ref, context);
                style
            },
        ))
    }
}

/// Every cascaded node's [`InheritedContext`], keyed by node. Document order
/// visits a parent before its children, so a child's lookup always finds its
/// parent's entry; a node with no parent (or, defensively, one whose parent
/// was never visited) inherits the root context.
struct InheritedContexts {
    root: InheritedContext,
    by_node: BTreeMap<SnapshotId, InheritedContext>,
}

impl InheritedContexts {
    fn new() -> Self {
        Self {
            root: InheritedContext::root(),
            by_node: BTreeMap::new(),
        }
    }

    fn of_parent(&self, node_ref: NodeRef<'_>) -> &InheritedContext {
        node_ref
            .parent()
            .and_then(|parent| self.by_node.get(&parent))
            .unwrap_or(&self.root)
    }

    fn record(&mut self, node_ref: NodeRef<'_>, context: InheritedContext) {
        self.by_node.insert(node_ref.id(), context);
    }
}

/// One node's finished style: the inherited/initial base, then every
/// matching rule — UA and author alike — in cascade order, then the node's
/// `style=` block; with the context its own children inherit.
fn cascade_style(
    node_ref: NodeRef<'_>,
    parent: Option<&ComputedStyle>,
    inherited: &InheritedContext,
    dom: &DomSnapshot,
    sheets: &StyleSheetSet,
) -> (ComputedStyle, InheritedContext) {
    let base = base_style(node_ref, parent);
    apply_author_rules(base, parent, inherited, node_ref, dom, sheets).into_parts()
}

/// The style before any rule applies: inherit from the parent (or take the
/// CSS `initial` value at the root), then the one rule no selector can ever
/// express — the fixed `display` a document, a text run or a comment gets.
fn base_style(node_ref: NodeRef<'_>, parent: Option<&ComputedStyle>) -> ComputedStyle {
    let base = parent.map_or_else(ComputedStyle::initial, ComputedStyle::inheriting_from);
    if node_ref.tag().is_some() {
        return base;
    }
    style_for_non_element(base, node_ref.kind())
}

/// A non-element node: no selector ever chooses one (`application/matching.rs`
/// doc-comment), so its `display` stays a fixed Rust rule rather than a CSS
/// one. An element's default is already [`Display::Block`] from
/// [`ComputedStyle::initial`], which is why this arm is the only one that
/// still needs Rust after `assets/ua.css` took over every per-tag exception.
fn style_for_non_element(base: ComputedStyle, kind: SnapshotNodeKind) -> ComputedStyle {
    match kind {
        SnapshotNodeKind::Document | SnapshotNodeKind::Element => base.with_display(Display::Block),
        SnapshotNodeKind::Text => base.with_display(Display::Inline),
        SnapshotNodeKind::Comment => base.with_display(Display::None),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use crate::application::snapshot::snapshot;

    /// Every element WHATWG HTML §15.3 lays out as a block, plus the table
    /// elements this engine stacks as blocks until it has table layout
    /// (MANIFEST cut). `hr` is listed for its `display` only (#82 owns the rest).
    const BLOCK_ELEMENTS: [&str; 49] = [
        "html",
        "body",
        "address",
        "blockquote",
        "center",
        "div",
        "figure",
        "figcaption",
        "footer",
        "form",
        "header",
        "hr",
        "legend",
        "listing",
        "main",
        "p",
        "plaintext",
        "pre",
        "search",
        "xmp",
        "article",
        "aside",
        "h1",
        "h2",
        "h3",
        "h4",
        "h5",
        "h6",
        "hgroup",
        "nav",
        "section",
        "dir",
        "dd",
        "dl",
        "dt",
        "menu",
        "ol",
        "ul",
        "details",
        "summary",
        "fieldset",
        "table",
        "caption",
        "thead",
        "tbody",
        "tfoot",
        "tr",
        "td",
        "th",
    ];

    /// Elements §15.3.1 never renders.
    const HIDDEN_ELEMENTS: [&str; 9] = [
        "area", "datalist", "noembed", "noframes", "param", "rp", "template", "col", "colgroup",
    ];

    /// The `display` the UA sheet alone gives the first `tag` element of
    /// `markup`, cascaded over an `inline` base — so a missing UA rule shows
    /// up as `inline` instead of hiding behind today's `block` initial value
    /// (#81: the initial value becomes `inline`).
    fn ua_display(markup: &str, tag: &str) -> Display {
        let tree = dom::parse(markup).expect("the fixture parses").into_tree();
        let dom = snapshot(&tree, tree.document());
        let node = dom
            .nodes_in_document_order()
            .filter_map(|id| dom.node(id))
            .find(|node| node.tag_str() == Some(tag))
            .unwrap_or_else(|| panic!("the fixture has a <{tag}>"));
        let base = ComputedStyle::initial().with_display(Display::Inline);
        let cascaded = apply_author_rules(
            base,
            None,
            &InheritedContext::root(),
            node,
            &dom,
            &UaCascade::new().ua_rules,
        );
        cascaded.into_parts().0.display()
    }

    fn markup_with(tag: &str) -> String {
        format!("<!doctype html><html><body><{tag}></{tag}></body></html>")
    }

    #[test]
    fn the_ua_sheet_itself_makes_every_flow_element_a_block() {
        for tag in BLOCK_ELEMENTS {
            let markup = table_aware_markup(tag);
            assert_eq!(ua_display(&markup, tag), Display::Block, "<{tag}>");
        }
    }

    #[test]
    fn the_ua_sheet_hides_the_elements_html_never_renders() {
        for tag in HIDDEN_ELEMENTS {
            let markup = table_aware_markup(tag);
            assert_eq!(ua_display(&markup, tag), Display::None, "<{tag}>");
        }
    }

    #[test]
    fn a_dialog_is_hidden_until_it_is_open() {
        let closed = "<!doctype html><body><dialog></dialog></body>";
        let open = "<!doctype html><body><dialog open></dialog></body>";
        assert_eq!(ua_display(closed, "dialog"), Display::None);
        assert_eq!(ua_display(open, "dialog"), Display::Block);
    }

    #[test]
    fn the_hidden_attribute_removes_the_box_of_any_element() {
        let markup = "<!doctype html><body><span hidden>x</span></body>";
        assert_eq!(ua_display(markup, "span"), Display::None);
    }

    /// Table-internal elements only survive tree construction inside a table.
    fn table_aware_markup(tag: &str) -> String {
        let table = "<table><caption></caption><colgroup><col></colgroup><thead><tr><th></th></tr></thead><tbody><tr><td></td></tr></tbody><tfoot></tfoot></table>";
        match tag {
            "table" | "caption" | "thead" | "tbody" | "tfoot" | "tr" | "td" | "th" | "col"
            | "colgroup" => format!("<!doctype html><html><body>{table}</body></html>"),
            _ => markup_with(tag),
        }
    }
}

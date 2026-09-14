//! Port mocks that prove the cascade/layout/measure seams swap
//! (`PRD-007:94-95`).
//!
//! Each forces its output to a distinctive sentinel a test can assert on, and
//! none is gated by a feature — they compile under `--no-default-features` too,
//! so the `no-script` build has a swappable adapter to point at.

use graphics::{Au, Point, Rect, Size};

use crate::application::ports::{CascadeResolver, LayoutEngine, TextMeasurer};
use crate::domain::color::CssColor;
use crate::domain::computed::style::ComputedStyle;
use crate::domain::dom_snapshot::{ChildIds, DomSnapshot};
use crate::domain::error::CssError;
use crate::domain::layout_box_tree::{EdgeSizes, LayoutBox, LayoutBoxTree, LayoutBoxTreeBuilder};
use crate::domain::styled_tree::StyledTree;
use crate::domain::stylesheet_set::StyleSheetSet;
use crate::domain::text::{ComputedText, TextMetrics, TextRun};
use crate::domain::viewport::ViewportConstraints;

/// A [`CascadeResolver`] that forces every node's computed `color` to one
/// sentinel value — visibly different from anything [`crate::UaCascade`]
/// produces, so a swap is observable.
#[derive(Clone, Copy, Debug, Default)]
pub struct MockCascadeResolver;

impl MockCascadeResolver {
    /// The colour this mock stamps on every node.
    pub const SENTINEL_COLOR: CssColor = CssColor::rgba(0x0B, 0xAD, 0xC0, 0xFF);

    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl CascadeResolver for MockCascadeResolver {
    fn resolve(&self, dom: &DomSnapshot, _sheets: &StyleSheetSet) -> Result<StyledTree, CssError> {
        Ok(StyledTree::recompute_in_document_order(
            dom,
            |_node, _parent| ComputedStyle::initial().with_color(Self::SENTINEL_COLOR),
        ))
    }
}

/// A [`LayoutEngine`] that gives every node a `1×1` [`Au`] box at the origin.
#[derive(Clone, Copy, Debug, Default)]
pub struct MockLayoutEngine;

impl MockLayoutEngine {
    /// The side length of every mock box.
    pub const UNIT: Au = Au::from_raw(1);

    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl LayoutEngine for MockLayoutEngine {
    fn layout(
        &self,
        styled: &StyledTree,
        _constraints: &ViewportConstraints,
    ) -> Result<LayoutBoxTree, CssError> {
        let mut builder = LayoutBoxTreeBuilder::new();
        let mut root = None;
        let content = Rect::new(
            Point::ORIGIN,
            Size::new(Self::UNIT, Self::UNIT).unwrap_or(Size::EMPTY),
        );
        for styled_node in styled.nodes_in_document_order() {
            if root.is_none() {
                root = Some(styled_node.node());
            }
            builder.push(LayoutBox::new(
                styled_node.node(),
                content,
                EdgeSizes::ZERO,
                EdgeSizes::ZERO,
                ChildIds::from_ids(styled_node.children().iter()),
            ));
        }
        Ok(builder.finish(root))
    }
}

/// A [`TextMeasurer`] that returns one fixed metric regardless of input.
#[derive(Clone, Copy, Debug, Default)]
pub struct MockTextMeasurer;

impl MockTextMeasurer {
    /// The metric this mock always returns.
    pub const METRICS: TextMetrics = TextMetrics::new(Au::from_raw(7), Au::from_raw(13));

    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl TextMeasurer for MockTextMeasurer {
    fn measure(&self, _run: &TextRun, _style: &ComputedText) -> Result<TextMetrics, CssError> {
        Ok(Self::METRICS)
    }
}

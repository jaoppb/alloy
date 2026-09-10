//! The three values every formatting context of v0.5 B4 speaks:
//! [`LayoutContext`] (what it may read), [`BlockInput`] (what its caller
//! decided for it) and [`BlockResult`] / [`ContentFlow`] (what it answers).
//!
//! Grouping the inputs is not decoration: `layout_box` needs a containing
//! width, an inherited font size, a recursion depth and — for a flex item — a
//! forced main size, and four positional arguments of the same type are four
//! chances to swap two of them.

use graphics::Au;

use crate::application::ports::TextMeasurer;
use crate::domain::error::{CssError, CssStage};
use crate::domain::layout_box_tree::BoxEdges;
use crate::domain::styled_tree::{StyledNode, StyledTree};
use crate::domain::text::{ComputedText, TextMetrics, TextRun};
use crate::infrastructure::layout::fragment::Fragments;
use crate::infrastructure::layout::margin_collapse::{CollapsedMargin, MarginFlow};

/// How deeply boxes may nest before the input is refused.
///
/// A document nested past this is not a page, it is the hostile input the fuzz
/// budget of the v0.5 report §2.11 exists for — the same judgement, and the
/// same answer, as `MAX_NESTING_DEPTH` in the stylesheet parser.
pub const MAX_LAYOUT_DEPTH: usize = 256;

/// Everything a formatting context may read: the tree it is laying out and the
/// text measurer behind the port.
pub struct LayoutContext<'tree> {
    styled: &'tree StyledTree,
    measurer: &'tree dyn TextMeasurer,
}

impl<'tree> LayoutContext<'tree> {
    pub const fn new(styled: &'tree StyledTree, measurer: &'tree dyn TextMeasurer) -> Self {
        Self { styled, measurer }
    }

    /// The styled node behind `id`, or the typed error for a dangling id.
    pub fn node(
        &self,
        id: crate::domain::dom_snapshot::SnapshotId,
    ) -> Result<&'tree StyledNode, CssError> {
        self.styled
            .node(id)
            .ok_or_else(|| CssError::missing_computed_style(CssStage::Layout, id))
    }

    /// The extent of `text` set at `font_size`, through the port — never
    /// through a font type.
    pub fn measure(&self, text: &str, font_size: Au) -> Result<TextMetrics, CssError> {
        let run = TextRun::new(text);
        let style = ComputedText::new(font_size);
        self.measurer.measure(&run, &style)
    }
}

/// What a caller decided before asking for a box.
#[derive(Clone, Copy, Debug)]
pub struct BlockInput {
    containing_width: Au,
    parent_font_size: Au,
    depth: usize,
    forced_content_height: Option<Au>,
    forced_content_width: Option<Au>,
}

impl BlockInput {
    pub const fn new(containing_width: Au, parent_font_size: Au) -> Self {
        Self {
            containing_width,
            parent_font_size,
            depth: 0,
            forced_content_height: None,
            forced_content_width: None,
        }
    }

    /// The same input one level deeper, for a child of the box being laid out.
    pub const fn nested(self, containing_width: Au, parent_font_size: Au) -> Self {
        Self {
            containing_width,
            parent_font_size,
            depth: self.depth.saturating_add(1),
            forced_content_height: None,
            forced_content_width: None,
        }
    }

    /// The same input with the content height pinned — what a flex container
    /// does to a `column`-direction item once it has resolved the item's main
    /// size, or to a `row`-direction item stretched on the cross axis.
    pub const fn with_forced_content_height(self, height: Au) -> Self {
        Self {
            forced_content_height: Some(height),
            ..self
        }
    }

    /// The same input with the content width pinned — what a `row`-direction
    /// flex container does to an item once flex-basis/grow/shrink have
    /// resolved its main size, overriding whatever `width` the item declared
    /// (CSS Flexbox L1 §7.2: flex-basis and its resolution take priority over
    /// `width` on the main axis).
    pub const fn with_forced_content_width(self, width: Au) -> Self {
        Self {
            forced_content_width: Some(width),
            ..self
        }
    }

    pub const fn containing_width(self) -> Au {
        self.containing_width
    }

    pub const fn parent_font_size(self) -> Au {
        self.parent_font_size
    }

    pub const fn depth(self) -> usize {
        self.depth
    }

    pub const fn forced_content_height(self) -> Option<Au> {
        self.forced_content_height
    }

    pub const fn forced_content_width(self) -> Option<Au> {
        self.forced_content_width
    }

    /// The typed refusal for a document nested past [`MAX_LAYOUT_DEPTH`].
    pub fn too_deep() -> CssError {
        CssError::unsupported(
            CssStage::Layout,
            format!("box nesting deeper than {MAX_LAYOUT_DEPTH} is refused"),
        )
    }
}

/// One laid-out block-level box: how tall it is, what margins escape it, and
/// its fragments **relative to its own border-box origin**.
pub struct BlockResult {
    size: BorderBoxSize,
    edges: BoxEdges,
    top_margin: CollapsedMargin,
    bottom_margin: CollapsedMargin,
    flow: MarginFlow,
    fragments: Fragments,
}

/// A border-box extent — the pair every caller of `layout_box` reads back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BorderBoxSize {
    width: Au,
    height: Au,
}

impl BorderBoxSize {
    pub const fn new(width: Au, height: Au) -> Self {
        Self { width, height }
    }

    pub const fn width(self) -> Au {
        self.width
    }

    pub const fn height(self) -> Au {
        self.height
    }
}

impl BlockResult {
    pub const fn new(
        size: BorderBoxSize,
        edges: BoxEdges,
        margins: (CollapsedMargin, CollapsedMargin),
        flow: MarginFlow,
        fragments: Fragments,
    ) -> Self {
        let (top_margin, bottom_margin) = margins;
        Self {
            size,
            edges,
            top_margin,
            bottom_margin,
            flow,
            fragments,
        }
    }

    /// The border-box height.
    pub const fn height(&self) -> Au {
        self.size.height()
    }

    pub const fn edges(&self) -> BoxEdges {
        self.edges
    }

    /// The border-box width plus this box's horizontal margins.
    pub const fn outer_width(&self) -> Au {
        let margin = self.edges.margin();
        self.size.width().saturating_add(margin.horizontal())
    }

    /// The border-box height plus this box's vertical margins.
    pub const fn outer_height(&self) -> Au {
        let margin = self.edges.margin();
        self.size.height().saturating_add(margin.vertical())
    }

    pub const fn top_margin(&self) -> CollapsedMargin {
        self.top_margin
    }

    pub const fn bottom_margin(&self) -> CollapsedMargin {
        self.bottom_margin
    }

    pub const fn flow(&self) -> MarginFlow {
        self.flow
    }

    pub fn into_fragments(self) -> Fragments {
        self.fragments
    }
}

/// What a formatting context makes of a box's children: their total height,
/// their fragments **relative to the content-box origin**, and the margins that
/// escape at either end.
pub struct ContentFlow {
    height: Au,
    fragments: Fragments,
    leading_margin: CollapsedMargin,
    trailing_margin: CollapsedMargin,
    flow: MarginFlow,
}

impl ContentFlow {
    pub const fn new(height: Au, fragments: Fragments) -> Self {
        Self {
            height,
            fragments,
            leading_margin: CollapsedMargin::ZERO,
            trailing_margin: CollapsedMargin::ZERO,
            flow: MarginFlow::Separated,
        }
    }

    /// An empty flow — a box with no in-flow children, whose own margins may
    /// therefore adjoin each other.
    pub const fn empty() -> Self {
        Self {
            height: Au::ZERO,
            fragments: Fragments::new(),
            leading_margin: CollapsedMargin::ZERO,
            trailing_margin: CollapsedMargin::ZERO,
            flow: MarginFlow::CollapsesThrough,
        }
    }

    pub fn with_margins(
        self,
        leading_margin: CollapsedMargin,
        trailing_margin: CollapsedMargin,
    ) -> Self {
        Self {
            leading_margin,
            trailing_margin,
            ..self
        }
    }

    pub fn with_flow(self, flow: MarginFlow) -> Self {
        Self { flow, ..self }
    }

    pub const fn height(&self) -> Au {
        self.height
    }

    pub const fn leading_margin(&self) -> CollapsedMargin {
        self.leading_margin
    }

    pub const fn trailing_margin(&self) -> CollapsedMargin {
        self.trailing_margin
    }

    pub const fn flow(&self) -> MarginFlow {
        self.flow
    }

    pub fn into_fragments(self) -> Fragments {
        self.fragments
    }
}

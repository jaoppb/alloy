//! The three values every formatting context of v0.5 B4 speaks:
//! [`LayoutContext`] (what it may read), [`BlockInput`] (what its caller
//! decided for it) and [`BlockResult`] / [`ContentFlow`] (what it answers).
//!
//! Grouping the inputs is not decoration: `layout_box` needs a containing
//! block, an inherited font size, a recursion depth and — for a flex item — a
//! forced main size, and four positional arguments of the same type are four
//! chances to swap two of them.

use core::cell::RefCell;
use std::collections::BTreeMap;

use graphics::Au;

use crate::application::ports::TextMeasurer;
use crate::domain::dom_snapshot::SnapshotId;
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
/// text measurer behind the port — plus the one memo layout keeps, the
/// shrink-to-fit widths already measured (see `block::layout_inline_block`).
pub struct LayoutContext<'tree, M> {
    styled: &'tree StyledTree,
    measurer: &'tree M,
    fits: RefCell<BTreeMap<SnapshotId, FittedWidth>>,
}

/// A shrink-to-fit measurement: the content width a box was offered and the
/// width its contents turned out to need.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FittedWidth {
    available: Au,
    fitted: Au,
}

impl FittedWidth {
    pub const fn new(available: Au, fitted: Au) -> Self {
        Self { available, fitted }
    }

    /// The fitted width still holds for an offer of `available`: anything from
    /// the fitted width up to the width measured against. Greedy line breaking
    /// at any limit in that range produces the very same lines, because none of
    /// them was wider than the fitted width.
    fn holds_for(self, available: Au) -> bool {
        self.fitted <= available && available <= self.available
    }
}

impl<'tree, M: TextMeasurer> LayoutContext<'tree, M> {
    pub const fn new(styled: &'tree StyledTree, measurer: &'tree M) -> Self {
        Self {
            styled,
            measurer,
            fits: RefCell::new(BTreeMap::new()),
        }
    }

    /// The shrink-to-fit width remembered for `node`, when it still holds for
    /// an offer of `available`.
    pub fn remembered_fit(&self, node: SnapshotId, available: Au) -> Option<Au> {
        let fits = self.fits.try_borrow().ok()?;
        let remembered = fits.get(&node)?;
        remembered.holds_for(available).then_some(remembered.fitted)
    }

    /// Remembers the shrink-to-fit width just measured for `node`. A memo that
    /// is momentarily borrowed is simply not updated — it is an optimisation,
    /// never a source of truth.
    pub fn remember_fit(&self, node: SnapshotId, fit: FittedWidth) {
        let Ok(mut fits) = self.fits.try_borrow_mut() else {
            return;
        };
        fits.insert(node, fit);
    }

    /// The styled node behind `id`, or the typed error for a dangling id.
    pub fn node(&self, id: SnapshotId) -> Result<&'tree StyledNode, CssError> {
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

/// The containing block a box is laid out against (CSS 2.1 §10.1): its width
/// is always definite in this engine; its height only when the parent's own
/// height does not depend on its content (a declared or forced height, or the
/// viewport for the root).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContainingBlock {
    width: Au,
    height: Option<Au>,
}

impl ContainingBlock {
    pub const fn new(width: Au, height: Option<Au>) -> Self {
        Self { width, height }
    }

    pub const fn width(self) -> Au {
        self.width
    }

    pub const fn height(self) -> Option<Au> {
        self.height
    }
}

/// What a caller decided before asking for a box.
#[derive(Clone, Copy, Debug)]
pub struct BlockInput {
    containing: ContainingBlock,
    parent_font_size: Au,
    depth: usize,
    forced_content_height: Option<Au>,
    forced_content_width: Option<Au>,
}

impl BlockInput {
    pub const fn new(containing: ContainingBlock, parent_font_size: Au) -> Self {
        Self {
            containing,
            parent_font_size,
            depth: 0,
            forced_content_height: None,
            forced_content_width: None,
        }
    }

    /// The same input one level deeper, for a child of the box being laid out.
    pub const fn nested(self, containing: ContainingBlock, parent_font_size: Au) -> Self {
        Self {
            containing,
            parent_font_size,
            depth: self.depth.saturating_add(1),
            forced_content_height: None,
            forced_content_width: None,
        }
    }

    /// The same input with a different inherited font size — what an inline
    /// formatting context does as it descends through inline boxes that set
    /// their own `font-size` before reaching an atomic inline (CSS 2.1 §9.2.4).
    pub const fn with_parent_font_size(self, parent_font_size: Au) -> Self {
        Self {
            parent_font_size,
            ..self
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
        self.containing.width
    }

    /// The containing block's height, or `None` when it is not definite — in
    /// which case a percentage measured against it behaves as `auto`
    /// (CSS 2.1 §10.5, §9.3.2).
    pub const fn containing_height(self) -> Option<Au> {
        self.containing.height
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

    /// The content width this box needs to hold what was laid out inside it,
    /// never more than the content width it was given — the "preferred width"
    /// half of CSS 2.1 §10.3.9's shrink-to-fit, measured from a layout at the
    /// available width instead of from a separate intrinsic-sizing pass.
    pub fn fitted_content_width(&self) -> Au {
        let border = self.edges.border();
        let padding = self.edges.padding();
        let inner = border.horizontal().saturating_add(padding.horizontal());
        let given = self.size.width().saturating_sub(inner);
        self.fragments.descendant_span().smaller(given)
    }

    /// Returns a new result with every fragment translated by `(dx, dy)`.
    ///
    /// Used by [`crate::infrastructure::layout::block`] to apply the visual
    /// offset of `position: relative` after normal-flow placement. The box
    /// still occupies its normal-flow space (size and margins are unchanged);
    /// only the paint position shifts (CSS Positioned Layout L3 §4.3).
    pub fn with_relative_offset(self, dx: Au, dy: Au) -> Self {
        Self {
            fragments: self.fragments.translated(dx, dy),
            ..self
        }
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

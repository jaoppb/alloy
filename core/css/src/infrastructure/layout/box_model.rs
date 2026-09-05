//! [`BoxMetrics`] — one node's three edges and its two axes, resolved from
//! author lengths to computed [`Au`] (v0.5 B4, step 1 of the phase).
//!
//! This is the only place `box-sizing` is read: a `border-box` declaration is
//! turned into the **content** size the rest of the engine works in, so no
//! formatting context has to remember which convention the author used
//! (CSS Box Sizing L3 §5).

use graphics::Au;

use crate::domain::computed::edges::LengthEdges;
use crate::domain::computed::sizing::{BoxSizing, Sizing};
use crate::domain::computed::style::ComputedStyle;
use crate::domain::error::{CssError, CssStage};
use crate::domain::layout_box_tree::{BoxEdges, EdgeSizes};

/// The CSS `initial` computed `font-size`, `16px`, as an [`Au`].
pub const DEFAULT_FONT_SIZE: Au = match Au::from_whole_px(16) {
    Some(size) => size,
    None => Au::ZERO,
};

/// The resolved box of one node, before its position is known.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoxMetrics {
    edges: BoxEdges,
    width: Option<Au>,
    height: Option<Au>,
}

impl BoxMetrics {
    pub const fn edges(self) -> BoxEdges {
        self.edges
    }

    /// The declared **content** width, or `None` for `width: auto`.
    pub const fn width(self) -> Option<Au> {
        self.width
    }

    /// The declared **content** height, or `None` for `height: auto` and for a
    /// percentage against an indefinite containing block.
    pub const fn height(self) -> Option<Au> {
        self.height
    }

    /// Border plus padding on the horizontal axis — the difference between a
    /// content box and a border box.
    pub const fn inner_horizontal(self) -> Au {
        let border = self.edges.border();
        let padding = self.edges.padding();
        border.horizontal().saturating_add(padding.horizontal())
    }

    /// Border plus padding on the vertical axis.
    pub const fn inner_vertical(self) -> Au {
        let border = self.edges.border();
        let padding = self.edges.padding();
        border.vertical().saturating_add(padding.vertical())
    }

    /// The content width this box uses inside a containing block `available`
    /// wide: the declared width when there is one, otherwise everything left
    /// after the three edges.
    pub const fn content_width_within(self, available: Au) -> Au {
        let Some(declared) = self.width else {
            return available
                .saturating_sub(self.edges.horizontal())
                .larger(Au::ZERO);
        };
        declared
    }

    /// The border-box width this box occupies, given its content width.
    pub const fn border_box_width(self, content_width: Au) -> Au {
        content_width.saturating_add(self.inner_horizontal())
    }

    /// The border-box height this box occupies, given its content height.
    pub const fn border_box_height(self, content_height: Au) -> Au {
        content_height.saturating_add(self.inner_vertical())
    }
}

/// The computed `font-size` of `style` in [`Au`], falling back to the initial
/// `16px` when the author wrote a magnitude with no correct reading.
pub fn font_size_of(style: &ComputedStyle, parent_font_size: Au) -> Au {
    style
        .font_size_au(parent_font_size)
        .unwrap_or(DEFAULT_FONT_SIZE)
}

/// Resolves every box property of `style` against a containing block
/// `container_width` wide.
pub fn resolve(
    style: &ComputedStyle,
    font_size: Au,
    container_width: Au,
) -> Result<BoxMetrics, CssError> {
    let margin = edges_of(style.margin(), font_size, container_width, "margin")?;
    let border = edges_of(style.border(), font_size, container_width, "border-width")?;
    let padding = edges_of(style.padding(), font_size, container_width, "padding")?;
    let edges = BoxEdges::new(margin, border, padding);
    let declared_width = style.width().resolve(font_size, container_width);
    let declared_height = definite_height(style, font_size);
    Ok(BoxMetrics {
        edges,
        width: declared_width.map(|width| content_from(style, width, edges, Axis::Horizontal)),
        height: declared_height.map(|height| content_from(style, height, edges, Axis::Vertical)),
    })
}

/// Which axis a declared size lies on — the two `box-sizing` subtractions
/// differ only in which pair of edges they take out.
#[derive(Clone, Copy)]
enum Axis {
    Horizontal,
    Vertical,
}

/// A declared size turned into a **content** size, honouring `box-sizing`.
fn content_from(style: &ComputedStyle, declared: Au, edges: BoxEdges, axis: Axis) -> Au {
    if style.box_sizing() == BoxSizing::ContentBox {
        return declared;
    }
    declared
        .saturating_sub(inner_of(edges, axis))
        .larger(Au::ZERO)
}

const fn inner_of(edges: BoxEdges, axis: Axis) -> Au {
    let border = edges.border();
    let padding = edges.padding();
    match axis {
        Axis::Horizontal => border.horizontal().saturating_add(padding.horizontal()),
        Axis::Vertical => border.vertical().saturating_add(padding.vertical()),
    }
}

/// A `height` that layout can use: a percentage resolves against the containing
/// block's height, and this engine never has a definite one to offer, so CSS
/// 2.1 §10.5's rule applies and the percentage computes to `auto`.
fn definite_height(style: &ComputedStyle, font_size: Au) -> Option<Au> {
    let Sizing::Fixed(length) = style.height() else {
        return None;
    };
    if length.is_percentage() {
        return None;
    }
    length.resolve_to_au(font_size, Au::ZERO)
}

fn edges_of(
    edges: LengthEdges,
    font_size: Au,
    container: Au,
    property: &'static str,
) -> Result<EdgeSizes, CssError> {
    resolved_edges(edges, font_size, container).ok_or_else(|| non_finite(property))
}

fn resolved_edges(edges: LengthEdges, font_size: Au, container: Au) -> Option<EdgeSizes> {
    Some(EdgeSizes::new(
        edges.top().resolve_to_au(font_size, container)?,
        edges.right().resolve_to_au(font_size, container)?,
        edges.bottom().resolve_to_au(font_size, container)?,
        edges.left().resolve_to_au(font_size, container)?,
    ))
}

fn non_finite(property: &'static str) -> CssError {
    CssError::unsupported(
        CssStage::Layout,
        format!("non-finite length in `{property}`"),
    )
}

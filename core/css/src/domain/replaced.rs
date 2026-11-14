//! Replaced elements (issue #83): what an `<img>` and its kin bring to layout
//! beyond their computed style, and how their used size follows from it.
//!
//! The size of a replaced box comes from three places (CSS 2.1 §10.3.2,
//! §10.6.2): the author's `width` / `height`, the resource's natural size once
//! it is loaded, and — before that — the ratio of the element's own `width` /
//! `height` attributes (WHATWG HTML §15.4.3). The resource is not the styled
//! tree's to know: layout asks the [`crate::NaturalSizes`] port, which answers
//! a [`ReplacedResource`] per node.
//!
//! Every length is an integer [`Au`] (`ADR-0016`): the ratio scales in `i64`
//! and saturates, never through a float.

use graphics::{AU_PER_PX, Au};
use html::TagName;

use crate::domain::dimension_value::DimensionValue;
use crate::domain::dom_snapshot::NodeRef;

/// The elements whose size comes from a resource rather than from CSS
/// (HTML Living Standard, "replaced elements").
///
/// Deliberately a short, closed list: an element outside it is laid out from
/// its own content, which is the correct answer for every non-replaced tag.
const REPLACED_TAGS: [TagName; 7] = [
    TagName::img(),
    TagName::video(),
    TagName::canvas(),
    TagName::iframe(),
    TagName::object(),
    TagName::embed(),
    TagName::svg(),
];

/// The elements whose `width` and `height` attributes also give a preferred
/// ratio before the resource loads (HTML §15.4.3, "map to the aspect-ratio
/// property"). `<canvas>` is on that list too, but its attributes are its
/// bitmap's — its natural size — which already carries the ratio.
const RATIO_TAGS: [TagName; 2] = [TagName::img(), TagName::video()];

/// The default object size of an embedded element with nothing else to go on
/// (CSS Images L3 §4.1): 300×150 CSS pixels.
const DEFAULT_OBJECT_WIDTH: Au = Au::from_raw(300 * AU_PER_PX);
const DEFAULT_OBJECT_HEIGHT: Au = Au::from_raw(150 * AU_PER_PX);

/// The natural size of a loaded resource, in CSS pixels — one image pixel per
/// CSS pixel (no `srcset` density).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NaturalSize {
    width: Au,
    height: Au,
}

impl NaturalSize {
    /// A `width × height` pixel resource, or `None` for an empty one or one
    /// too large to lay out.
    #[must_use]
    pub fn from_pixels(width: u32, height: u32) -> Option<Self> {
        let width = Au::from_whole_px(i32::try_from(width).ok()?)?;
        let height = Au::from_whole_px(i32::try_from(height).ok()?)?;
        let empty = width <= Au::ZERO || height <= Au::ZERO;
        if empty {
            return None;
        }
        Some(Self { width, height })
    }

    #[must_use]
    pub const fn width(self) -> Au {
        self.width
    }

    #[must_use]
    pub const fn height(self) -> Au {
        self.height
    }
}

/// What has become of the resource behind one replaced element — the answer
/// of the [`crate::NaturalSizes`] port.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ReplacedResource {
    /// Decoded: the box may take its natural size.
    Loaded(NaturalSize),
    /// Requested and not answered yet: the box's size may still change.
    Loading,
    /// There is none and there will be none — no `src`, a failed fetch or
    /// decode, or a pipeline that never fetches.
    Unavailable,
}

/// A preferred width-to-height ratio, both sides positive.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct AspectRatio {
    width: Au,
    height: Au,
}

impl AspectRatio {
    fn new(width: Au, height: Au) -> Option<Self> {
        let positive = width > Au::ZERO && height > Au::ZERO;
        positive.then_some(Self { width, height })
    }

    fn height_for(self, width: Au) -> Au {
        scale(width, self.height, self.width)
    }

    fn width_for(self, height: Au) -> Au {
        scale(height, self.width, self.height)
    }
}

/// `value × numerator / denominator`, in `i64`, saturating to the `Au` range.
fn scale(value: Au, numerator: Au, denominator: Au) -> Au {
    let product = i64::from(value.raw()).saturating_mul(i64::from(numerator.raw()));
    let quotient = product
        .checked_div(i64::from(denominator.raw()))
        .unwrap_or(0);
    let clamped = quotient.clamp(i64::from(i32::MIN), i64::from(i32::MAX));
    Au::from_raw(i32::try_from(clamped).unwrap_or(0))
}

/// The two axes an author gave a box — `None` for `auto`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DeclaredSize {
    width: Option<Au>,
    height: Option<Au>,
}

impl DeclaredSize {
    pub(crate) const fn new(width: Option<Au>, height: Option<Au>) -> Self {
        Self { width, height }
    }

    /// Whether both axes are declared, so no resource can change the size.
    pub(crate) const fn is_pinned(self) -> bool {
        self.width.is_some() && self.height.is_some()
    }
}

/// A used content-box size.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct UsedSize {
    width: Au,
    height: Au,
}

impl UsedSize {
    pub(crate) const fn width(self) -> Au {
        self.width
    }

    pub(crate) const fn height(self) -> Au {
        self.height
    }

    fn ratio(self) -> Option<AspectRatio> {
        AspectRatio::new(self.width, self.height)
    }
}

/// Which default a replaced element falls back on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum ReplacedKind {
    /// An `<img>`: nothing to show, so nothing to size — 0×0 (issue #83).
    Image,
    /// A `<canvas>`: its bitmap is its natural size, set by its `width` /
    /// `height` attributes, 300×150 when absent (HTML §4.12.5).
    Canvas(UsedSize),
    /// Every other replaced element: the 300×150 default object size.
    Embedded,
}

/// A replaced element as layout sees it: which default it falls back on, and
/// the ratio its own `width` / `height` attributes give. Crate-private: a
/// consumer only asks [`crate::StyledNode::is_replaced`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ReplacedElement {
    kind: ReplacedKind,
    attribute_ratio: Option<AspectRatio>,
}

impl ReplacedElement {
    /// The element `node_ref` is, if it is a replaced one.
    #[must_use]
    pub(crate) fn for_node(node_ref: NodeRef<'_>) -> Option<Self> {
        let tag = node_ref.tag()?;
        if !REPLACED_TAGS.contains(tag) {
            return None;
        }
        Some(Self {
            kind: kind_of(tag, node_ref),
            attribute_ratio: attribute_ratio(tag, node_ref),
        })
    }

    /// The used content size under `declared` for `resource` (CSS 2.1
    /// §10.3.2, §10.6.2): both axes declared win; one declared axis gives the
    /// other through the ratio, else through the natural size, else through the
    /// default; neither gives the natural size, else the default.
    pub(crate) fn used_size(self, resource: ReplacedResource, declared: DeclaredSize) -> UsedSize {
        let natural = loaded_size(resource)
            .map(natural_as_used)
            .or_else(|| self.own_natural_size());
        let ratio = natural.and_then(UsedSize::ratio).or(self.attribute_ratio);
        let fallback = natural.unwrap_or_else(|| self.default_size());
        let width = declared
            .width
            .or_else(|| scaled(ratio, declared.height, AspectRatio::width_for))
            .unwrap_or(fallback.width);
        let height = declared
            .height
            .or_else(|| scaled(ratio, declared.width, AspectRatio::height_for))
            .unwrap_or(fallback.height);
        UsedSize { width, height }
    }

    /// The natural size an element has without any resource: a canvas's
    /// bitmap.
    const fn own_natural_size(self) -> Option<UsedSize> {
        match self.kind {
            ReplacedKind::Canvas(bitmap) => Some(bitmap),
            ReplacedKind::Image | ReplacedKind::Embedded => None,
        }
    }

    const fn default_size(self) -> UsedSize {
        match self.kind {
            ReplacedKind::Image => UsedSize {
                width: Au::ZERO,
                height: Au::ZERO,
            },
            ReplacedKind::Canvas(bitmap) => bitmap,
            ReplacedKind::Embedded => UsedSize {
                width: DEFAULT_OBJECT_WIDTH,
                height: DEFAULT_OBJECT_HEIGHT,
            },
        }
    }
}

/// The other axis from `known` through `ratio`, when both exist.
fn scaled(
    ratio: Option<AspectRatio>,
    known: Option<Au>,
    other_axis: fn(AspectRatio, Au) -> Au,
) -> Option<Au> {
    let (ratio, length) = ratio.zip(known)?;
    Some(other_axis(ratio, length))
}

const fn natural_as_used(natural: NaturalSize) -> UsedSize {
    UsedSize {
        width: natural.width,
        height: natural.height,
    }
}

const fn loaded_size(resource: ReplacedResource) -> Option<NaturalSize> {
    match resource {
        ReplacedResource::Loaded(natural) => Some(natural),
        ReplacedResource::Loading | ReplacedResource::Unavailable => None,
    }
}

fn kind_of(tag: &TagName, node_ref: NodeRef<'_>) -> ReplacedKind {
    match tag {
        TagName::Img => ReplacedKind::Image,
        TagName::Canvas => ReplacedKind::Canvas(canvas_bitmap(node_ref)),
        _ => ReplacedKind::Embedded,
    }
}

/// A canvas's bitmap size: each attribute read with the rules for parsing
/// non-negative integers (HTML §2.3.4.2), its default when absent or invalid.
fn canvas_bitmap(node_ref: NodeRef<'_>) -> UsedSize {
    UsedSize {
        width: canvas_side(node_ref.attribute("width"), DEFAULT_OBJECT_WIDTH),
        height: canvas_side(node_ref.attribute("height"), DEFAULT_OBJECT_HEIGHT),
    }
}

fn canvas_side(raw: Option<&str>, default: Au) -> Au {
    raw.and_then(non_negative_integer)
        .and_then(Au::from_whole_px)
        .unwrap_or(default)
}

/// Leading white space, then digits; whatever follows is ignored.
fn non_negative_integer(raw: &str) -> Option<i32> {
    let rest = raw.trim_start_matches(|character: char| character.is_ascii_whitespace());
    let end = rest
        .find(|character: char| !character.is_ascii_digit())
        .unwrap_or(rest.len());
    rest.get(..end)?.parse().ok()
}

fn attribute_ratio(tag: &TagName, node_ref: NodeRef<'_>) -> Option<AspectRatio> {
    if !RATIO_TAGS.contains(tag) {
        return None;
    }
    let width = attribute_pixels(node_ref, "width")?;
    let height = attribute_pixels(node_ref, "height")?;
    AspectRatio::new(width, height)
}

/// A dimension attribute as a length — a percentage gives no ratio.
fn attribute_pixels(node_ref: NodeRef<'_>, name: &str) -> Option<Au> {
    DimensionValue::parse(node_ref.attribute(name)?)?.pixels()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn px(value: i32) -> Au {
        Au::from_whole_px(value).unwrap_or(Au::ZERO)
    }

    #[test]
    fn an_empty_resource_has_no_natural_size() {
        assert_eq!(NaturalSize::from_pixels(0, 10), None);
        assert!(NaturalSize::from_pixels(4, 2).is_some());
    }

    #[test]
    fn the_ratio_scales_in_integers() {
        let ratio = AspectRatio {
            width: px(3),
            height: px(2),
        };
        assert_eq!(ratio.height_for(px(30)), px(20));
        assert_eq!(ratio.width_for(px(20)), px(30));
    }
}

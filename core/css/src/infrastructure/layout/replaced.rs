//! Laying out a replaced element's own box (issue #83, CSS 2.1 §10.3.2,
//! §10.6.2).
//!
//! A replaced box has no content of its own to lay out: its size follows from
//! the author's `width` / `height`, its resource ([`crate::NaturalSizes`]) and
//! its attribute ratio (`crate::domain::replaced`). Its children — fallback
//! content — are never laid out.

use crate::application::ports::TextMeasurer;
use crate::domain::computed::intrinsic::IntrinsicSize;
use crate::domain::replaced::{DeclaredSize, ReplacedElement, ReplacedResource, UsedSize};
use crate::domain::styled_tree::StyledNode;
use crate::infrastructure::layout::box_model::BoxMetrics;
use crate::infrastructure::layout::context::LayoutContext;

/// A replaced element's box: its used content size and what it reports about
/// its resource.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReplacedBox {
    size: UsedSize,
    marker: IntrinsicSize,
}

impl ReplacedBox {
    pub const fn size(self) -> UsedSize {
        self.size
    }

    pub const fn marker(self) -> IntrinsicSize {
        self.marker
    }
}

/// How a node's own box is laid out, decided in one place from its tag and
/// its resource (asked again by each caller; the port answers the same within
/// one layout).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OwnBox {
    /// Not a replaced element: laid out from its content.
    Content,
    /// A replaced element whose resource will never come, laid out as its own
    /// `alt` text, like an `inline-block` holding it (WHATWG HTML §15.4.2).
    AlternativeText,
    /// A replaced element sized from its resource.
    Replaced(ReplacedElement, ReplacedResource),
}

impl OwnBox {
    /// What `node`'s box is: a loaded or loading resource never shows its
    /// `alt` text; an unavailable one does when the element carries one.
    pub fn of<M: TextMeasurer>(context: &LayoutContext<'_, M>, node: &StyledNode) -> Self {
        let Some(replaced) = node.replaced() else {
            return Self::Content;
        };
        let resource = context.resource_of(node.node());
        if resource == ReplacedResource::Unavailable && node.text().is_some() {
            return Self::AlternativeText;
        }
        Self::Replaced(replaced, resource)
    }

    /// Whether the box sizes itself as a replaced box rather than from content
    /// — so shrink-to-fit has nothing to measure.
    pub const fn sizes_itself(self) -> bool {
        matches!(self, Self::Replaced(..))
    }

    /// The replaced box under `metrics`, or `None` for a box laid out from
    /// content (its own or its `alt` text).
    pub fn replaced_box(self, metrics: BoxMetrics) -> Option<ReplacedBox> {
        let Self::Replaced(replaced, resource) = self else {
            return None;
        };
        let declared = DeclaredSize::new(metrics.width(), metrics.height());
        Some(ReplacedBox {
            size: replaced.used_size(resource, declared),
            marker: marker_for(resource, declared),
        })
    }
}

/// A loaded resource reports its natural size; one still loading leaves the
/// box `Pending` unless the author pinned both axes; an unavailable one never
/// changes the box again.
const fn marker_for(resource: ReplacedResource, declared: DeclaredSize) -> IntrinsicSize {
    match resource {
        ReplacedResource::Loaded(natural) => IntrinsicSize::Natural(natural),
        ReplacedResource::Loading if !declared.is_pinned() => IntrinsicSize::Pending,
        ReplacedResource::Loading | ReplacedResource::Unavailable => IntrinsicSize::Resolved,
    }
}

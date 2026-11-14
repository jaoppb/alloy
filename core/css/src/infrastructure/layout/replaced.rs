//! Laying out a replaced element's own box (issue #83, CSS 2.1 §10.3.2,
//! §10.6.2).
//!
//! A replaced box has no content of its own to lay out: its size follows from
//! the author's `width` / `height`, its resource ([`crate::NaturalSizes`]) and
//! its attribute ratio (`crate::domain::replaced`). Its children — fallback
//! content — are never laid out.

use crate::application::ports::TextMeasurer;
use crate::domain::computed::intrinsic::IntrinsicSize;
use crate::domain::replaced::{DeclaredSize, ReplacedResource, UsedSize};
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

/// The replaced box `node` lays out as under `metrics`, or `None` for an
/// element that is not replaced.
pub fn replaced_box<M: TextMeasurer>(
    context: &LayoutContext<'_, M>,
    node: &StyledNode,
    metrics: BoxMetrics,
) -> Option<ReplacedBox> {
    let replaced = node.replaced()?;
    let resource = context.resource_of(node.node());
    let declared = DeclaredSize::new(metrics.width(), metrics.height());
    Some(ReplacedBox {
        size: replaced.used_size(resource, declared),
        marker: marker_for(resource, declared),
    })
}

/// Whether `node` sizes itself as a replaced box rather than from content —
/// so shrink-to-fit has nothing to measure.
pub const fn sizes_itself(node: &StyledNode) -> bool {
    node.replaced().is_some()
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

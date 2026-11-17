//! [`NoResources`] — the [`NaturalSizes`] adapter for a pipeline that fetches
//! nothing (issue #83).

use crate::application::ports::NaturalSizes;
use crate::domain::dom_snapshot::SnapshotId;
use crate::domain::replaced::ReplacedResource;

/// Every replaced element's resource is unavailable.
///
/// An `<img>` shows its `alt` text or nothing, every other replaced element
/// takes the default object size. What [`crate::BlockLayout`] uses until it is
/// given a real source.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NoResources;

impl NaturalSizes for NoResources {
    fn resource_of(&self, _node: SnapshotId) -> ReplacedResource {
        ReplacedResource::Unavailable
    }
}

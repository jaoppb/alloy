//! Value objects of the region seam — no behavior beyond identity (`ADR-0010`).

mod content_region;
mod context_id;
mod layout_epoch;
mod region_id;

pub use content_region::ContentRegion;
pub use context_id::ContextId;
pub use layout_epoch::LayoutEpoch;
pub use region_id::RegionId;

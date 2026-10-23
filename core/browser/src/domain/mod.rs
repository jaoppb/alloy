//! Entities, value objects and typed errors of the browsing context — no
//! threads, no I/O (`ADR-0010`).

mod context_id;
mod document;
mod effect;
mod fetch;
mod image_store;
mod link;
mod navigation;
mod region;
mod regions;
mod stats;
mod subresource;

pub use context_id::ContextId;
pub use document::{Document, StylesheetRejected};
pub use effect::BrowserEffect;
pub use fetch::{FetchFailure, FetchPayload, FetchRequest, FetchResult, LoadedDocument};
pub use image_store::{ImageStore, StoreSnapshot};
pub use link::LinkDestination;
pub use navigation::{NavigationId, Navigations};
pub use region::{ContentRegion, LayoutEpoch, LinkTarget, RegionId};
pub use regions::{ContentRegions, RegionError};
pub use stats::ContextStats;
pub use subresource::{ImageRequest, StylesheetRequest, SubresourceRequest, Subresources};

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
pub use document::Document;
pub use effect::{BrowserEffect, Effects};
pub use fetch::{FetchFailure, FetchPayload, FetchRequest, FetchResult, LoadedDocument};
pub use image_store::{ImageStore, StoreSnapshot};
pub use link::LinkDestination;
pub use navigation::{NavigationId, Navigations};
pub use region::{ContentRegion, LayoutEpoch, LinkTarget, LinkTargets, RegionId};
pub use regions::{ContentRegions, RegionError};
pub(crate) use stats::Accepted;
pub use stats::ContextStats;
pub use subresource::{ImageRequest, StylesheetRequest, SubresourceRequest, Subresources};

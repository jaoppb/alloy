//! Entities, value objects and typed errors of the browsing context — no
//! threads, no I/O (`ADR-0010`).

mod image_store;
mod subresource;

pub use image_store::{ImageStore, StoreSnapshot};
pub use subresource::{ImageRequest, StylesheetRequest, SubresourceRequest, Subresources};

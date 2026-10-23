//! # `browser` — the browsing context
//!
//! Everything the old `alloy` `Session` knew about *the document*: the DOM
//! tree, its base URL, the fetched stylesheets and images, the clickable link
//! regions of the last layout, and which navigation is the current one. It is
//! a **pure state machine** (#110 decision 4): commands go in, [`BrowserEffect`]s
//! come out, and `alloy`'s executor does the threads, the sockets and the
//! omnibox. Nothing here spawns, sleeps, or touches a transport — an
//! `arch-lint.toml` rule keeps it that way — so every behaviour is testable
//! without a clock.
//!
//! Sibling of `core/ui` (`ADR-0026`): neither depends on the other.
//!
//! ## Layout (`ADR-0010` §1)
//!
//! - [`domain`] — ids, documents, requests, effects, typed errors.
//! - [`application`] — the [`BrowsingContext`] and the [`SubresourceDiscoverer`] port.
//! - [`infrastructure`] — [`MarkupDiscoverer`], the tag-based adapter.

#![forbid(unsafe_code)]

pub mod application;
pub mod domain;
pub mod infrastructure;

pub use application::{
    BrowsingContext, ErrorPageError, SubresourceDiscoverer, document_base_url, error_page,
};
pub use domain::{
    BrowserEffect, ContentRegion, ContextId, ContextStats, Document, FetchFailure, FetchPayload,
    FetchRequest, FetchResult, ImageRequest, ImageStore, LayoutEpoch, LinkDestination, LinkTarget,
    LoadedDocument, NavigationId, Navigations, RegionError, RegionId, StoreSnapshot,
    StylesheetRejected, StylesheetRequest, SubresourceRequest, Subresources,
};
pub use infrastructure::MarkupDiscoverer;

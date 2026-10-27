//! What the browsing context asks `alloy` to fetch, and what comes back.
//!
//! The context never fetches (#110 decision 4): it emits a [`FetchRequest`]
//! stamped with a [`NavigationId`], an executor runs it on whatever thread it
//! likes, and hands back a [`FetchResult`] carrying the same stamp.

use css::MediaQuery;
use dom::DomTree;
use graphics::{Framebuffer, ImageId};
use network::Url;
use region::ContextId;

use crate::domain::{NavigationId, SubresourceRequest};

/// Something for an executor to fetch.
#[derive(Clone, Debug, PartialEq)]
pub enum FetchRequest {
    /// The document a navigation targets.
    Document(Url),
    /// A stylesheet or image the loaded document references.
    Subresource(SubresourceRequest),
}

/// Why a fetch produced nothing usable, already rendered for a person to read.
///
/// The cause (a transport error, a policy denial, a non-`2xx` status) lives in
/// the executor's own error type; the browsing context only needs the message
/// to show and to log.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct FetchFailure(String);

impl FetchFailure {
    /// A failure described by `description`.
    #[must_use]
    pub fn new(description: impl Into<String>) -> Self {
        Self(description.into())
    }
}

/// A navigation's response, parsed: the tree and the URL it answered for.
#[derive(Debug)]
pub struct LoadedDocument {
    tree: DomTree,
    url: Url,
}

impl LoadedDocument {
    /// The `tree` parsed from the response to `url`.
    #[must_use]
    pub const fn new(tree: DomTree, url: Url) -> Self {
        Self { tree, url }
    }

    pub(crate) fn into_parts(self) -> (DomTree, Url) {
        (self.tree, self.url)
    }
}

/// What one finished fetch produced.
#[derive(Debug)]
pub enum FetchPayload {
    /// The navigation's document.
    Document(Result<LoadedDocument, FetchFailure>),
    /// A stylesheet's text, with the media it was linked for.
    Stylesheet(MediaQuery, Result<String, FetchFailure>),
    /// An image's decoded pixels, for the node `ImageId` names.
    Image(ImageId, Result<Framebuffer, FetchFailure>),
}

/// A finished fetch, stamped with the navigation it was started for.
#[derive(Debug)]
pub struct FetchResult {
    navigation: NavigationId,
    payload: FetchPayload,
}

impl FetchResult {
    /// `payload`, produced for `navigation`.
    #[must_use]
    pub const fn new(navigation: NavigationId, payload: FetchPayload) -> Self {
        Self {
            navigation,
            payload,
        }
    }

    /// The navigation this result belongs to.
    #[must_use]
    pub const fn navigation(&self) -> NavigationId {
        self.navigation
    }

    /// The context this result is for.
    #[must_use]
    pub const fn context(&self) -> ContextId {
        self.navigation.context()
    }

    pub(crate) fn into_payload(self) -> FetchPayload {
        self.payload
    }
}

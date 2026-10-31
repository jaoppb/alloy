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

use crate::domain::{DocumentRequest, NavigationId, SubresourceRequest};

/// Something for an executor to fetch.
#[derive(Clone, Debug, PartialEq)]
pub enum FetchRequest {
    /// The document a navigation targets.
    Document(DocumentRequest),
    /// A stylesheet or image the loaded document references.
    Subresource(SubresourceRequest),
}

impl FetchRequest {
    /// The address being fetched, for logs.
    #[must_use]
    pub const fn url(&self) -> &Url {
        match self {
            Self::Document(document) => document.url(),
            Self::Subresource(SubresourceRequest::Stylesheet(stylesheet)) => stylesheet.url(),
            Self::Subresource(SubresourceRequest::Image(image)) => image.url(),
        }
    }

    /// The payload that reports this request failed with `failure`, shaped
    /// like the one a success would have produced — so an executor that could
    /// not run the request at all (a worker that panicked) still answers it.
    #[must_use]
    pub fn failed(&self, failure: FetchFailure) -> FetchPayload {
        match self {
            Self::Document(_) => FetchPayload::Document(Err(failure)),
            Self::Subresource(SubresourceRequest::Stylesheet(stylesheet)) => {
                FetchPayload::Stylesheet(stylesheet.media().clone(), Err(failure))
            }
            Self::Subresource(SubresourceRequest::Image(image)) => {
                FetchPayload::Image(image.id(), Err(failure))
            }
        }
    }
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

#[cfg(test)]
mod tests {
    use css::MediaQuery;
    use graphics::ImageId;
    use network::Url;

    use super::{DocumentRequest, FetchFailure, FetchPayload, FetchRequest};
    use crate::domain::{ImageRequest, StylesheetRequest, SubresourceRequest};

    fn url() -> Url {
        Url::parse("http://example.com/a").expect("a valid test URL")
    }

    fn failure() -> FetchFailure {
        FetchFailure::new("worker panicked")
    }

    #[test]
    fn every_kind_of_request_names_its_url() {
        let sheet = StylesheetRequest::new(url(), MediaQuery::always());
        let image = ImageRequest::new(ImageId::new(1), url());

        assert_eq!(
            FetchRequest::Document(DocumentRequest::get(url())).url(),
            &url()
        );
        assert_eq!(
            FetchRequest::Subresource(SubresourceRequest::Stylesheet(sheet)).url(),
            &url()
        );
        assert_eq!(
            FetchRequest::Subresource(SubresourceRequest::Image(image)).url(),
            &url()
        );
    }

    #[test]
    fn a_failed_document_answers_as_a_document() {
        let request = FetchRequest::Document(DocumentRequest::get(url()));

        assert!(matches!(
            request.failed(failure()),
            FetchPayload::Document(Err(_))
        ));
    }

    #[test]
    fn a_failed_stylesheet_keeps_its_media() {
        let sheet = StylesheetRequest::new(url(), MediaQuery::always());
        let request = FetchRequest::Subresource(SubresourceRequest::Stylesheet(sheet));

        let FetchPayload::Stylesheet(media, Err(_)) = request.failed(failure()) else {
            panic!("a stylesheet request fails as a stylesheet payload");
        };
        assert_eq!(media, MediaQuery::always());
    }

    #[test]
    fn a_failed_image_keeps_its_id() {
        let image = ImageRequest::new(ImageId::new(7), url());
        let request = FetchRequest::Subresource(SubresourceRequest::Image(image));

        let FetchPayload::Image(id, Err(_)) = request.failed(failure()) else {
            panic!("an image request fails as an image payload");
        };
        assert_eq!(id, ImageId::new(7));
    }
}

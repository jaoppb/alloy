//! What a document references: a *closed* set of typed fetch requests.
//!
//! How references are found is the open extension point
//! ([`crate::SubresourceDiscoverer`]); this module only names what can be
//! fetched, so a new kind of subresource is a new variant every fetch executor
//! must then handle.

use css::MediaQuery;
use graphics::ImageId;
use network::Url;

/// A `<link rel="stylesheet" href>` target, already resolved against the
/// document's base URL ([`crate::document_base_url`]), gated by its `media=`
/// attribute.
#[derive(Clone, Debug, PartialEq)]
pub struct StylesheetRequest {
    url: Url,
    media: MediaQuery,
}

impl StylesheetRequest {
    /// A request for the stylesheet at `url` with `media` constraints.
    #[must_use]
    pub const fn new(url: Url, media: MediaQuery) -> Self {
        Self { url, media }
    }

    /// Where to fetch the stylesheet from.
    #[must_use]
    pub const fn url(&self) -> &Url {
        &self.url
    }

    /// Which media the stylesheet applies to.
    #[must_use]
    pub const fn media(&self) -> &MediaQuery {
        &self.media
    }
}

/// An `<img src>` target, tagged with the [`ImageId`] the painter will look up
/// for that same node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageRequest {
    id: ImageId,
    url: Url,
}

impl ImageRequest {
    /// A request for the image at `url`, painted for the node `id`.
    #[must_use]
    pub const fn new(id: ImageId, url: Url) -> Self {
        Self { id, url }
    }

    /// The identity the painter looks this image up by.
    #[must_use]
    pub const fn id(&self) -> ImageId {
        self.id
    }

    /// Where to fetch the image from.
    #[must_use]
    pub const fn url(&self) -> &Url {
        &self.url
    }
}

/// Everything a page can ask to have fetched — closed on purpose.
#[derive(Clone, Debug, PartialEq)]
pub enum SubresourceRequest {
    /// An external stylesheet.
    Stylesheet(StylesheetRequest),
    /// A decodable image.
    Image(ImageRequest),
}

/// One page's discovered subresources, in document order.
///
/// A reference that fails to resolve (a relative URL with no scheme, an
/// unsupported one) is silently dropped — a broken subresource link must
/// never abort navigation.
#[derive(Clone, Debug, Default)]
pub struct Subresources {
    requests: Vec<SubresourceRequest>,
}

impl Subresources {
    /// Appends `request`.
    pub fn push(&mut self, request: SubresourceRequest) {
        self.requests.push(request);
    }

    /// The requests in document order.
    pub fn iter(&self) -> std::slice::Iter<'_, SubresourceRequest> {
        self.requests.iter()
    }
}

impl<'a> IntoIterator for &'a Subresources {
    type Item = &'a SubresourceRequest;
    type IntoIter = std::slice::Iter<'a, SubresourceRequest>;

    fn into_iter(self) -> Self::IntoIter {
        self.requests.iter()
    }
}

impl IntoIterator for Subresources {
    type Item = SubresourceRequest;
    type IntoIter = std::vec::IntoIter<SubresourceRequest>;

    fn into_iter(self) -> Self::IntoIter {
        self.requests.into_iter()
    }
}

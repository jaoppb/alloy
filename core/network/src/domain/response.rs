//! [`HttpResponse`] — the aggregate a transport hands back.
//!
//! The media type is **resolved at construction**, not re-parsed by every
//! caller: a response either knows what it is or knows that its `Content-Type`
//! was unusable, and it decides that once.

use core::fmt;

use crate::domain::body::Body;
use crate::domain::header_map::{HeaderMap, HeaderName};
use crate::domain::media_type::MediaType;
use crate::domain::status::StatusCode;

/// A response, with its body already decoded.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct HttpResponse {
    status: StatusCode,
    headers: HeaderMap,
    body: Body,
    media_type: Option<MediaType>,
}

impl HttpResponse {
    /// Assemble a response and resolve its media type from the field section.
    ///
    /// A `Content-Type` that does not parse leaves [`HttpResponse::media_type`]
    /// as `None` rather than failing the whole exchange: an unusable
    /// `Content-Type` is a common server bug, and the body is still the body.
    /// A body whose *charset* could not be resolved never gets here — the
    /// transport raises [`NetworkError::Decode`] before constructing the
    /// response.
    ///
    /// [`NetworkError::Decode`]: crate::domain::error::NetworkError::Decode
    #[must_use]
    pub fn new(status: StatusCode, headers: HeaderMap, body: Body) -> Self {
        let media_type = Self::resolve_media_type(&headers);
        Self {
            status,
            headers,
            body,
            media_type,
        }
    }

    fn resolve_media_type(headers: &HeaderMap) -> Option<MediaType> {
        headers
            .text(&HeaderName::content_type())
            .and_then(|raw| MediaType::parse(raw).ok())
    }

    /// The status.
    #[must_use]
    pub const fn status(&self) -> StatusCode {
        self.status
    }

    /// The field section.
    #[must_use]
    pub const fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    /// The decoded payload.
    #[must_use]
    pub const fn body(&self) -> &Body {
        &self.body
    }

    /// The media type, resolved once from `Content-Type`.
    #[must_use]
    pub const fn media_type(&self) -> Option<&MediaType> {
        self.media_type.as_ref()
    }

    /// The same response carrying a different payload — how the transport
    /// installs the body it decoded.
    #[must_use]
    pub fn with_body(mut self, body: Body) -> Self {
        self.body = body;
        self
    }
}

impl fmt::Display for HttpResponse {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} ({})", self.status, self.body)
    }
}

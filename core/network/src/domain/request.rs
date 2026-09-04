//! [`HttpRequest`] — the request aggregate.
//!
//! It is what crosses
//! [`RequestPolicy::decide`](crate::application::ports::RequestPolicy::decide)
//! and [`HttpTransport::execute`](crate::application::ports::HttpTransport::execute).
//!
//! It is immutable in the pipeline sense (`ADR-0010:114-117`): every "change"
//! is a `with_…` that consumes and returns a new value, so a policy can rewrite
//! a request without any caller wondering whether the one it still holds
//! changed underneath it.

use core::fmt;

use crate::domain::body::Body;
use crate::domain::header_map::{HeaderMap, HeaderName, HeaderValue};
use crate::domain::method::Method;
use crate::domain::url::Url;

/// A request to make.
///
/// The `Host` field is not stored: it is a function of [`Url::authority`], and
/// keeping it in the map would let the two disagree. The HTTP/1.1 serializer
/// writes it from the URL at send time.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct HttpRequest {
    method: Method,
    url: Url,
    headers: HeaderMap,
    body: Body,
}

impl HttpRequest {
    /// A request with no extra fields and no body.
    #[must_use]
    pub const fn new(method: Method, url: Url) -> Self {
        Self {
            method,
            url,
            headers: HeaderMap::new(),
            body: Body::empty(),
        }
    }

    /// A `GET` for `url`.
    #[must_use]
    pub const fn get(url: Url) -> Self {
        Self::new(Method::Get, url)
    }

    /// The same request with one more field set.
    #[must_use]
    pub fn with_header(mut self, name: HeaderName, value: HeaderValue) -> Self {
        self.headers.set(name, value);
        self
    }

    /// The same request carrying `body`.
    #[must_use]
    pub fn with_body(mut self, body: Body) -> Self {
        self.body = body;
        self
    }

    /// The same request aimed at a different URL — what a redirect hop and a
    /// [`PolicyVerdict::Rewrite`](crate::application::ports::PolicyVerdict::Rewrite)
    /// both produce.
    #[must_use]
    pub fn with_url(mut self, url: Url) -> Self {
        self.url = url;
        self
    }

    /// The same request with a different method, and — since a method rewrite
    /// on a redirect always drops the payload (RFC 9110 §15.4.4) — no body.
    #[must_use]
    pub fn rewritten_to(mut self, method: Method) -> Self {
        self.method = method;
        self.body = Body::empty();
        self
    }

    /// The same request without `name`.
    #[must_use]
    pub fn without_header(mut self, name: &HeaderName) -> Self {
        self.headers.remove(name);
        self
    }

    /// The method.
    #[must_use]
    pub const fn method(&self) -> Method {
        self.method
    }

    /// The target URL.
    #[must_use]
    pub const fn url(&self) -> &Url {
        &self.url
    }

    /// The caller-supplied field section.
    #[must_use]
    pub const fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    /// The payload.
    #[must_use]
    pub const fn body(&self) -> &Body {
        &self.body
    }
}

impl fmt::Display for HttpRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} {}", self.method, self.url)
    }
}

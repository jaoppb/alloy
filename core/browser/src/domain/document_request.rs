//! The request a navigation asks `alloy` to make for a document.

use network::{Body, HttpRequest, Method, Url};

/// What to ask for when a navigation fetches its document.
///
/// Only a `GET` is issued today; the method and body are here so a form
/// submission (`method=post`, #88) is a [`BrowserEffect::Fetch`] like any other
/// instead of a new effect (#110, #114).
///
/// [`BrowserEffect::Fetch`]: crate::BrowserEffect::Fetch
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocumentRequest {
    url: Url,
    method: Method,
    body: Body,
}

impl DocumentRequest {
    /// A `GET` for `url`, with no body: what a typed address or a link asks for.
    #[must_use]
    pub const fn get(url: Url) -> Self {
        Self {
            url,
            method: Method::Get,
            body: Body::empty(),
        }
    }

    /// `method` for `url`, carrying `body`.
    #[must_use]
    pub const fn new(url: Url, method: Method, body: Body) -> Self {
        Self { url, method, body }
    }

    /// The address the document is fetched from, and the one it answers for.
    #[must_use]
    pub const fn url(&self) -> &Url {
        &self.url
    }

    /// How the document is requested.
    #[must_use]
    pub const fn method(&self) -> Method {
        self.method
    }

    /// The payload sent with the request; empty for a `GET`.
    #[must_use]
    pub const fn body(&self) -> &Body {
        &self.body
    }

    /// The wire-level request this asks for: the one place a document request
    /// becomes an [`HttpRequest`], so the executor and a [`RequestPolicy`] see
    /// the same method and body.
    ///
    /// [`RequestPolicy`]: network::RequestPolicy
    #[must_use]
    pub fn into_http_request(self) -> HttpRequest {
        let request = HttpRequest::new(self.method, self.url);
        if self.body.is_empty() {
            return request;
        }
        request.with_body(self.body)
    }
}

#[cfg(test)]
mod tests {
    use network::{Body, Method, Url};

    use super::DocumentRequest;

    fn url() -> Url {
        Url::parse("http://example.com/form").expect("a valid test URL")
    }

    #[test]
    fn a_get_has_no_body() {
        let request = DocumentRequest::get(url());

        assert_eq!(request.method(), Method::Get);
        assert!(request.body().is_empty());
        assert_eq!(request.url(), &url());
    }

    #[test]
    fn the_http_request_carries_the_method_and_body() {
        let request =
            DocumentRequest::new(url(), Method::Post, Body::from_text("a=1")).into_http_request();

        assert_eq!(request.method(), Method::Post);
        assert_eq!(request.url(), &url());
        assert_eq!(request.body().as_bytes(), b"a=1");
    }

    #[test]
    fn a_get_becomes_a_bodyless_http_request() {
        let request = DocumentRequest::get(url()).into_http_request();

        assert_eq!(request.method(), Method::Get);
        assert!(request.body().is_empty());
    }

    #[test]
    fn a_post_keeps_its_method_and_body() {
        let request = DocumentRequest::new(url(), Method::Post, Body::from_text("a=1"));

        assert_eq!(request.method(), Method::Post);
        assert_eq!(request.body().as_bytes(), b"a=1");
    }
}

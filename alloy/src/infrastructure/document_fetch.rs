//! Document fetching (v0.5 Phase I4): `DocumentRequest -> RequestPolicy ->
//! HttpTransport -> core/html -> LoadedDocument`.
//!
//! The document half of [`HttpFetcher`]: the request goes through
//! [`HttpFetcher::send`] (policy before mechanism), then the body is checked
//! and parsed.

use browser::{DocumentRequest, LoadedDocument};
use network::{HttpTransport, RequestPolicy};

use super::http_fetch::HttpFetcher;
use crate::application::ports::ImageDecoder;
use crate::error::AlloyError;

impl<T: HttpTransport, P: RequestPolicy, D: ImageDecoder> HttpFetcher<T, P, D> {
    /// Fetches the document `document` asks for and parses the response body
    /// as HTML.
    ///
    /// A response that "succeeds" at the transport layer but carries a
    /// non-`2xx` status, an empty body, or bytes that are not UTF-8 text is a
    /// typed `Err` here, not an empty document: the event loop turns any `Err`
    /// into the visible error card, where a silently-parsed `""` would just be
    /// a blank window (v0.5 Phase I4).
    pub(super) fn fetch_document(
        &self,
        document: &DocumentRequest,
    ) -> Result<LoadedDocument, AlloyError> {
        let url = document.url().clone();
        let response = self.send(document.to_http_request())?;
        if response.body().is_empty() {
            return Err(AlloyError::EmptyDocument {
                url: url.to_string(),
            });
        }
        let text = response
            .body()
            .as_str()
            .ok_or_else(|| AlloyError::NonTextualDocument {
                url: url.to_string(),
            })?;
        let tree = crate::application::html_parse::parse_html(text)?;
        Ok(LoadedDocument::new(tree, url))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use browser::{DocumentRequest, LoadedDocument};
    use network::{
        AllowAllPolicy, Body, HeaderMap, HttpRequest, HttpResponse, Method, MockTransport,
        NetworkError, PolicyVerdict, RequestPolicy, StatusCode, Url,
    };

    use crate::error::AlloyError;
    use crate::infrastructure::{HttpFetcher, PngImageDecoder};

    fn navigate_to(url: &Url, response: HttpResponse) -> Result<LoadedDocument, AlloyError> {
        let transport = MockTransport::new().with_response(url.clone(), response);
        let fetcher = HttpFetcher::new(transport, AllowAllPolicy::new(), PngImageDecoder);
        fetcher.fetch_document(&DocumentRequest::get(url.clone()))
    }

    fn html_ok(body: &str) -> HttpResponse {
        HttpResponse::new(StatusCode::OK, HeaderMap::new(), Body::from_text(body))
    }

    #[test]
    fn a_normal_200_html_body_parses() {
        let url = Url::parse("http://example.com/").unwrap();
        let result = navigate_to(&url, html_ok("<html><body>hi</body></html>"));
        assert!(
            result.is_ok(),
            "a plain 200 HTML body must parse, got {result:?}"
        );
    }

    #[test]
    fn an_empty_200_body_is_a_typed_error() {
        let url = Url::parse("http://example.com/empty").unwrap();
        let result = navigate_to(&url, html_ok(""));
        assert!(
            matches!(result, Err(AlloyError::EmptyDocument { .. })),
            "got {result:?}"
        );
    }

    #[test]
    fn a_204_is_a_typed_error() {
        let url = Url::parse("http://example.com/no-content").unwrap();
        let response = HttpResponse::new(
            StatusCode::new(204).unwrap(),
            HeaderMap::new(),
            Body::empty(),
        );
        let result = navigate_to(&url, response);
        assert!(
            matches!(result, Err(AlloyError::EmptyDocument { .. })),
            "got {result:?}"
        );
    }

    #[test]
    fn a_304_is_a_typed_http_status_error() {
        let url = Url::parse("http://example.com/cached").unwrap();
        let response = HttpResponse::new(StatusCode::NOT_MODIFIED, HeaderMap::new(), Body::empty());
        let result = navigate_to(&url, response);
        assert!(
            matches!(result, Err(AlloyError::HttpStatus { status: 304, .. })),
            "got {result:?}"
        );
    }

    #[test]
    fn a_404_is_a_typed_http_status_error() {
        let url = Url::parse("http://example.com/missing").unwrap();
        let response = HttpResponse::new(
            StatusCode::NOT_FOUND,
            HeaderMap::new(),
            Body::from_text("<title>404</title>"),
        );
        let result = navigate_to(&url, response);
        assert!(
            matches!(result, Err(AlloyError::HttpStatus { status: 404, .. })),
            "got {result:?}"
        );
    }

    #[test]
    fn a_non_utf8_body_is_a_typed_error() {
        let url = Url::parse("http://example.com/binary").unwrap();
        // No Content-Type: core/network leaves the bytes untranscoded, so a
        // non-UTF-8 payload survives MockTransport verbatim.
        let response = HttpResponse::new(
            StatusCode::OK,
            HeaderMap::new(),
            Body::from_bytes(vec![0xFF, 0xFE, 0x00, 0x9C]),
        );
        let result = navigate_to(&url, response);
        assert!(
            matches!(result, Err(AlloyError::NonTextualDocument { .. })),
            "got {result:?}"
        );
    }

    /// Denies everything, quoting the method and body it was asked about.
    struct QuotingPolicy;

    impl RequestPolicy for QuotingPolicy {
        fn decide(&self, request: &HttpRequest) -> Result<PolicyVerdict, NetworkError> {
            Ok(PolicyVerdict::deny(format!(
                "{} carrying {} body bytes",
                request.method(),
                request.body().len()
            )))
        }
    }

    #[test]
    fn a_documents_method_and_body_reach_the_policy() {
        let url = Url::parse("http://example.com/form").unwrap();
        let document = DocumentRequest::new(url, Method::Post, Body::from_text("a=1"));

        let fetcher = HttpFetcher::new(MockTransport::new(), QuotingPolicy, PngImageDecoder);

        let result = fetcher.fetch_document(&document);

        let message = result.unwrap_err().to_string();
        assert!(
            message.contains("POST carrying 3 body bytes"),
            "got {message}"
        );
    }
}

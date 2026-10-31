//! What a [`FetchRequest`] means over HTTP: the one place a request variant
//! maps to how it is fetched, and a response maps to a [`FetchPayload`].
//!
//! Every request, document (in `document`) or subresource, goes through [`HttpFetcher::send`]:
//! policy runs before mechanism (`PRD-009` §3.3), so `decide` is consulted
//! before [`HttpTransport::execute`] ever opens a socket and a denial costs no
//! connection. Runs wherever the executor runs it — a worker thread in
//! production, the caller's own thread in a deterministic test.

use browser::{FetchPayload, FetchRequest, SubresourceRequest};
use graphics::Framebuffer;
use network::{HttpRequest, HttpResponse, HttpTransport, RequestPolicy, StatusCode, Url};

use crate::application::ports::ImageDecoder;
use crate::error::AlloyError;

/// Fetches over one transport, under one policy, decoding images with one
/// decoder.
pub struct HttpFetcher<T, P, D> {
    transport: T,
    policy: P,
    decoder: D,
}

impl<T: HttpTransport, P: RequestPolicy, D: ImageDecoder> HttpFetcher<T, P, D> {
    /// A fetcher over `transport`, `policy` and `decoder`.
    pub const fn new(transport: T, policy: P, decoder: D) -> Self {
        Self {
            transport,
            policy,
            decoder,
        }
    }

    /// Fetches `request`. A failure is already a message for the error page;
    /// the cause stays in [`AlloyError`] and the log.
    pub fn fetch(&self, request: &FetchRequest) -> FetchPayload {
        match request {
            FetchRequest::Document(document) => {
                FetchPayload::Document(self.fetch_document(document).map_err(Into::into))
            }
            FetchRequest::Subresource(SubresourceRequest::Stylesheet(stylesheet)) => {
                let text = self.fetch_stylesheet_text(stylesheet.url());
                FetchPayload::Stylesheet(stylesheet.media().clone(), text.map_err(Into::into))
            }
            FetchRequest::Subresource(SubresourceRequest::Image(image)) => {
                let frame = self.fetch_image(image.url());
                FetchPayload::Image(image.id(), frame.map_err(Into::into))
            }
        }
    }

    /// Sends `requested` through the policy, then the transport, and rejects a
    /// non-`2xx` answer: a fetch that redirected to an error page still
    /// "succeeds" at the transport layer, and an HTML 404/500 body must never
    /// reach a parser as if it were the resource.
    fn send(&self, requested: HttpRequest) -> Result<HttpResponse, AlloyError> {
        let verdict = self.policy.decide(&requested)?;
        let request = verdict.into_request(requested)?;
        let response = self.transport.execute(&request)?;
        ensure_success(request.url(), response.status())?;
        Ok(response)
    }

    fn fetch_stylesheet_text(&self, url: &Url) -> Result<String, AlloyError> {
        let response = self.send(HttpRequest::get(url.clone()))?;
        let body = response
            .body()
            .as_str()
            .ok_or_else(|| AlloyError::NonTextualStylesheet { url: url.clone() })?
            .to_owned();
        tracing::debug!(
            %url,
            status = response.status().code(),
            bytes = body.len(),
            "stylesheet fetched"
        );
        Ok(body)
    }

    fn fetch_image(&self, url: &Url) -> Result<Framebuffer, AlloyError> {
        let response = self.send(HttpRequest::get(url.clone()))?;
        self.decoder.decode(response.body().as_bytes())
    }
}

/// Rejects a non-`2xx` status with a typed [`AlloyError::HttpStatus`].
fn ensure_success(url: &Url, status: StatusCode) -> Result<(), AlloyError> {
    if status.is_success() {
        return Ok(());
    }
    Err(AlloyError::HttpStatus {
        url: url.to_string(),
        status: status.code(),
    })
}

mod document;

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use network::{
        AllowAllPolicy, Body, HeaderMap, HttpRequest, HttpResponse, MockTransport, NetworkError,
        PolicyVerdict, RequestPolicy, StatusCode, Url,
    };

    use super::HttpFetcher;
    use crate::error::AlloyError;
    use crate::infrastructure::PngImageDecoder;

    #[test]
    fn a_non_2xx_stylesheet_status_is_a_typed_error_not_an_empty_body() {
        let sheet_url = Url::parse("http://example.com/missing.css").unwrap();
        let not_found = HttpResponse::new(
            StatusCode::NOT_FOUND,
            HeaderMap::new(),
            Body::from_text("<!doctype html><title>404</title>"),
        );
        let transport = MockTransport::new().with_response(sheet_url.clone(), not_found);
        let fetcher = HttpFetcher::new(transport, AllowAllPolicy::new(), PngImageDecoder);

        let result = fetcher.fetch_stylesheet_text(&sheet_url);

        assert!(
            matches!(result, Err(AlloyError::HttpStatus { status: 404, .. })),
            "a 404 error page must not reach the CSS parser as a stylesheet"
        );
    }

    #[test]
    fn a_non_utf8_stylesheet_is_a_typed_error_not_an_empty_sheet() {
        let sheet_url = Url::parse("http://example.com/binary.css").unwrap();
        let binary = HttpResponse::new(
            StatusCode::OK,
            HeaderMap::new(),
            Body::from_bytes(vec![0xFF, 0xFE, 0x00, 0x9C]),
        );
        let transport = MockTransport::new().with_response(sheet_url.clone(), binary);
        let fetcher = HttpFetcher::new(transport, AllowAllPolicy::new(), PngImageDecoder);

        let result = fetcher.fetch_stylesheet_text(&sheet_url);

        assert!(
            matches!(result, Err(AlloyError::NonTextualStylesheet { .. })),
            "got {result:?}"
        );
    }

    struct DenyAll;

    impl RequestPolicy for DenyAll {
        fn decide(&self, _request: &HttpRequest) -> Result<PolicyVerdict, NetworkError> {
            Ok(PolicyVerdict::deny("blocked"))
        }
    }

    #[test]
    fn a_policy_denial_stops_a_subresource_before_the_transport() {
        let sheet_url = Url::parse("http://example.com/a.css").unwrap();
        let transport = MockTransport::new().with_response(
            sheet_url.clone(),
            HttpResponse::new(StatusCode::OK, HeaderMap::new(), Body::from_text("p {}")),
        );
        let fetcher = HttpFetcher::new(transport, DenyAll, PngImageDecoder);

        let result = fetcher.fetch_stylesheet_text(&sheet_url);

        assert!(
            matches!(
                result,
                Err(AlloyError::Network(NetworkError::PolicyDenied { .. }))
            ),
            "got {result:?}"
        );
    }
}

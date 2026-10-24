//! What a [`FetchRequest`] means over HTTP: the one place a request variant
//! maps to how it is fetched, and a response maps to a [`FetchPayload`].
//!
//! Runs wherever the executor runs it — a worker thread in production, the
//! caller's own thread in a deterministic test.

use browser::{
    FetchFailure, FetchPayload, FetchRequest, LoadedDocument, StylesheetRequest, SubresourceRequest,
};
use graphics::Framebuffer;
use network::{HttpRequest, HttpTransport, RequestPolicy, Url};

use super::navigation;
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
    pub fn fetch(&self, request: FetchRequest) -> FetchPayload {
        match request {
            FetchRequest::Document(document) => {
                let loaded = navigation::navigate(&document, &self.transport, &self.policy)
                    .map(|tree| LoadedDocument::new(tree, document.url().clone()));
                FetchPayload::Document(loaded.map_err(|error| describe(&error)))
            }
            FetchRequest::Subresource(SubresourceRequest::Stylesheet(stylesheet)) => {
                self.fetch_stylesheet(&stylesheet)
            }
            FetchRequest::Subresource(SubresourceRequest::Image(image)) => {
                let frame = self.fetch_image(image.url());
                FetchPayload::Image(image.id(), frame.map_err(|error| describe(&error)))
            }
        }
    }

    fn fetch_stylesheet(&self, stylesheet: &StylesheetRequest) -> FetchPayload {
        let text = self
            .fetch_text(stylesheet.url())
            .map_err(|error| describe(&error));
        FetchPayload::Stylesheet(stylesheet.media().clone(), text)
    }

    fn fetch_text(&self, url: &Url) -> Result<String, AlloyError> {
        let response = self.transport.execute(&HttpRequest::get(url.clone()))?;
        navigation::ensure_success(url, response.status())?;
        let body = response.body().as_str().unwrap_or_default().to_owned();
        tracing::debug!(
            %url,
            status = response.status().code(),
            bytes = body.len(),
            "stylesheet fetched"
        );
        Ok(body)
    }

    fn fetch_image(&self, url: &Url) -> Result<Framebuffer, AlloyError> {
        let response = self.transport.execute(&HttpRequest::get(url.clone()))?;
        navigation::ensure_success(url, response.status())?;
        self.decoder.decode(response.body().as_bytes())
    }
}

/// The error page shows the message; the cause stays in this crate's error.
fn describe(error: &AlloyError) -> FetchFailure {
    FetchFailure::new(error.to_string())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use network::{AllowAllPolicy, Body, HeaderMap, HttpResponse, MockTransport, StatusCode, Url};

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

        let result = fetcher.fetch_text(&sheet_url);

        assert!(
            matches!(result, Err(AlloyError::HttpStatus { status: 404, .. })),
            "a 404 error page must not reach the CSS parser as a stylesheet"
        );
    }
}

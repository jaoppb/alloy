//! Navigation (v0.5 Phase I4): `DocumentRequest -> RequestPolicy ->
//! HttpTransport -> core/html -> DomTree`.
//!
//! Policy runs before mechanism (`PRD-009` §3.3): `decide` is consulted
//! before [`HttpTransport::execute`] ever opens a socket, so a denial costs
//! no connection. This is infrastructure (#114): it is the one place that
//! calls `execute` for a document, run by a fetch worker.

use browser::DocumentRequest;
use dom::DomTree;
use network::{
    HttpRequest, HttpTransport, NetworkError, PolicyVerdict, RequestPolicy, StatusCode, Url,
};

use crate::error::AlloyError;

/// Fetches the document `request` asks for through `policy` then `transport`,
/// and parses the response body as HTML.
///
/// A response that "succeeds" at the transport layer but carries a non-`2xx`
/// status, an empty body, or bytes that are not UTF-8 text is a typed `Err`
/// here, not an empty document: the event loop turns any `Err` into the
/// visible error card, where a silently-parsed `""` would just be a blank
/// window (v0.5 Phase I4).
pub fn navigate<T: HttpTransport, P: RequestPolicy>(
    document: &DocumentRequest,
    transport: &T,
    policy: &P,
) -> Result<DomTree, AlloyError> {
    let url = document.url();
    let requested =
        HttpRequest::new(document.method(), url.clone()).with_body(document.body().clone());
    let request = match policy.decide(&requested)? {
        PolicyVerdict::Allow => requested,
        PolicyVerdict::Rewrite(rewritten) => rewritten,
        PolicyVerdict::Deny { reason } => {
            return Err(AlloyError::from(NetworkError::policy_denied(reason)));
        }
        _ => {
            return Err(AlloyError::from(NetworkError::policy_denied(
                "unrecognised policy verdict",
            )));
        }
    };
    let response = transport.execute(&request)?;
    ensure_success(url, response.status())?;
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
    Ok(crate::application::html_parse::parse_html(text)?)
}

/// Rejects a non-`2xx` status with a typed [`AlloyError::HttpStatus`].
///
/// A fetch that redirected to an error page still "succeeds" at the transport
/// layer; both the main document ([`navigate`]) and the subresource fetches
/// (`http_fetch`) go through here so an HTML 404/500 body never reaches a
/// parser as if it were the resource.
pub(crate) fn ensure_success(url: &Url, status: StatusCode) -> Result<(), AlloyError> {
    if status.is_success() {
        return Ok(());
    }
    Err(AlloyError::HttpStatus {
        url: url.to_string(),
        status: status.code(),
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use browser::DocumentRequest;
    use network::{
        AllowAllPolicy, Body, HeaderMap, HttpRequest, HttpResponse, Method, MockTransport,
        NetworkError, PolicyVerdict, RequestPolicy, StatusCode, Url,
    };

    use super::navigate;
    use crate::error::AlloyError;

    fn navigate_to(url: &Url, response: HttpResponse) -> Result<dom::DomTree, AlloyError> {
        let transport = MockTransport::new().with_response(url.clone(), response);
        navigate(
            &DocumentRequest::get(url.clone()),
            &transport,
            &AllowAllPolicy,
        )
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

        let result = navigate(&document, &MockTransport::new(), &QuotingPolicy);

        let message = result.unwrap_err().to_string();
        assert!(
            message.contains("POST carrying 3 body bytes"),
            "got {message}"
        );
    }
}

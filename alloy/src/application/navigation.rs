//! Navigation (v0.5 Phase I4): `Url -> RequestPolicy -> HttpTransport ->
//! core/html -> DomTree`.
//!
//! Policy runs before mechanism (`PRD-009` §3.3): `decide` is consulted
//! before [`HttpTransport::execute`] ever opens a socket, so a denial costs
//! no connection.

use dom::DomTree;
use network::{HttpRequest, HttpTransport, NetworkError, PolicyVerdict, RequestPolicy, Url};

use crate::error::AlloyError;

/// Fetches `url` through `policy` then `transport`, and parses the response
/// body as HTML.
pub fn navigate(
    url: &Url,
    transport: &dyn HttpTransport,
    policy: &dyn RequestPolicy,
) -> Result<DomTree, AlloyError> {
    let requested = HttpRequest::get(url.clone());
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
    let body = response.body().as_str().unwrap_or_default();
    Ok(html::parse(body)?)
}

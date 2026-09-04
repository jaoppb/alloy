//! Following `Location`, and stopping.
//!
//! Two independent guards, because they fail differently: a **hop limit** stops
//! a chain that keeps going somewhere new, and a **visited set** stops one that
//! keeps going back. Either produces a typed
//! [`NetworkError::Redirect`](crate::domain::error::NetworkError::Redirect) —
//! never an infinite loop, which is the whole point of the rule.
//!
//! Pure value logic over [`Url`] and the message aggregates: no socket, so this
//! module is not gated behind `real-transport`.

use std::collections::BTreeSet;

use crate::domain::defect::RedirectDefect;
use crate::domain::error::NetworkError;
use crate::domain::header_map::HeaderName;
use crate::domain::method::Method;
use crate::domain::request::HttpRequest;
use crate::domain::response::HttpResponse;
use crate::domain::url::Url;

/// How many hops a redirect chain may take.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RedirectLimit(usize);

impl RedirectLimit {
    /// Twenty, the ceiling every mainstream browser uses.
    pub const DEFAULT: Self = Self(20);

    /// A limit of `hops` hops.
    #[must_use]
    pub const fn of_hops(hops: usize) -> Self {
        Self(hops)
    }

    /// The limit in hops.
    #[must_use]
    pub const fn hops(self) -> usize {
        self.0
    }
}

impl Default for RedirectLimit {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// The record of where one exchange has already been sent.
#[derive(Clone, Debug)]
pub struct RedirectTrail {
    visited: BTreeSet<Url>,
    limit: RedirectLimit,
    hops: usize,
}

impl RedirectTrail {
    /// An empty trail under `limit`.
    #[must_use]
    pub const fn new(limit: RedirectLimit) -> Self {
        Self {
            visited: BTreeSet::new(),
            limit,
            hops: 0,
        }
    }

    /// Record that `url` is about to be requested.
    ///
    /// # Errors
    ///
    /// [`NetworkError::Redirect`] with [`RedirectDefect::Cycle`] when the chain
    /// revisits a URL, or [`RedirectDefect::LimitExceeded`] when it outlives
    /// its hop budget.
    pub fn record(&mut self, url: &Url) -> Result<(), NetworkError> {
        if !self.visited.insert(url.clone()) {
            return Err(NetworkError::redirect(RedirectDefect::Cycle));
        }
        if self.hops > self.limit.hops() {
            return Err(NetworkError::redirect(RedirectDefect::LimitExceeded));
        }
        self.hops = self.hops.saturating_add(1);
        Ok(())
    }

    /// How many URLs the chain has been sent to.
    #[must_use]
    pub const fn hops(&self) -> usize {
        self.hops
    }
}

/// The request that follows `response`, or `None` when the chain ends here.
///
/// # Errors
///
/// [`NetworkError::Redirect`] when a followable `3xx` carries no `Location`, or
/// one that does not resolve against the request's URL.
pub fn next_request(
    previous: &HttpRequest,
    response: &HttpResponse,
) -> Result<Option<HttpRequest>, NetworkError> {
    if !response.status().is_followable_redirect() {
        return Ok(None);
    }
    let location = response
        .headers()
        .text(&HeaderName::location())
        .ok_or_else(|| NetworkError::redirect(RedirectDefect::LocationMissing))?;
    let target = previous
        .url()
        .join(location)
        .map_err(|_| NetworkError::redirect(RedirectDefect::LocationUnresolvable))?;
    Ok(Some(rewrite_for(previous, response, target)))
}

/// RFC 9110 §15.4: `301`/`302`/`303` rewrite an unsafe method to `GET` and drop
/// the payload; `307`/`308` preserve both. Credentials never survive a hop to
/// another origin.
fn rewrite_for(previous: &HttpRequest, response: &HttpResponse, target: Url) -> HttpRequest {
    let crossed_origin = !previous.url().has_same_origin_as(&target);
    let mut next = previous.clone().with_url(target);
    if response.status().rewrites_method() && previous.method().is_rewritten_on_redirect() {
        next = next.rewritten_to(Method::Get);
    }
    if crossed_origin {
        next = next
            .without_header(&HeaderName::authorization())
            .without_header(&HeaderName::cookie());
    }
    next
}

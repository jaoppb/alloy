//! [`Url`] — a validated, normalised absolute HTTP URL.
//!
//! Parsing is deliberately narrow: `http` and `https` only, no userinfo, no
//! fragment kept, dot segments removed, scheme and host lowercased, the port
//! always resolved to a concrete number. Two spellings of the same resource
//! therefore compare equal — which is what makes
//! [`RedirectTrail`](crate::infrastructure::redirect::RedirectTrail)'s cycle
//! detection sound rather than approximate.
//!
//! No `&text[a..b]` anywhere in this file: the strict lint set denies
//! `string_slice` and `indexing_slicing`, so every split is `split_once`,
//! `strip_prefix`, `find` + `get`, or an iterator.

use core::fmt;

use crate::domain::authority::{Authority, Host, Port};
use crate::domain::defect::UrlDefect;
use crate::domain::error::NetworkError;
use crate::domain::phase::ProtocolPhase;
use crate::domain::scheme::Scheme;
use crate::domain::target::{Path, Query, RequestTarget};

/// An absolute `http` or `https` URL.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Url {
    scheme: Scheme,
    authority: Authority,
    target: RequestTarget,
}

impl Url {
    /// Parse an absolute URL.
    ///
    /// The fragment is dropped — it never travels on the wire.
    ///
    /// # Errors
    ///
    /// [`NetworkError::InvalidUrl`] in the [`ProtocolPhase::Dns`] phase, since
    /// this is the step that decides *where* the exchange would go. A caller
    /// resolving a redirect target re-labels it with
    /// [`NetworkError::in_phase`].
    pub fn parse(raw: &str) -> Result<Self, NetworkError> {
        Self::parse_parts(raw)
            .map_err(|defect| NetworkError::invalid_url(ProtocolPhase::Dns, defect))
    }

    fn parse_parts(raw: &str) -> Result<Self, UrlDefect> {
        let (scheme_text, rest) = raw.split_once("://").ok_or(UrlDefect::MissingScheme)?;
        let scheme = Scheme::parse(scheme_text)?;
        let without_fragment = strip_fragment(rest);
        let (authority_text, target_text) = split_authority(without_fragment);
        let authority = parse_authority(authority_text, scheme)?;
        let target = parse_target(target_text)?;
        Ok(Self {
            scheme,
            authority,
            target,
        })
    }

    /// Build a URL from already-validated parts.
    #[must_use]
    pub const fn from_parts(scheme: Scheme, authority: Authority, target: RequestTarget) -> Self {
        Self {
            scheme,
            authority,
            target,
        }
    }

    /// The scheme.
    #[must_use]
    pub const fn scheme(&self) -> Scheme {
        self.scheme
    }

    /// The host and port.
    #[must_use]
    pub const fn authority(&self) -> &Authority {
        &self.authority
    }

    /// The host alone.
    #[must_use]
    pub const fn host(&self) -> &Host {
        self.authority.host()
    }

    /// The port, always concrete — the scheme default when the text had none.
    #[must_use]
    pub const fn port(&self) -> Port {
        self.authority.port()
    }

    /// The origin-form target that goes on the request line.
    #[must_use]
    pub const fn target(&self) -> &RequestTarget {
        &self.target
    }

    /// The path alone.
    #[must_use]
    pub const fn path(&self) -> &Path {
        self.target.path()
    }

    /// The query alone, if there was one.
    #[must_use]
    pub const fn query(&self) -> Option<&Query> {
        self.target.query()
    }

    /// Whether two URLs share scheme, host and port — the test that decides
    /// whether `Authorization` survives a redirect.
    #[must_use]
    pub fn has_same_origin_as(&self, other: &Self) -> bool {
        self.scheme == other.scheme && self.authority == other.authority
    }

    /// The canonical text of this URL.
    #[must_use]
    pub fn to_text(&self) -> String {
        format!(
            "{}://{}{}",
            self.scheme,
            self.authority.to_header_text(self.scheme),
            self.target.to_text()
        )
    }

    /// Resolve a reference — a `Location` header, an `href` — against this URL
    /// as its base (RFC 3986 §5.2).
    ///
    /// # Errors
    ///
    /// [`NetworkError::InvalidUrl`] in the [`ProtocolPhase::Redirect`] phase
    /// when the reference is empty or does not resolve to a usable absolute
    /// URL.
    pub fn join(&self, reference: &str) -> Result<Self, NetworkError> {
        self.join_parts(reference)
            .map_err(|defect| NetworkError::invalid_url(ProtocolPhase::Redirect, defect))
    }

    fn join_parts(&self, reference: &str) -> Result<Self, UrlDefect> {
        let trimmed = reference.trim();
        let without_fragment = strip_fragment(trimmed);
        if without_fragment.is_empty() {
            return Ok(self.clone());
        }
        if is_absolute(without_fragment) {
            return Self::parse_parts(without_fragment);
        }
        if let Some(network_path) = without_fragment.strip_prefix("//") {
            let scheme = self.scheme.as_str();
            return Self::parse_parts(&format!("{scheme}://{network_path}"));
        }
        self.merge(without_fragment)
    }

    fn merge(&self, reference: &str) -> Result<Self, UrlDefect> {
        if let Some(query_text) = reference.strip_prefix('?') {
            let query = Query::new(query_text)?;
            let target = RequestTarget::new(self.path().clone(), Some(query));
            return Ok(Self::from_parts(
                self.scheme,
                self.authority.clone(),
                target,
            ));
        }
        let merged = match reference.strip_prefix('/') {
            Some(_) => reference.to_owned(),
            None => format!("{}{reference}", self.path().directory()),
        };
        let target = parse_target(&merged)?;
        Ok(Self::from_parts(
            self.scheme,
            self.authority.clone(),
            target,
        ))
    }
}

impl fmt::Display for Url {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.to_text())
    }
}

/// A reference is absolute when it carries `scheme://` before any delimiter.
fn is_absolute(reference: &str) -> bool {
    let Some((prefix, _)) = reference.split_once("://") else {
        return false;
    };
    !prefix.is_empty()
        && prefix
            .chars()
            .all(|character| character.is_ascii_alphanumeric())
}

fn strip_fragment(text: &str) -> &str {
    match text.split_once('#') {
        Some((before, _)) => before,
        None => text,
    }
}

/// Split `host[:port][/path][?query]` into the authority and everything after
/// it. The delimiter is whichever of `/` or `?` comes first.
fn split_authority(rest: &str) -> (&str, &str) {
    rest.find(['/', '?']).map_or((rest, ""), |index| {
        (
            rest.get(..index).unwrap_or_default(),
            rest.get(index..).unwrap_or_default(),
        )
    })
}

fn parse_authority(text: &str, scheme: Scheme) -> Result<Authority, UrlDefect> {
    if text.contains('@') {
        return Err(UrlDefect::EmbeddedCredentials);
    }
    let (host_text, port_text) = split_host_and_port(text)?;
    let host = Host::new(host_text)?;
    let port = match port_text {
        Some(digits) => Port::parse(digits)?,
        None => Port::default_for(scheme),
    };
    Ok(Authority::new(host, port))
}

/// IPv6 literals are bracketed, so the port separator is the `:` *after* the
/// closing bracket, never one inside it.
fn split_host_and_port(text: &str) -> Result<(&str, Option<&str>), UrlDefect> {
    if text.starts_with('[') {
        let closing = text.find(']').ok_or(UrlDefect::MalformedHost)?;
        let host = text
            .get(..closing.saturating_add(1))
            .ok_or(UrlDefect::MalformedHost)?;
        let remainder = text.get(closing.saturating_add(1)..).unwrap_or_default();
        return match remainder.strip_prefix(':') {
            Some(digits) => Ok((host, Some(digits))),
            None if remainder.is_empty() => Ok((host, None)),
            None => Err(UrlDefect::MalformedHost),
        };
    }
    match text.rsplit_once(':') {
        Some((host, digits)) => Ok((host, Some(digits))),
        None => Ok((text, None)),
    }
}

fn parse_target(text: &str) -> Result<RequestTarget, UrlDefect> {
    if text.is_empty() {
        return Ok(RequestTarget::new(Path::root(), None));
    }
    if let Some(query_text) = text.strip_prefix('?') {
        let query = Query::new(query_text)?;
        return Ok(RequestTarget::new(Path::root(), Some(query)));
    }
    match text.split_once('?') {
        Some((path_text, query_text)) => Ok(RequestTarget::new(
            Path::new(path_text)?,
            Some(Query::new(query_text)?),
        )),
        None => Ok(RequestTarget::new(Path::new(text)?, None)),
    }
}

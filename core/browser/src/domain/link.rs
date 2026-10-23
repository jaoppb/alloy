//! The link policy: where a clicked `href` leads.

use network::{NetworkError, Url};

/// What following a link does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LinkDestination {
    /// `#fragment` — stays on the page; scrolling to it is not built yet.
    InPageAnchor,
    /// Another document.
    Navigate(Url),
}

impl LinkDestination {
    /// Where `href`, as written in a document based at `base`, leads.
    ///
    /// # Errors
    ///
    /// [`NetworkError`] when `href` does not resolve to a usable absolute URL.
    pub fn resolve(base: &Url, href: &str) -> Result<Self, NetworkError> {
        if href.starts_with('#') {
            return Ok(Self::InPageAnchor);
        }
        base.join(href).map(Self::Navigate)
    }
}

#[cfg(test)]
mod tests {
    use network::Url;

    use super::LinkDestination;

    fn base() -> Url {
        Url::parse("https://cdn.example/app/index.html").unwrap()
    }

    #[test]
    fn a_fragment_only_href_stays_on_the_page() {
        assert_eq!(
            LinkDestination::resolve(&base(), "#top").unwrap(),
            LinkDestination::InPageAnchor
        );
    }

    #[test]
    fn a_relative_href_resolves_against_the_base() {
        let expected = Url::parse("https://cdn.example/app/docs.html").unwrap();
        assert_eq!(
            LinkDestination::resolve(&base(), "docs.html").unwrap(),
            LinkDestination::Navigate(expected)
        );
    }

    #[test]
    fn an_unresolvable_href_is_an_error_not_a_navigation() {
        assert!(LinkDestination::resolve(&base(), "http://").is_err());
    }
}

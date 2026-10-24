//! [`OmniboxClassifier`], [`OmniboxDestination`] and [`SearchTemplate`].
//!
//! Domain service for classifying raw address bar input into either a direct
//! navigation [`Url`] or a search engine query [`Url`].

use core::fmt;

use network::{NetworkError, Url};

/// The target destination resolved from omnibox input.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum OmniboxDestination {
    /// A direct URL navigation destination.
    DirectUrl(Url),
    /// A search engine query URL destination.
    SearchQuery(Url),
}

impl OmniboxDestination {
    /// The resolved URL.
    #[must_use]
    pub const fn url(&self) -> &Url {
        match self {
            Self::DirectUrl(url) | Self::SearchQuery(url) => url,
        }
    }

    /// Extract the resolved URL, consuming `self`.
    #[must_use]
    pub fn into_url(self) -> Url {
        match self {
            Self::DirectUrl(url) | Self::SearchQuery(url) => url,
        }
    }

    /// Whether this destination is a direct URL.
    #[must_use]
    pub const fn is_direct_url(&self) -> bool {
        matches!(self, Self::DirectUrl(_))
    }

    /// Whether this destination is a search query.
    #[must_use]
    pub const fn is_search_query(&self) -> bool {
        matches!(self, Self::SearchQuery(_))
    }
}

impl fmt::Display for OmniboxDestination {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.url().to_text().as_str())
    }
}

/// A search engine URL template.
///
/// Supports `{searchTerms}` or `{}` as placeholder parameters, or appending
/// to a query URL.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SearchTemplate(String);

impl SearchTemplate {
    /// Default `DuckDuckGo` template URL.
    pub const DUCK_DUCK_GO: &'static str = "https://duckduckgo.com/?q={searchTerms}";

    /// The default `DuckDuckGo` search engine template.
    #[must_use]
    pub fn duck_duck_go() -> Self {
        Self(Self::DUCK_DUCK_GO.to_owned())
    }

    /// Create and validate a search template URL.
    ///
    /// # Errors
    ///
    /// Returns [`NetworkError::InvalidUrl`] if the template cannot produce a valid URL.
    pub fn new(template: &str) -> Result<Self, NetworkError> {
        let candidate = Self(template.to_owned());
        candidate.render("test")?;
        Ok(candidate)
    }

    /// Render the template with the provided encoded search query.
    ///
    /// # Errors
    ///
    /// Returns [`NetworkError::InvalidUrl`] if the resulting URL is invalid.
    pub fn render(&self, encoded_query: &str) -> Result<Url, NetworkError> {
        if self.0.contains("{searchTerms}") {
            return Url::parse(&self.0.replace("{searchTerms}", encoded_query));
        }
        if self.0.contains("{}") {
            return Url::parse(&self.0.replace("{}", encoded_query));
        }
        Url::parse(&format!("{}{encoded_query}", self.0))
    }

    /// The template pattern as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for SearchTemplate {
    fn default() -> Self {
        Self::duck_duck_go()
    }
}

impl fmt::Display for SearchTemplate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl TryFrom<&str> for SearchTemplate {
    type Error = NetworkError;

    fn try_from(raw: &str) -> Result<Self, Self::Error> {
        Self::new(raw)
    }
}

/// Disambiguates omnibox input between direct URLs and search engine queries.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OmniboxClassifier {
    search_template: SearchTemplate,
}

impl OmniboxClassifier {
    /// Create a classifier with a custom search template.
    #[must_use]
    pub const fn new(search_template: SearchTemplate) -> Self {
        Self { search_template }
    }

    /// The active search engine template.
    #[must_use]
    pub const fn search_template(&self) -> &SearchTemplate {
        &self.search_template
    }

    /// Update the search engine template at runtime.
    pub fn set_search_template(&mut self, search_template: SearchTemplate) {
        self.search_template = search_template;
    }

    /// Classify raw user input into an [`OmniboxDestination`].
    ///
    /// # Errors
    ///
    /// Returns [`NetworkError`] if direct URL parsing fails with an explicit scheme,
    /// or if the search engine URL construction fails.
    pub fn classify(&self, input: &str) -> Result<OmniboxDestination, NetworkError> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return self.build_search_destination("");
        }
        if has_explicit_scheme(trimmed) {
            return Url::parse(trimmed).map(OmniboxDestination::DirectUrl);
        }
        let host_candidate = extract_host_candidate(trimmed);
        let host_without_port = strip_candidate_port(host_candidate);
        if is_local_address(host_without_port) {
            return Url::parse(&format!("http://{trimmed}")).map(OmniboxDestination::DirectUrl);
        }
        if trimmed.chars().any(char::is_whitespace) {
            return self.build_search_destination(trimmed);
        }
        if !has_valid_hostname_dots(host_without_port) {
            return self.build_search_destination(trimmed);
        }
        let normalized = format!("https://{trimmed}");
        if let Ok(url) = Url::parse(&normalized) {
            return Ok(OmniboxDestination::DirectUrl(url));
        }
        self.build_search_destination(trimmed)
    }

    fn build_search_destination(&self, query: &str) -> Result<OmniboxDestination, NetworkError> {
        let encoded_query = percent_encode_query_component(query);
        let url = self.search_template.render(&encoded_query)?;
        Ok(OmniboxDestination::SearchQuery(url))
    }
}

impl Default for OmniboxClassifier {
    fn default() -> Self {
        Self::new(SearchTemplate::default())
    }
}

fn has_explicit_scheme(input: &str) -> bool {
    let Some((scheme, _)) = input.split_once("://") else {
        return false;
    };
    !scheme.is_empty()
        && scheme
            .chars()
            .all(|character| character.is_ascii_alphanumeric())
}

fn extract_host_candidate(input: &str) -> &str {
    let before_path = input.split_once('/').map_or(input, |(before, _)| before);
    before_path
        .split_once('?')
        .map_or(before_path, |(before, _)| before)
}

fn strip_candidate_port(candidate: &str) -> &str {
    if candidate.starts_with('[') {
        let Some(closing_index) = candidate.find(']') else {
            return candidate;
        };
        return candidate.get(..=closing_index).unwrap_or(candidate);
    }
    candidate
        .rsplit_once(':')
        .map_or(candidate, |(host, _)| host)
}

fn is_local_address(candidate_host: &str) -> bool {
    candidate_host.eq_ignore_ascii_case("localhost")
        || candidate_host == "127.0.0.1"
        || candidate_host == "[::1]"
}

fn has_valid_hostname_dots(host: &str) -> bool {
    if !host.contains('.') {
        return false;
    }
    if host.starts_with('.') || host.ends_with('.') {
        return false;
    }
    !host.contains("..")
}

fn percent_encode_query_component(raw: &str) -> String {
    let mut encoded = String::with_capacity(raw.len());
    for byte in raw.as_bytes().iter().copied() {
        match byte {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'*' => {
                encoded.push(char::from(byte));
            }
            b' ' => {
                encoded.push('+');
            }
            _ => {
                push_hex_byte(&mut encoded, byte);
            }
        }
    }
    encoded
}

fn push_hex_byte(output: &mut String, byte: u8) {
    const HEX_UPPERCASE: &[u8; 16] = b"0123456789ABCDEF";
    output.push('%');
    let high = usize::from(byte >> 4);
    let low = usize::from(byte & 0x0F);
    if let Some(&high_char) = HEX_UPPERCASE.get(high) {
        output.push(char::from(high_char));
    }
    if let Some(&low_char) = HEX_UPPERCASE.get(low) {
        output.push(char::from(low_char));
    }
}

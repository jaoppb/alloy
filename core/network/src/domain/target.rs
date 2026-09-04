//! [`Path`], [`Query`] and [`RequestTarget`] — the "what" half of a URL.
//!
//! The request target is what goes on the request line after the method. It is
//! built once, validated once, and never re-assembled from loose strings at the
//! call site.

use core::fmt;

use crate::domain::defect::UrlDefect;

/// A validated absolute path: begins with `/`, carries no control character,
/// no raw space and no `?` or `#` (those delimit the query and the fragment).
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Path(String);

impl Path {
    /// The path a URL gets when its authority is followed by nothing.
    #[must_use]
    pub fn root() -> Self {
        Self(String::from("/"))
    }

    /// Validate a path, normalising `.` and `..` segments away (RFC 3986
    /// §5.2.4) so two spellings of the same resource compare equal — which is
    /// what makes redirect-cycle detection sound.
    ///
    /// # Errors
    ///
    /// [`UrlDefect::MalformedPath`] when a character is not permitted.
    pub fn new(raw: &str) -> Result<Self, UrlDefect> {
        if raw.chars().any(is_forbidden_in_target) {
            return Err(UrlDefect::MalformedPath);
        }
        if !raw.starts_with('/') {
            return Err(UrlDefect::MalformedPath);
        }
        Ok(Self(remove_dot_segments(raw)))
    }

    /// The normalised path text, always beginning with `/`.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Everything up to and including the last `/` — the base a relative
    /// reference is merged onto (RFC 3986 §5.2.3).
    #[must_use]
    pub fn directory(&self) -> &str {
        self.0.rfind('/').map_or("/", |index| {
            self.0.get(..index.saturating_add(1)).unwrap_or("/")
        })
    }
}

impl fmt::Display for Path {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// A validated query string — the text after `?`, without the `?`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Query(String);

impl Query {
    /// Validate a query string.
    ///
    /// # Errors
    ///
    /// [`UrlDefect::MalformedPath`] when a character is not permitted.
    pub fn new(raw: &str) -> Result<Self, UrlDefect> {
        if raw.chars().any(is_forbidden_in_target) {
            return Err(UrlDefect::MalformedPath);
        }
        Ok(Self(raw.to_owned()))
    }

    /// The query text, without the leading `?`.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Query {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// The origin-form request target: `/path` or `/path?query`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RequestTarget {
    path: Path,
    query: Option<Query>,
}

impl RequestTarget {
    /// Pair a path with an optional query.
    #[must_use]
    pub const fn new(path: Path, query: Option<Query>) -> Self {
        Self { path, query }
    }

    /// The path half.
    #[must_use]
    pub const fn path(&self) -> &Path {
        &self.path
    }

    /// The query half, if the URL had one.
    #[must_use]
    pub const fn query(&self) -> Option<&Query> {
        self.query.as_ref()
    }

    /// The origin-form text that goes on the request line.
    #[must_use]
    pub fn to_text(&self) -> String {
        self.query.as_ref().map_or_else(
            || self.path.as_str().to_owned(),
            |query| format!("{}?{query}", self.path),
        )
    }
}

impl fmt::Display for RequestTarget {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.to_text())
    }
}

/// A raw space, a control character or a fragment marker has no business in a
/// path or query: each of them lets crafted text break out onto the request
/// line.
const fn is_forbidden_in_target(character: char) -> bool {
    !character.is_ascii()
        || character.is_ascii_control()
        || character.is_ascii_whitespace()
        || matches!(
            character,
            '#' | '\\' | '"' | '<' | '>' | '^' | '{' | '}' | '|'
        )
}

/// RFC 3986 §5.2.4, written so a `..` can never climb above the root.
fn remove_dot_segments(path: &str) -> String {
    let segments: Vec<&str> = path.split('/').collect();
    let last_index = segments.len().saturating_sub(1);
    let mut kept: Vec<&str> = Vec::with_capacity(segments.len());
    for (index, segment) in segments.iter().enumerate() {
        push_segment(&mut kept, segment, index == last_index);
    }
    let joined = kept.join("/");
    if joined.is_empty() {
        return String::from("/");
    }
    joined
}

fn push_segment<'a>(kept: &mut Vec<&'a str>, segment: &'a str, is_last: bool) {
    if segment == "." {
        keep_trailing_slash(kept, is_last);
        return;
    }
    if segment == ".." {
        if kept.len() > 1 {
            kept.pop();
        }
        keep_trailing_slash(kept, is_last);
        return;
    }
    kept.push(segment);
}

fn keep_trailing_slash(kept: &mut Vec<&str>, is_last: bool) {
    if is_last {
        kept.push("");
    }
}

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

/// One decoded `key=value` pair of a query string, in the order it appeared.
///
/// Decoding follows the WHATWG URL Standard's `application/x-www-form-urlencoded`
/// parser (§5, "urlencoded parsing"), the algorithm every browser runs on a
/// query string: a key with no `=` decodes to an empty-string value (`?flag`
/// and `?flag=` are indistinguishable), never `Option::None`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct QueryParam {
    key: String,
    value: String,
}

impl QueryParam {
    /// The decoded key.
    #[must_use]
    pub fn key(&self) -> &str {
        &self.key
    }

    /// The decoded value — empty when the pair carried none (`?flag`).
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }
}

/// A validated query string — the text after `?`, without the `?`.
///
/// Parsed into its ordered `key=value` pairs the way the WHATWG URL Standard's
/// `application/x-www-form-urlencoded` parser does, rather than kept as one
/// string a caller would have to re-split and percent-decode itself.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Query {
    raw: String,
    params: Vec<QueryParam>,
}

impl Query {
    /// Validate and parse a query string.
    ///
    /// # Errors
    ///
    /// [`UrlDefect::MalformedPath`] when the raw text carries a character
    /// that has no business on the request line (a control character, a raw
    /// space, an embedded fragment marker). Once past that wire-safety check,
    /// decoding the pairs themselves can never fail — the WHATWG algorithm is
    /// total, the same way a browser never rejects a query string as
    /// unparsable.
    pub fn new(raw: &str) -> Result<Self, UrlDefect> {
        if raw.chars().any(is_forbidden_in_target) {
            return Err(UrlDefect::MalformedPath);
        }
        Ok(Self {
            raw: raw.to_owned(),
            params: parse_params(raw),
        })
    }

    /// The query text, without the leading `?`, exactly as it travels on the
    /// wire.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.raw
    }

    /// Every pair, in the order it appeared.
    pub fn params(&self) -> impl Iterator<Item = &QueryParam> + '_ {
        self.params.iter()
    }

    /// The decoded value of the first pair named `key`, if any.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&str> {
        self.params
            .iter()
            .find(|param| param.key() == key)
            .map(QueryParam::value)
    }
}

impl fmt::Display for Query {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.raw)
    }
}

/// WHATWG URL §5, "urlencoded parsing": split on `&`, drop empty sequences,
/// keep every other one — including a lone bare key — as a pair.
fn parse_params(raw: &str) -> Vec<QueryParam> {
    raw.split('&')
        .filter(|pair| !pair.is_empty())
        .map(parse_param)
        .collect()
}

/// A pair with no `=` names an empty-string value, never "no value" — the
/// same rule `URLSearchParams` uses, so `?flag` round-trips the same way in
/// this engine as it would in a browser.
fn parse_param(pair: &str) -> QueryParam {
    let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
    QueryParam {
        key: decode_form_component(name),
        value: decode_form_component(value),
    }
}

/// WHATWG URL §5: replace `+` with space, percent-decode leniently, then
/// UTF-8 decode with the replacement character standing in for anything
/// that isn't valid UTF-8. Unlike RFC 3986 percent-decoding this can never
/// fail — a malformed escape is not a parse error, it is data, exactly as a
/// browser treats it.
fn decode_form_component(text: &str) -> String {
    let space_replaced = text.replace('+', " ");
    let decoded = percent_decode_lenient(space_replaced.as_bytes());
    String::from_utf8_lossy(&decoded).into_owned()
}

fn percent_decode_lenient(bytes: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(bytes.len());
    let mut cursor = bytes.iter().copied();
    while let Some(byte) = cursor.next() {
        push_decoded_byte(byte, &mut cursor, &mut output);
    }
    output
}

/// Consume one input byte from `cursor`, and — for a `%` that opens a valid
/// two-digit hex escape — the two bytes it decodes. A `%` that does not open
/// a valid escape passes through untouched and consumes nothing extra, so a
/// stray `%` in the middle of an otherwise well-formed query never loses
/// data.
fn push_decoded_byte(
    byte: u8,
    cursor: &mut (impl Iterator<Item = u8> + Clone),
    output: &mut Vec<u8>,
) {
    if byte != b'%' {
        output.push(byte);
        return;
    }
    let mut lookahead = cursor.clone();
    let Some(high) = lookahead.next() else {
        output.push(byte);
        return;
    };
    let Some(low) = lookahead.next() else {
        output.push(byte);
        return;
    };
    let Some(escaped) = hex_pair(high, low) else {
        output.push(byte);
        return;
    };
    output.push(escaped);
    cursor.next();
    cursor.next();
}

fn hex_pair(high: u8, low: u8) -> Option<u8> {
    let high = hex_digit(high)?;
    let low = hex_digit(low)?;
    Some((high << 4) | low)
}

const fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte.wrapping_sub(b'0')),
        b'a'..=b'f' => Some(byte.wrapping_sub(b'a').wrapping_add(10)),
        b'A'..=b'F' => Some(byte.wrapping_sub(b'A').wrapping_add(10)),
        _ => None,
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

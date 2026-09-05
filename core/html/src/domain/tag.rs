//! Vocabulary of HTML tags and their structural semantics.

use core::fmt;
use core::str::FromStr;

use crate::domain::error::HtmlError;

macro_rules! define_tags {
    (
        void: [ $( $void_var:ident => $void_str:literal ),* $(,)? ],
        normal: [ $( $norm_var:ident => $norm_str:literal ),* $(,)? ]
    ) => {
        /// A strongly-typed HTML tag name representing standard HTML5 W3C elements
        /// or a validated custom element tag.
        #[derive(Clone, Debug, PartialEq, Eq, Hash)]
        pub enum TagName {
            $(
                #[doc = concat!("The `<", $void_str, ">` void element tag.")]
                $void_var,
            )*
            $(
                #[doc = concat!("The `<", $norm_str, ">` element tag.")]
                $norm_var,
            )*
            /// An autonomous custom element tag (e.g. `<my-component>`).
            Custom(String),
        }

        impl TagName {
            /// The tag name as a lowercase string slice.
            #[must_use]
            pub const fn as_str(&self) -> &str {
                match self {
                    $( Self::$void_var => $void_str, )*
                    $( Self::$norm_var => $norm_str, )*
                    Self::Custom(custom) => custom.as_str(),
                }
            }

            /// Whether this tag is a W3C HTML void element (which cannot have children
            /// and never emits a closing tag during serialization).
            #[must_use]
            pub const fn is_void(&self) -> bool {
                matches!(self, $( Self::$void_var )|*)
            }

            fn from_standard_name(name: &str) -> Option<Self> {
                match name {
                    $( $void_str => Some(Self::$void_var), )*
                    $( $norm_str => Some(Self::$norm_var), )*
                    _ => None,
                }
            }
        }
    };
}

define_tags! {
    void: [
        Area => "area",
        Base => "base",
        Br => "br",
        Col => "col",
        Embed => "embed",
        Hr => "hr",
        Img => "img",
        Input => "input",
        Link => "link",
        Meta => "meta",
        Param => "param",
        Source => "source",
        Track => "track",
        Wbr => "wbr",
    ],
    normal: [
        // Document / metadata / root
        Html => "html",
        Head => "head",
        Title => "title",
        Style => "style",

        // Sectioning
        Body => "body",
        Article => "article",
        Section => "section",
        Nav => "nav",
        Aside => "aside",
        H1 => "h1",
        H2 => "h2",
        H3 => "h3",
        H4 => "h4",
        H5 => "h5",
        H6 => "h6",
        Header => "header",
        Footer => "footer",
        Address => "address",
        Main => "main",
        Hgroup => "hgroup",

        // Grouping
        P => "p",
        Pre => "pre",
        Blockquote => "blockquote",
        Ol => "ol",
        Ul => "ul",
        Menu => "menu",
        Li => "li",
        Dl => "dl",
        Dt => "dt",
        Dd => "dd",
        Figure => "figure",
        Figcaption => "figcaption",
        Div => "div",

        // Text-level semantics
        A => "a",
        Em => "em",
        Strong => "strong",
        Small => "small",
        S => "s",
        Cite => "cite",
        Q => "q",
        Dfn => "dfn",
        Abbr => "abbr",
        Ruby => "ruby",
        Rt => "rt",
        Rp => "rp",
        Data => "data",
        Time => "time",
        Code => "code",
        Var => "var",
        Samp => "samp",
        Kbd => "kbd",
        Sub => "sub",
        Sup => "sup",
        I => "i",
        B => "b",
        U => "u",
        Mark => "mark",
        Bdi => "bdi",
        Bdo => "bdo",
        Span => "span",

        // Embedded / multimedia content
        Picture => "picture",
        Iframe => "iframe",
        Object => "object",
        Video => "video",
        Audio => "audio",
        Canvas => "canvas",
        Svg => "svg",
        Map => "map",

        // Tabular data
        Table => "table",
        Caption => "caption",
        Colgroup => "colgroup",
        Tbody => "tbody",
        Thead => "thead",
        Tfoot => "tfoot",
        Tr => "tr",
        Td => "td",
        Th => "th",

        // Forms
        Form => "form",
        Label => "label",
        Button => "button",
        Select => "select",
        Datalist => "datalist",
        Optgroup => "optgroup",
        Option => "option",
        Textarea => "textarea",
        Output => "output",
        Progress => "progress",
        Meter => "meter",
        Fieldset => "fieldset",
        Legend => "legend",

        // Interactive / Scripting
        Details => "details",
        Summary => "summary",
        Dialog => "dialog",
        Script => "script",
        Noscript => "noscript",
        Template => "template",
        Slot => "slot",
    ]
}

impl TagName {
    /// Validate and normalise `raw`. `Err(HtmlError::InvalidTag)` when it is
    /// empty, starts with a non-letter, or contains a character other than an
    /// ASCII alphanumeric or `-`.
    pub fn new(raw: &str) -> Result<Self, HtmlError> {
        let valid = starts_with_letter(raw) && raw.chars().skip(1).all(is_tag_character);
        if !valid {
            return Err(HtmlError::InvalidTag(raw.to_string()));
        }

        let lower = raw.to_ascii_lowercase();
        Ok(Self::from_standard_name(&lower).unwrap_or(Self::Custom(lower)))
    }

    /// The `<html>` element tag.
    #[must_use]
    pub const fn html() -> Self {
        Self::Html
    }

    /// The `<head>` element tag.
    #[must_use]
    pub const fn head() -> Self {
        Self::Head
    }

    /// The `<body>` element tag.
    #[must_use]
    pub const fn body() -> Self {
        Self::Body
    }

    /// The `<title>` element tag.
    #[must_use]
    pub const fn title() -> Self {
        Self::Title
    }

    /// The `<style>` element tag.
    #[must_use]
    pub const fn style() -> Self {
        Self::Style
    }

    /// The `<script>` element tag.
    #[must_use]
    pub const fn script() -> Self {
        Self::Script
    }

    /// The `<p>` element tag.
    #[must_use]
    pub const fn p() -> Self {
        Self::P
    }

    /// The `<li>` element tag.
    #[must_use]
    pub const fn li() -> Self {
        Self::Li
    }

    /// The `<div>` element tag.
    #[must_use]
    pub const fn div() -> Self {
        Self::Div
    }

    /// The `<span>` element tag.
    #[must_use]
    pub const fn span() -> Self {
        Self::Span
    }

    /// The `<a>` element tag.
    #[must_use]
    pub const fn a() -> Self {
        Self::A
    }

    /// The `<img>` void element tag.
    #[must_use]
    pub const fn img() -> Self {
        Self::Img
    }

    /// The `<video>` element tag.
    #[must_use]
    pub const fn video() -> Self {
        Self::Video
    }

    /// The `<meta>` void element tag.
    #[must_use]
    pub const fn meta() -> Self {
        Self::Meta
    }

    /// The `<link>` void element tag.
    #[must_use]
    pub const fn link() -> Self {
        Self::Link
    }

    /// The `<noscript>` element tag.
    #[must_use]
    pub const fn noscript() -> Self {
        Self::Noscript
    }

    /// Whether this tag is a raw-text or script-data element (`<script>` or `<style>`).
    #[must_use]
    pub const fn is_rawtext(&self) -> bool {
        matches!(self, Self::Script | Self::Style)
    }

    /// Whether this tag is a block-level element in the HTML content model.
    #[must_use]
    pub const fn is_block(&self) -> bool {
        matches!(
            self,
            Self::Address
                | Self::Article
                | Self::Aside
                | Self::Blockquote
                | Self::Details
                | Self::Dialog
                | Self::Dd
                | Self::Div
                | Self::Dl
                | Self::Dt
                | Self::Fieldset
                | Self::Figcaption
                | Self::Figure
                | Self::Footer
                | Self::Form
                | Self::H1
                | Self::H2
                | Self::H3
                | Self::H4
                | Self::H5
                | Self::H6
                | Self::Header
                | Self::Hgroup
                | Self::Hr
                | Self::Main
                | Self::Menu
                | Self::Nav
                | Self::Ol
                | Self::P
                | Self::Pre
                | Self::Section
                | Self::Table
                | Self::Ul
        )
    }

    /// Checks whether an open `<p>` tag should be automatically closed before inserting this tag.
    #[must_use]
    pub const fn closes_paragraph(&self) -> bool {
        self.is_block()
    }

    /// Checks whether an open `<li>` tag should be automatically closed before inserting this tag.
    #[must_use]
    pub const fn closes_list_item(&self) -> bool {
        matches!(self, Self::Li)
    }

    /// Checks whether this is one of the heading tags (`h1` through `h6`).
    #[must_use]
    pub const fn is_heading(&self) -> bool {
        matches!(
            self,
            Self::H1 | Self::H2 | Self::H3 | Self::H4 | Self::H5 | Self::H6
        )
    }

    /// Checks whether this tag is allowed inside `<head>` without triggering head termination.
    #[must_use]
    pub const fn is_head_content(&self) -> bool {
        matches!(
            self,
            Self::Title | Self::Meta | Self::Style | Self::Link | Self::Script | Self::Noscript
        )
    }

    /// Checks whether this tag is `<html>`.
    #[must_use]
    pub const fn is_html(&self) -> bool {
        matches!(self, Self::Html)
    }

    /// Checks whether this tag is `<head>`.
    #[must_use]
    pub const fn is_head(&self) -> bool {
        matches!(self, Self::Head)
    }

    /// Checks whether this tag is `<body>`.
    #[must_use]
    pub const fn is_body(&self) -> bool {
        matches!(self, Self::Body)
    }
}

fn starts_with_letter(raw: &str) -> bool {
    raw.chars()
        .next()
        .is_some_and(|character| character.is_ascii_alphabetic())
}

const fn is_tag_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '-'
}

impl fmt::Display for TagName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl PartialEq<str> for TagName {
    fn eq(&self, other: &str) -> bool {
        self.as_str().eq_ignore_ascii_case(other)
    }
}

impl PartialEq<&str> for TagName {
    fn eq(&self, other: &&str) -> bool {
        self.as_str().eq_ignore_ascii_case(other)
    }
}

impl PartialEq<TagName> for str {
    fn eq(&self, other: &TagName) -> bool {
        self.eq_ignore_ascii_case(other.as_str())
    }
}

impl PartialEq<TagName> for &str {
    fn eq(&self, other: &TagName) -> bool {
        self.eq_ignore_ascii_case(other.as_str())
    }
}

impl TryFrom<&str> for TagName {
    type Error = HtmlError;

    fn try_from(raw: &str) -> Result<Self, Self::Error> {
        Self::new(raw)
    }
}

impl TryFrom<String> for TagName {
    type Error = HtmlError;

    fn try_from(raw: String) -> Result<Self, Self::Error> {
        Self::new(&raw)
    }
}

impl FromStr for TagName {
    type Err = HtmlError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

/// Checks whether `name` is a W3C HTML5 void element.
///
/// Void elements never have child nodes or end tags.
#[must_use]
pub fn is_void_tag(name: &str) -> bool {
    TagName::new(name).is_ok_and(|tag| tag.is_void())
}

/// Checks whether `name` is a raw-text or script-data element.
///
/// In these elements, child markup is not tokenized; content is read as raw characters
/// until the corresponding end tag is reached.
#[must_use]
pub fn is_rawtext_tag(name: &str) -> bool {
    TagName::new(name).is_ok_and(|tag| tag.is_rawtext())
}

/// Checks whether `name` is a block-level element in the HTML content model.
#[must_use]
pub fn is_block_tag(name: &str) -> bool {
    TagName::new(name).is_ok_and(|tag| tag.is_block())
}

/// Checks whether an open `<p>` tag should be automatically closed before inserting `tag`.
#[must_use]
pub fn closes_paragraph(tag: &str) -> bool {
    TagName::new(tag).is_ok_and(|t| t.closes_paragraph())
}

/// Checks whether an open `<li>` tag should be automatically closed before inserting `tag`.
#[must_use]
pub fn closes_list_item(tag: &str) -> bool {
    TagName::new(tag).is_ok_and(|t| t.closes_list_item())
}

/// Checks whether `name` is one of the heading tags (`h1` through `h6`).
#[must_use]
pub fn is_heading_tag(name: &str) -> bool {
    TagName::new(name).is_ok_and(|t| t.is_heading())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_void_tags() {
        assert!(is_void_tag("br"));
        assert!(is_void_tag("img"));
        assert!(is_void_tag("input"));
        assert!(is_void_tag("hr"));
        assert!(is_void_tag("meta"));
        assert!(is_void_tag("link"));
        assert!(!is_void_tag("div"));
        assert!(!is_void_tag("p"));
    }

    #[test]
    fn test_rawtext_tags() {
        assert!(is_rawtext_tag("script"));
        assert!(is_rawtext_tag("style"));
        assert!(!is_rawtext_tag("textarea"));
        assert!(!is_rawtext_tag("div"));
    }

    #[test]
    fn test_block_and_heading_tags() {
        assert!(is_block_tag("div"));
        assert!(is_block_tag("p"));
        assert!(is_block_tag("h1"));
        assert!(is_heading_tag("h1"));
        assert!(is_heading_tag("h6"));
        assert!(!is_heading_tag("p"));
        assert!(!is_block_tag("span"));
    }

    #[test]
    fn test_omission_triggers() {
        assert!(closes_paragraph("p"));
        assert!(closes_paragraph("div"));
        assert!(!closes_paragraph("span"));
        assert!(closes_list_item("li"));
        assert!(!closes_list_item("div"));
    }
}

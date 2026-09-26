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
    fn a_standard_name_parses_case_insensitively_to_its_variant() {
        assert_eq!(TagName::new("DIV"), Ok(TagName::Div));
        assert_eq!("h3".parse::<TagName>(), Ok(TagName::H3));
        assert_eq!(TagName::try_from("Span"), Ok(TagName::Span));
        assert_eq!(TagName::try_from("br".to_owned()), Ok(TagName::Br));
    }

    #[test]
    fn an_unknown_but_well_formed_name_is_a_custom_element() {
        let tag = TagName::new("My-Widget").expect("valid custom element");
        assert_eq!(tag, TagName::Custom("my-widget".to_owned()));
        assert_eq!(tag.as_str(), "my-widget");
        assert_eq!(tag.to_string(), "my-widget");
    }

    #[test]
    fn a_malformed_name_is_a_typed_error() {
        for raw in ["", "1div", "-x", "a b", "a_b"] {
            assert_eq!(
                TagName::new(raw),
                Err(HtmlError::InvalidTag(raw.to_owned()))
            );
        }
    }

    #[test]
    fn named_constructors_match_the_parsed_tag() {
        let pairs = [
            (TagName::html(), "html"),
            (TagName::head(), "head"),
            (TagName::body(), "body"),
            (TagName::title(), "title"),
            (TagName::style(), "style"),
            (TagName::script(), "script"),
            (TagName::p(), "p"),
            (TagName::li(), "li"),
            (TagName::div(), "div"),
            (TagName::span(), "span"),
            (TagName::a(), "a"),
            (TagName::img(), "img"),
            (TagName::video(), "video"),
            (TagName::meta(), "meta"),
            (TagName::link(), "link"),
            (TagName::noscript(), "noscript"),
        ];
        for (constructed, name) in pairs {
            assert_eq!(TagName::new(name), Ok(constructed));
        }
    }

    #[test]
    fn only_the_w3c_void_elements_are_void() {
        for name in [
            "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param",
            "source", "track", "wbr",
        ] {
            assert!(is_void_tag(name), "{name} must be void");
        }
        assert!(!is_void_tag("div"));
        assert!(!is_void_tag("1bad"));
    }

    #[test]
    fn only_script_and_style_are_rawtext() {
        assert!(is_rawtext_tag("script"));
        assert!(is_rawtext_tag("STYLE"));
        assert!(!is_rawtext_tag("p"));
        assert!(!is_rawtext_tag(""));
    }

    #[test]
    fn block_level_membership_follows_the_content_model() {
        for name in [
            "address",
            "article",
            "aside",
            "blockquote",
            "details",
            "dialog",
            "dd",
            "div",
            "dl",
            "dt",
            "fieldset",
            "figcaption",
            "figure",
            "footer",
            "form",
            "h1",
            "h2",
            "h3",
            "h4",
            "h5",
            "h6",
            "header",
            "hgroup",
            "hr",
            "main",
            "menu",
            "nav",
            "ol",
            "p",
            "pre",
            "section",
            "table",
            "ul",
        ] {
            assert!(is_block_tag(name), "{name} must be block");
            assert!(closes_paragraph(name), "{name} must close an open <p>");
        }
        assert!(!is_block_tag("span"));
        assert!(!closes_paragraph("span"));
    }

    #[test]
    fn only_li_closes_a_list_item() {
        assert!(closes_list_item("li"));
        assert!(!closes_list_item("ul"));
    }

    #[test]
    fn headings_are_h1_through_h6() {
        for name in ["h1", "h2", "h3", "h4", "h5", "h6"] {
            assert!(is_heading_tag(name));
        }
        assert!(!is_heading_tag("header"));
    }

    #[test]
    fn head_content_excludes_body_flow() {
        for tag in [
            TagName::Title,
            TagName::Meta,
            TagName::Style,
            TagName::Link,
            TagName::Script,
            TagName::Noscript,
        ] {
            assert!(tag.is_head_content());
        }
        assert!(!TagName::Div.is_head_content());
    }

    #[test]
    fn structural_predicates_identify_exactly_one_tag() {
        assert!(TagName::Html.is_html() && !TagName::Head.is_html());
        assert!(TagName::Head.is_head() && !TagName::Body.is_head());
        assert!(TagName::Body.is_body() && !TagName::Html.is_body());
    }

    #[test]
    fn comparison_with_a_string_ignores_ascii_case_in_both_directions() {
        let tag = TagName::Div;
        assert_eq!(tag, *"DIV");
        assert_eq!(tag, "Div");
        assert_eq!(*"dIv", tag);
        assert_eq!("div", tag);
        assert!(tag != "span");
    }
}

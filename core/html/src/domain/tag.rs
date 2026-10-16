//! Vocabulary of HTML tags and their structural semantics.

use crate::domain::error::InvalidTagName;
use core::fmt;

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
    /// Validate and normalise `raw`. `Err(InvalidTagName)` when it is empty, starts with a
    /// non-letter, or contains a character other than an ASCII alphanumeric or `-`.
    pub fn new(raw: &str) -> Result<Self, InvalidTagName> {
        let valid = starts_with_letter(raw) && raw.chars().skip(1).all(is_tag_character);
        if !valid {
            return Err(InvalidTagName::new(raw));
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

    /// The `<canvas>` element tag.
    #[must_use]
    pub const fn canvas() -> Self {
        Self::Canvas
    }

    /// The `<iframe>` element tag.
    #[must_use]
    pub const fn iframe() -> Self {
        Self::Iframe
    }

    /// The `<object>` element tag.
    #[must_use]
    pub const fn object() -> Self {
        Self::Object
    }

    /// The `<embed>` void element tag.
    #[must_use]
    pub const fn embed() -> Self {
        Self::Embed
    }

    /// The `<svg>` element tag.
    #[must_use]
    pub const fn svg() -> Self {
        Self::Svg
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

    /// Checks whether an open `<p>` tag should be automatically closed before inserting this tag
    /// (the block-level elements of the HTML content model, WHATWG §13.2.6.4.7).
    #[must_use]
    pub const fn closes_paragraph(&self) -> bool {
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

    /// Checks whether an open `<li>` tag should be automatically closed before inserting this tag.
    #[must_use]
    pub const fn closes_list_item(&self) -> bool {
        matches!(self, Self::Li)
    }

    /// Checks whether an open `<dd>` or `<dt>` tag should be automatically closed before inserting this tag.
    #[must_use]
    pub const fn closes_definition_item(&self) -> bool {
        matches!(self, Self::Dd | Self::Dt)
    }

    /// Checks whether this tag belongs to the WHATWG §13.2.4.2 "special" category.
    #[must_use]
    pub const fn is_special(&self) -> bool {
        if self.is_void() {
            return true;
        }
        matches!(
            self,
            Self::Html
                | Self::Head
                | Self::Title
                | Self::Style
                | Self::Body
                | Self::Article
                | Self::Section
                | Self::Nav
                | Self::Aside
                | Self::H1
                | Self::H2
                | Self::H3
                | Self::H4
                | Self::H5
                | Self::H6
                | Self::Header
                | Self::Footer
                | Self::Address
                | Self::Main
                | Self::Hgroup
                | Self::P
                | Self::Pre
                | Self::Blockquote
                | Self::Ol
                | Self::Ul
                | Self::Menu
                | Self::Li
                | Self::Dl
                | Self::Dt
                | Self::Dd
                | Self::Figure
                | Self::Figcaption
                | Self::Div
                | Self::Table
                | Self::Caption
                | Self::Colgroup
                | Self::Tbody
                | Self::Thead
                | Self::Tfoot
                | Self::Tr
                | Self::Td
                | Self::Th
                | Self::Form
                | Self::Button
                | Self::Select
                | Self::Textarea
                | Self::Fieldset
                | Self::Details
                | Self::Summary
                | Self::Dialog
                | Self::Script
                | Self::Noscript
                | Self::Template
                | Self::Iframe
        )
    }

    /// Checks whether this tag acts as a list-item scope boundary (WHATWG §13.2.6.4.7: any
    /// element in the "special" category other than `address`, `div`, or `p`).
    #[must_use]
    pub const fn is_list_item_scope_boundary(&self) -> bool {
        if matches!(self, Self::Address | Self::Div | Self::P) {
            return false;
        }
        self.is_special()
    }

    /// Checks whether an end tag may close over this open element without a parse error.
    ///
    /// The "generate implied end tags" set restricted to the elements this builder models (WHATWG
    /// §13.2.6.3), plus `body`/`html`, which the spec also lets an end tag close over.
    #[must_use]
    pub const fn is_implied_end_tag(&self) -> bool {
        matches!(
            self,
            Self::P | Self::Li | Self::Dd | Self::Dt | Self::Body | Self::Html
        )
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

#[cfg(test)]
mod tests {
    use super::*;

    fn tag(raw: &str) -> TagName {
        TagName::new(raw).expect("valid tag name")
    }

    #[test]
    fn a_standard_name_parses_case_insensitively_to_its_variant() {
        assert_eq!(TagName::new("DIV"), Ok(TagName::Div));
        assert_eq!(TagName::new("h3"), Ok(TagName::H3));
        assert_eq!(TagName::new("Span"), Ok(TagName::Span));
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
            assert_eq!(TagName::new(raw), Err(InvalidTagName::new(raw)));
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
            (TagName::canvas(), "canvas"),
            (TagName::iframe(), "iframe"),
            (TagName::object(), "object"),
            (TagName::embed(), "embed"),
            (TagName::svg(), "svg"),
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
            assert!(tag(name).is_void(), "{name} must be void");
        }
        assert!(!tag("div").is_void());
    }

    #[test]
    fn only_script_and_style_are_rawtext() {
        assert!(tag("script").is_rawtext());
        assert!(tag("STYLE").is_rawtext());
        assert!(!tag("p").is_rawtext());
    }

    #[test]
    fn block_level_elements_close_an_open_paragraph() {
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
            assert!(
                tag(name).closes_paragraph(),
                "{name} must close an open <p>"
            );
        }
        assert!(!tag("span").closes_paragraph());
    }

    #[test]
    fn only_li_closes_a_list_item() {
        assert!(tag("li").closes_list_item());
        assert!(!tag("ul").closes_list_item());
    }

    #[test]
    fn closes_definition_item_matches_dd_and_dt() {
        assert!(tag("dd").closes_definition_item());
        assert!(tag("dt").closes_definition_item());
        assert!(!tag("dl").closes_definition_item());
        assert!(!tag("li").closes_definition_item());
    }

    #[test]
    fn headings_are_h1_through_h6() {
        for name in ["h1", "h2", "h3", "h4", "h5", "h6"] {
            assert!(tag(name).is_heading());
        }
        assert!(!tag("header").is_heading());
    }

    #[test]
    fn implied_end_tags_match_spec_subset() {
        for name in ["p", "li", "dd", "dt", "body", "html"] {
            assert!(tag(name).is_implied_end_tag(), "{name}");
        }
        assert!(!tag("span").is_implied_end_tag());
    }

    #[test]
    fn list_item_scope_boundary_respects_spec_rules() {
        for name in ["ul", "ol", "dl", "body", "html", "table", "section"] {
            assert!(
                tag(name).is_list_item_scope_boundary(),
                "{name} must be a scope boundary"
            );
        }
        for name in ["address", "div", "p", "span", "a", "strong"] {
            assert!(
                !tag(name).is_list_item_scope_boundary(),
                "{name} must not be a scope boundary"
            );
        }
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

//! [`TagName`] — a validated, strongly-typed element tag enum supporting standard
//! HTML5 W3C elements and autonomous custom tags (v0.2 report §2.2).

use core::fmt;

use crate::domain::error::DomError;

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
    /// Validate and normalise `raw`. `Err(DomError::InvalidTagName)` when it is
    /// empty, starts with a non-letter, or contains a character other than an
    /// ASCII alphanumeric or `-`.
    pub fn new(raw: &str) -> Result<Self, DomError> {
        let valid = starts_with_letter(raw) && raw.chars().skip(1).all(is_tag_character);
        if !valid {
            return Err(DomError::invalid_tag_name(raw));
        }

        let lower = raw.to_ascii_lowercase();
        Ok(Self::from_standard_name(&lower).unwrap_or(Self::Custom(lower)))
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

    /// The `<div>` element tag.
    #[must_use]
    pub const fn div() -> Self {
        Self::Div
    }

    /// The `<p>` element tag.
    #[must_use]
    pub const fn p() -> Self {
        Self::P
    }

    /// The `<span>` element tag.
    #[must_use]
    pub const fn span() -> Self {
        Self::Span
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

    /// Whether this tag is a replaced element whose size comes from a resource.
    #[must_use]
    pub const fn is_replaced(&self) -> bool {
        matches!(
            self,
            Self::Img
                | Self::Video
                | Self::Canvas
                | Self::Iframe
                | Self::Object
                | Self::Embed
                | Self::Svg
        )
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
        self.as_str() == other
    }
}

impl PartialEq<&str> for TagName {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<TagName> for str {
    fn eq(&self, other: &TagName) -> bool {
        self == other.as_str()
    }
}

impl PartialEq<TagName> for &str {
    fn eq(&self, other: &TagName) -> bool {
        *self == other.as_str()
    }
}

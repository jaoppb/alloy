use crate::domain::error::DomError;
use std::fmt;

/// Strongly typed HTML5 element tag name mapped to the W3C standard with extensible fallback (C-25).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TagName {
    // Document root & metadata
    Html,
    Head,
    Title,
    Base,
    Link,
    Meta,
    Style,

    // Sectioning elements
    Body,
    Article,
    Section,
    Nav,
    Aside,
    H1,
    H2,
    H3,
    H4,
    H5,
    H6,
    Hgroup,
    Header,
    Footer,
    Address,
    Main,

    // Grouping content
    P,
    Hr,
    Pre,
    Blockquote,
    Ol,
    Ul,
    Menu,
    Li,
    Dl,
    Dt,
    Dd,
    Figure,
    Figcaption,
    Div,

    // Text-level semantics
    A,
    Em,
    Strong,
    Small,
    S,
    Cite,
    Q,
    Dfn,
    Abbr,
    Ruby,
    Rt,
    Rp,
    Data,
    Time,
    Code,
    Var,
    Samp,
    Kbd,
    Sub,
    Sup,
    I,
    B,
    U,
    Mark,
    Bdi,
    Bdo,
    Span,
    Br,
    Wbr,

    // Edits
    Ins,
    Del,

    // Embedded content
    Picture,
    Source,
    Img,
    Iframe,
    Embed,
    Object,
    Video,
    Audio,
    Track,
    Canvas,

    // Tabular data
    Table,
    Caption,
    Colgroup,
    Col,
    Tbody,
    Thead,
    Tfoot,
    Tr,
    Td,
    Th,

    // Forms
    Form,
    Label,
    Input,
    Button,
    Select,
    Datalist,
    Optgroup,
    Option,
    Textarea,
    Output,
    Progress,
    Meter,
    Fieldset,
    Legend,

    // Interactive & scripting
    Details,
    Summary,
    Dialog,
    Script,
    Noscript,
    Template,
    Slot,

    // Extensible custom elements / Web Components (e.g. <my-widget>)
    Custom(String),
}

impl TagName {
    /// Creates and validates a new `TagName`, normalizing to lowercase.
    ///
    /// # Errors
    /// Returns `DomError::InvalidTagName` if `name` is empty or only whitespace.
    pub fn new(name: impl Into<String>) -> Result<Self, DomError> {
        let raw = name.into();
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(DomError::InvalidTagName(raw));
        }

        let lower = trimmed.to_ascii_lowercase();
        Ok(match lower.as_str() {
            "html" => Self::Html,
            "head" => Self::Head,
            "title" => Self::Title,
            "base" => Self::Base,
            "link" => Self::Link,
            "meta" => Self::Meta,
            "style" => Self::Style,
            "body" => Self::Body,
            "article" => Self::Article,
            "section" => Self::Section,
            "nav" => Self::Nav,
            "aside" => Self::Aside,
            "h1" => Self::H1,
            "h2" => Self::H2,
            "h3" => Self::H3,
            "h4" => Self::H4,
            "h5" => Self::H5,
            "h6" => Self::H6,
            "hgroup" => Self::Hgroup,
            "header" => Self::Header,
            "footer" => Self::Footer,
            "address" => Self::Address,
            "main" => Self::Main,
            "p" => Self::P,
            "hr" => Self::Hr,
            "pre" => Self::Pre,
            "blockquote" => Self::Blockquote,
            "ol" => Self::Ol,
            "ul" => Self::Ul,
            "menu" => Self::Menu,
            "li" => Self::Li,
            "dl" => Self::Dl,
            "dt" => Self::Dt,
            "dd" => Self::Dd,
            "figure" => Self::Figure,
            "figcaption" => Self::Figcaption,
            "div" => Self::Div,
            "a" => Self::A,
            "em" => Self::Em,
            "strong" => Self::Strong,
            "small" => Self::Small,
            "s" => Self::S,
            "cite" => Self::Cite,
            "q" => Self::Q,
            "dfn" => Self::Dfn,
            "abbr" => Self::Abbr,
            "ruby" => Self::Ruby,
            "rt" => Self::Rt,
            "rp" => Self::Rp,
            "data" => Self::Data,
            "time" => Self::Time,
            "code" => Self::Code,
            "var" => Self::Var,
            "samp" => Self::Samp,
            "kbd" => Self::Kbd,
            "sub" => Self::Sub,
            "sup" => Self::Sup,
            "i" => Self::I,
            "b" => Self::B,
            "u" => Self::U,
            "mark" => Self::Mark,
            "bdi" => Self::Bdi,
            "bdo" => Self::Bdo,
            "span" => Self::Span,
            "br" => Self::Br,
            "wbr" => Self::Wbr,
            "ins" => Self::Ins,
            "del" => Self::Del,
            "picture" => Self::Picture,
            "source" => Self::Source,
            "img" => Self::Img,
            "iframe" => Self::Iframe,
            "embed" => Self::Embed,
            "object" => Self::Object,
            "video" => Self::Video,
            "audio" => Self::Audio,
            "track" => Self::Track,
            "canvas" => Self::Canvas,
            "table" => Self::Table,
            "caption" => Self::Caption,
            "colgroup" => Self::Colgroup,
            "col" => Self::Col,
            "tbody" => Self::Tbody,
            "thead" => Self::Thead,
            "tfoot" => Self::Tfoot,
            "tr" => Self::Tr,
            "td" => Self::Td,
            "th" => Self::Th,
            "form" => Self::Form,
            "label" => Self::Label,
            "input" => Self::Input,
            "button" => Self::Button,
            "select" => Self::Select,
            "datalist" => Self::Datalist,
            "optgroup" => Self::Optgroup,
            "option" => Self::Option,
            "textarea" => Self::Textarea,
            "output" => Self::Output,
            "progress" => Self::Progress,
            "meter" => Self::Meter,
            "fieldset" => Self::Fieldset,
            "legend" => Self::Legend,
            "details" => Self::Details,
            "summary" => Self::Summary,
            "dialog" => Self::Dialog,
            "script" => Self::Script,
            "noscript" => Self::Noscript,
            "template" => Self::Template,
            "slot" => Self::Slot,
            _ => Self::Custom(lower),
        })
    }

    /// Accesses the canonical tag name as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Html => "html",
            Self::Head => "head",
            Self::Title => "title",
            Self::Base => "base",
            Self::Link => "link",
            Self::Meta => "meta",
            Self::Style => "style",
            Self::Body => "body",
            Self::Article => "article",
            Self::Section => "section",
            Self::Nav => "nav",
            Self::Aside => "aside",
            Self::H1 => "h1",
            Self::H2 => "h2",
            Self::H3 => "h3",
            Self::H4 => "h4",
            Self::H5 => "h5",
            Self::H6 => "h6",
            Self::Hgroup => "hgroup",
            Self::Header => "header",
            Self::Footer => "footer",
            Self::Address => "address",
            Self::Main => "main",
            Self::P => "p",
            Self::Hr => "hr",
            Self::Pre => "pre",
            Self::Blockquote => "blockquote",
            Self::Ol => "ol",
            Self::Ul => "ul",
            Self::Menu => "menu",
            Self::Li => "li",
            Self::Dl => "dl",
            Self::Dt => "dt",
            Self::Dd => "dd",
            Self::Figure => "figure",
            Self::Figcaption => "figcaption",
            Self::Div => "div",
            Self::A => "a",
            Self::Em => "em",
            Self::Strong => "strong",
            Self::Small => "small",
            Self::S => "s",
            Self::Cite => "cite",
            Self::Q => "q",
            Self::Dfn => "dfn",
            Self::Abbr => "abbr",
            Self::Ruby => "ruby",
            Self::Rt => "rt",
            Self::Rp => "rp",
            Self::Data => "data",
            Self::Time => "time",
            Self::Code => "code",
            Self::Var => "var",
            Self::Samp => "samp",
            Self::Kbd => "kbd",
            Self::Sub => "sub",
            Self::Sup => "sup",
            Self::I => "i",
            Self::B => "b",
            Self::U => "u",
            Self::Mark => "mark",
            Self::Bdi => "bdi",
            Self::Bdo => "bdo",
            Self::Span => "span",
            Self::Br => "br",
            Self::Wbr => "wbr",
            Self::Ins => "ins",
            Self::Del => "del",
            Self::Picture => "picture",
            Self::Source => "source",
            Self::Img => "img",
            Self::Iframe => "iframe",
            Self::Embed => "embed",
            Self::Object => "object",
            Self::Video => "video",
            Self::Audio => "audio",
            Self::Track => "track",
            Self::Canvas => "canvas",
            Self::Table => "table",
            Self::Caption => "caption",
            Self::Colgroup => "colgroup",
            Self::Col => "col",
            Self::Tbody => "tbody",
            Self::Thead => "thead",
            Self::Tfoot => "tfoot",
            Self::Tr => "tr",
            Self::Td => "td",
            Self::Th => "th",
            Self::Form => "form",
            Self::Label => "label",
            Self::Input => "input",
            Self::Button => "button",
            Self::Select => "select",
            Self::Datalist => "datalist",
            Self::Optgroup => "optgroup",
            Self::Option => "option",
            Self::Textarea => "textarea",
            Self::Output => "output",
            Self::Progress => "progress",
            Self::Meter => "meter",
            Self::Fieldset => "fieldset",
            Self::Legend => "legend",
            Self::Details => "details",
            Self::Summary => "summary",
            Self::Dialog => "dialog",
            Self::Script => "script",
            Self::Noscript => "noscript",
            Self::Template => "template",
            Self::Slot => "slot",
            Self::Custom(name) => name.as_str(),
        }
    }

    /// Checks if this is an HTML5 void element that forbids closing tags and child nodes (C-37).
    #[must_use]
    pub const fn is_void(&self) -> bool {
        matches!(
            self,
            Self::Base
                | Self::Link
                | Self::Meta
                | Self::Hr
                | Self::Br
                | Self::Wbr
                | Self::Img
                | Self::Embed
                | Self::Source
                | Self::Track
                | Self::Col
                | Self::Input
        )
    }

    /// Returns the default CSS display mode for this element (C-13, C-49).
    #[must_use]
    pub const fn default_display(&self) -> engine::DisplayType {
        match self {
            Self::Span
            | Self::A
            | Self::B
            | Self::I
            | Self::Em
            | Self::Strong
            | Self::Small
            | Self::S
            | Self::Cite
            | Self::Q
            | Self::Dfn
            | Self::Abbr
            | Self::Code
            | Self::Var
            | Self::Samp
            | Self::Kbd
            | Self::Sub
            | Self::Sup
            | Self::U
            | Self::Mark
            | Self::Bdi
            | Self::Bdo
            | Self::Br
            | Self::Wbr
            | Self::Img
            | Self::Input
            | Self::Button
            | Self::Select
            | Self::Textarea
            | Self::Label => engine::DisplayType::Inline,

            Self::Head
            | Self::Title
            | Self::Base
            | Self::Link
            | Self::Meta
            | Self::Style
            | Self::Script
            | Self::Noscript
            | Self::Template => engine::DisplayType::None,

            _ => engine::DisplayType::Block,
        }
    }
}

impl fmt::Display for TagName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

//! Vocabulary of HTML tags and their structural semantics.

/// Checks whether `name` is a W3C HTML5 void element.
///
/// Void elements never have child nodes or end tags.
#[must_use]
pub fn is_void_tag(name: &str) -> bool {
    matches!(
        name,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

/// Checks whether `name` is a raw-text or script-data element.
///
/// In these elements, child markup is not tokenized; content is read as raw characters
/// until the corresponding end tag is reached.
#[must_use]
pub fn is_rawtext_tag(name: &str) -> bool {
    matches!(name, "script" | "style")
}

/// Checks whether `name` is a block-level element in the HTML content model.
#[must_use]
pub fn is_block_tag(name: &str) -> bool {
    matches!(
        name,
        "address"
            | "article"
            | "aside"
            | "blockquote"
            | "details"
            | "dialog"
            | "dd"
            | "div"
            | "dl"
            | "dt"
            | "fieldset"
            | "figcaption"
            | "figure"
            | "footer"
            | "form"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "header"
            | "hgroup"
            | "hr"
            | "main"
            | "menu"
            | "nav"
            | "ol"
            | "p"
            | "pre"
            | "section"
            | "table"
            | "ul"
    )
}

/// Checks whether an open `<p>` tag should be automatically closed before inserting `tag`.
#[must_use]
pub fn closes_paragraph(tag: &str) -> bool {
    is_block_tag(tag)
}

/// Checks whether an open `<li>` tag should be automatically closed before inserting `tag`.
#[must_use]
pub fn closes_list_item(tag: &str) -> bool {
    matches!(tag, "li")
}

/// Checks whether `name` is one of the heading tags (`h1` through `h6`).
#[must_use]
pub fn is_heading_tag(name: &str) -> bool {
    matches!(name, "h1" | "h2" | "h3" | "h4" | "h5" | "h6")
}

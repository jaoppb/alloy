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

// Alloy CSS Cascade Engine (Rhai Module)
// Paired with core/css/src/application/cascade.rs

fn resolve_default_display(tag_name) {
    switch tag_name {
        "div" | "p" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" |
        "body" | "html" | "header" | "footer" | "section" | "article" | "nav" => "block",
        "span" | "a" | "b" | "i" | "strong" | "em" | "code" => "inline",
        _ => "block",
    }
}

fn can_inherit_property(property_name) {
    switch property_name {
        "color" | "font-size" | "line-height" | "text-align" => true,
        _ => false,
    }
}

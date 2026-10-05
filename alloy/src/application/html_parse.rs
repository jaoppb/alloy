//! The one place `alloy` calls `dom::parse`: a page's recoverable parse errors are summarized in
//! a single log line instead of being dropped or logged one by one (issue #36, ADR-0023).

use html::HtmlError;

/// Parses `markup` into a DOM tree, logging one summary when the parser had to recover.
pub fn parse_html(markup: &str) -> Result<dom::DomTree, HtmlError> {
    let outcome = dom::parse(markup)?;
    let diagnostics = outcome.diagnostics();
    if let Some(first) = diagnostics.first() {
        tracing::debug!(
            count = diagnostics.len(),
            first = %first,
            "html parse recovered from errors"
        );
    }
    Ok(outcome.into_tree())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_malformed_page_still_parses_into_a_tree() {
        let tree = parse_html("<p>a</p><my_widget>b</my_widget>").unwrap();
        assert!(tree.descendants(tree.document()).count() > 1);
    }
}

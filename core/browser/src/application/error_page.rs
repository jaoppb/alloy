//! The visible page a failed navigation is replaced with — a failure must be
//! seen, not just logged.
//!
//! The markup is an embedded resource (#110 decision 16: the Muscle may replace
//! it later) and the message is attached to it as a DOM **text node**, so no
//! character of it can ever be read as markup and nothing needs escaping.

use dom::DomTree;
use dom::NodeId;
use html::{AttributeName, HtmlError};

const TEMPLATE: &str = include_str!("error.html");
const MESSAGE_SLOT_ID: &str = "error-message";

/// Why the error page could not be built. Never reachable with the embedded
/// template unless it is edited without its test.
#[derive(Debug, thiserror::Error)]
pub enum ErrorPageError {
    /// The embedded template did not parse.
    #[error("error page template does not parse: {0}")]
    Template(#[from] HtmlError),
    /// The embedded template lost its `id="error-message"` element.
    #[error("error page template has no element with id `{MESSAGE_SLOT_ID}`")]
    MissingMessageSlot,
    /// Attaching the message to the tree was refused.
    #[error("error page message could not be attached: {0}")]
    Attach(#[from] dom::DomError),
}

/// A document announcing that loading failed, with `message` shown as text.
///
/// # Errors
///
/// [`ErrorPageError`] when the embedded template is malformed or the message
/// cannot be attached — neither happens with the template as shipped.
pub fn error_page(message: &str) -> Result<DomTree, ErrorPageError> {
    let mut tree = dom::parse(TEMPLATE)?.into_tree();
    let slot = message_slot(&tree).ok_or(ErrorPageError::MissingMessageSlot)?;
    let text = tree.create_text(dom::TextContent::new(message));
    tree.append_child(slot, text)?;
    Ok(tree)
}

fn message_slot(tree: &DomTree) -> Option<NodeId> {
    let id_name = AttributeName::id();
    tree.descendants(tree.document()).find(|&node| {
        tree.attribute(node, &id_name)
            .is_ok_and(|value| value.is_some_and(|value| value.as_str() == MESSAGE_SLOT_ID))
    })
}

#[cfg(test)]
mod tests {
    use super::error_page;

    fn text_nodes(tree: &dom::DomTree) -> Vec<String> {
        tree.descendants(tree.document())
            .filter_map(|node| tree.text(node).ok())
            .map(|content| content.as_str().to_owned())
            .collect()
    }

    fn element_count(tree: &dom::DomTree, tag: &str) -> usize {
        tree.descendants(tree.document())
            .filter(|&node| tree.tag(node).is_ok_and(|name| name.as_str() == tag))
            .count()
    }

    #[test]
    fn the_message_is_shown_as_a_text_node() {
        let tree = error_page("connection refused").unwrap();

        assert!(text_nodes(&tree).contains(&"connection refused".to_owned()));
    }

    #[test]
    fn markup_in_the_message_stays_text_and_injects_no_element() {
        let tree = error_page("<script>&").unwrap();

        assert!(
            text_nodes(&tree).contains(&"<script>&".to_owned()),
            "the message must arrive verbatim, got {:?}",
            text_nodes(&tree)
        );
        assert_eq!(element_count(&tree, "script"), 0, "no markup was injected");
    }

    #[test]
    fn an_entity_in_the_message_is_not_decoded() {
        let tree = error_page("a &amp; b").unwrap();

        assert!(text_nodes(&tree).contains(&"a &amp; b".to_owned()));
    }

    #[test]
    fn the_embedded_template_parses_without_recovering_from_errors() {
        let outcome = dom::parse(super::TEMPLATE).unwrap();

        assert!(
            outcome.diagnostics().is_empty(),
            "{:?}",
            outcome.diagnostics()
        );
    }
}

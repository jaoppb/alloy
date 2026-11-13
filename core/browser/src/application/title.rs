//! Reading a document's title out of its tree (#84).

use dom::{DomTree, NodeId, TextContent};
use html::TagName;

use crate::domain::DocumentTitle;

/// The collapsed text of the first HTML `<title>` in tree order (WHATWG
/// §4.2.2); `None` when there is none or it is blank. An SVG `<title>` is a
/// different element and never names the document.
#[must_use]
pub fn document_title(tree: &DomTree) -> Option<DocumentTitle> {
    let title = tree
        .descendants(tree.document())
        .find(|&node| is_html_title(tree, node))?;
    DocumentTitle::collapsed(&child_text(tree, title))
}

fn is_html_title(tree: &DomTree, node: NodeId) -> bool {
    let is_title = tree.tag(node).is_ok_and(|tag| *tag == TagName::Title);
    is_title && tree.is_foreign(node).is_ok_and(|foreign| !foreign)
}

/// WHATWG "child text content": the direct text children, concatenated.
fn child_text(tree: &DomTree, parent: NodeId) -> String {
    tree.children(parent)
        .filter_map(|child| tree.text(child).ok())
        .map(TextContent::as_str)
        .collect()
}

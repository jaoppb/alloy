use crate::domain::token::{HtmlError, HtmlToken};
use dom::{DomTree, NodeId};

/// Constructs a `DomTree` hierarchy by consuming `HtmlToken` items.
pub struct TreeBuilder {
    tree: DomTree,
    open_elements: Vec<NodeId>,
}

impl Default for TreeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TreeBuilder {
    /// Creates a new `TreeBuilder` initialized with a root Document node.
    #[must_use]
    pub fn new() -> Self {
        let mut tree = DomTree::new();
        let doc_id = tree.create_document();
        Self {
            tree,
            open_elements: vec![doc_id],
        }
    }

    /// Consumes a token and updates the DOM insertion state.
    ///
    /// # Errors
    /// Returns `HtmlError` if a DOM insertion invariant is violated.
    pub fn process_token(&mut self, token: HtmlToken) -> Result<(), HtmlError> {
        match token {
            HtmlToken::Doctype(raw) => {
                let doctype = dom::Doctype::parse(&raw);
                self.tree.set_doctype(doctype);
                Ok(())
            }
            HtmlToken::Comment(comment) => {
                let comment_id = self.tree.create_comment(comment);
                self.attach_to_current(comment_id)
            }
            HtmlToken::Character(text) => {
                if text.is_empty() {
                    return Ok(());
                }
                let text_id = self.tree.create_text(text);
                self.attach_to_current(text_id)
            }
            HtmlToken::StartTag {
                name,
                attributes,
                self_closing,
            } => {
                let is_void = name.is_void() || self_closing;
                let el_id = self.tree.create_element(name, attributes);
                self.attach_to_current(el_id)?;

                if !is_void {
                    self.open_elements.push(el_id);
                }

                Ok(())
            }
            HtmlToken::EndTag(name) => {
                self.close_element(&name);
                Ok(())
            }
            HtmlToken::Eof => {
                self.open_elements.truncate(1); // Keep only document root
                Ok(())
            }
        }
    }

    /// Completes parsing and yields the finalized `DomTree`.
    #[must_use]
    pub fn finish(mut self) -> DomTree {
        self.open_elements.truncate(1);
        self.tree
    }

    fn attach_to_current(&mut self, child_id: NodeId) -> Result<(), HtmlError> {
        let current_parent = *self.open_elements.last().ok_or(HtmlError::UnexpectedEof)?;

        self.tree
            .append_child(current_parent, child_id)
            .map_err(HtmlError::from)
    }

    fn close_element(&mut self, tag: &dom::TagName) {
        // Search backwards in insertion stack for matching element (skip index 0, which is Document)
        let mut target_index = None;
        for (i, &node_id) in self.open_elements.iter().enumerate().skip(1).rev() {
            if let Some(node) = self.tree.get(node_id) {
                if let Some(node_tag) = node.data().as_element_tag() {
                    if node_tag == tag {
                        target_index = Some(i);
                        break;
                    }
                }
            }
        }

        if let Some(idx) = target_index {
            self.open_elements.truncate(idx);
        }
    }
}

/// Checks if a tag is an HTML5 void element that does not require closing tags (C-37).
#[must_use]
pub fn is_void_element(tag: &str) -> bool {
    dom::TagName::new(tag).is_ok_and(|t| t.is_void())
}

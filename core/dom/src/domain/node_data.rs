use crate::domain::attribute::AttributeMap;
use crate::domain::tag_name::TagName;

/// The specific payload associated with a node in the DOM tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeData {
    /// Root document node.
    Document,
    /// Element node with a tag name and attributes.
    Element {
        /// The HTML/XML tag name.
        tag_name: TagName,
        /// Element attributes.
        attributes: AttributeMap,
    },
    /// Text leaf node.
    Text(String),
    /// Comment node.
    Comment(String),
}

impl NodeData {
    /// Returns the element tag name if this is an element node.
    #[must_use]
    pub fn as_element_tag(&self) -> Option<&TagName> {
        match self {
            Self::Element { tag_name, .. } => Some(tag_name),
            _ => None,
        }
    }

    /// Returns the text content if this is a text node.
    #[must_use]
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(text) => Some(text.as_str()),
            _ => None,
        }
    }

    /// Returns a mutable reference to text content if this is a text node.
    pub fn as_text_mut(&mut self) -> Option<&mut String> {
        match self {
            Self::Text(text) => Some(text),
            _ => None,
        }
    }

    /// Checks if the node is an element.
    #[must_use]
    pub fn is_element(&self) -> bool {
        matches!(self, Self::Element { .. })
    }

    /// Checks if the node is text.
    #[must_use]
    pub fn is_text(&self) -> bool {
        matches!(self, Self::Text(_))
    }
}

//! Replaceable port traits for HTML token processing and tree construction.
//!
//! Conforms to ADR-0011 (Replaceable Subsystem Ports) and PRD-008.

use crate::domain::attribute::AttributeList;
use crate::domain::error::HtmlError;
use crate::domain::handle::NodeHandle;
use crate::domain::tag_name::TagName;
use crate::domain::text::Text;
use crate::domain::token::Token;

/// Kind of raw text element requiring specialized tokenizer handling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RawKind {
    /// `<script>` script data mode.
    Script,
    /// `<style>` raw text mode.
    Style,
}

/// A descriptor for a script encountered by the tokenizer or tree sink.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptDescriptor {
    content: String,
}

impl ScriptDescriptor {
    /// Creates a script descriptor from script content.
    #[must_use]
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
        }
    }

    /// Access the script source content.
    #[must_use]
    pub fn content(&self) -> &str {
        &self.content
    }
}

/// Action to be taken by the tokenizer after delivering a token to [`TokenSink`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TokenSinkResult {
    /// Proceed with normal parsing in current state.
    Continue,
    /// Switch tokenizer into the specified raw text / script data mode.
    SwitchToRawText(RawKind),
    /// Script encountered that may suspend parsing.
    Script(ScriptDescriptor),
    /// Request the tokenizer to suspend immediately (e.g. for synchronous script execution).
    Suspend,
}

/// A sink that consumes tokens emitted by an HTML tokenizer.
pub trait TokenSink: Send + Sync {
    /// Process a single token from the tokenizer.
    fn process_token(&mut self, token: Token) -> Result<TokenSinkResult, HtmlError>;

    /// Complete token processing and finalize any pending tree operations.
    fn finish(&mut self) -> Result<(), HtmlError>;
}

/// An abstract, replaceable tree-construction sink that builds a DOM-like hierarchy.
///
/// Implements PRD-008 §3.3 without exposing any foreign concrete types.
pub trait TreeSink: Send + Sync {
    /// Create an element node with given tag name and attributes.
    fn create_element(
        &mut self,
        tag: TagName,
        attributes: &AttributeList,
    ) -> Result<NodeHandle, HtmlError>;

    /// Create a character data text node.
    fn create_text(&mut self, text: &Text) -> Result<NodeHandle, HtmlError>;

    /// Create a comment node.
    fn create_comment(&mut self, text: &Text) -> Result<NodeHandle, HtmlError>;

    /// Append `child` as the last child of `parent`.
    fn append_child(&mut self, parent: NodeHandle, child: NodeHandle) -> Result<(), HtmlError>;

    /// Insert `child` immediately before `sibling`.
    fn append_before_sibling(
        &mut self,
        sibling: NodeHandle,
        child: NodeHandle,
    ) -> Result<(), HtmlError>;

    /// Add attributes to an existing node if not already present.
    fn add_attributes_if_missing(
        &mut self,
        target: NodeHandle,
        attributes: &AttributeList,
    ) -> Result<(), HtmlError>;

    /// Remove `target` node from its parent.
    fn remove_from_parent(&mut self, target: NodeHandle) -> Result<(), HtmlError>;

    /// Reparent all children from `from` node to `to` node.
    fn reparent_children(&mut self, from: NodeHandle, to: NodeHandle) -> Result<(), HtmlError>;

    /// Return the root document node handle.
    fn root_node(&self) -> NodeHandle;
}

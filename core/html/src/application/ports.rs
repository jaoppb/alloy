//! Replaceable port traits for HTML token processing and tree construction.
//!
//! Conforms to ADR-0011 (Replaceable Subsystem Ports) and PRD-008.

use crate::domain::error::HtmlError;
use crate::domain::token::{AttributeList, Token};

/// Kind of raw text element requiring specialized tokenizer handling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RawKind {
    /// `<script>` script data mode.
    Script,
    /// `<style>` raw text mode.
    Style,
}

/// Action to be taken by the tokenizer after delivering a token to [`TokenSink`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenSinkResult {
    /// Proceed with normal parsing in current state.
    Continue,
    /// Switch tokenizer into the specified raw text / script data mode.
    SwitchToRawText(RawKind),
}

/// A sink that consumes tokens emitted by an HTML tokenizer.
pub trait TokenSink: Send + Sync {
    /// Process a single token from the tokenizer.
    fn process_token(&mut self, token: Token) -> Result<TokenSinkResult, HtmlError>;

    /// Complete token processing and finalize any pending tree operations.
    fn finish(&mut self) -> Result<(), HtmlError>;
}

/// An abstract tree-construction sink that builds a DOM-like hierarchy.
///
/// Implementations isolate tree construction logic from specific arena representations.
pub trait TreeSink: Send + Sync {
    /// Create an element node with given tag name and attributes.
    fn create_element(
        &mut self,
        tag: &str,
        attributes: &AttributeList,
    ) -> Result<dom::NodeId, HtmlError>;

    /// Create a character data text node.
    fn create_text(&mut self, text: &str) -> Result<dom::NodeId, HtmlError>;

    /// Create a comment node.
    fn create_comment(&mut self, text: &str) -> Result<dom::NodeId, HtmlError>;

    /// Append `child` as the last child of `parent`.
    fn append_child(&mut self, parent: dom::NodeId, child: dom::NodeId) -> Result<(), HtmlError>;

    /// Return the root document node id.
    fn root_node(&self) -> dom::NodeId;
}

//! HTML5 tree builder implementing the [`TokenSink`] port on top of a [`TreeSink`].

use crate::application::ports::{RawKind, TokenSink, TokenSinkResult, TreeSink};
use crate::domain::error::HtmlError;
use crate::domain::tag::{closes_list_item, closes_paragraph, is_void_tag};
use crate::domain::token::{AttributeList, TagToken, Token};

struct OpenElement {
    tag: String,
    node: dom::NodeId,
}

/// Builds a tree structure by consuming tokens and applying HTML5 tree construction rules.
pub struct TreeBuilder<'a> {
    sink: &'a mut dyn TreeSink,
    open_elements: Vec<OpenElement>,
    html_node: Option<dom::NodeId>,
    head_node: Option<dom::NodeId>,
    body_node: Option<dom::NodeId>,
    in_head: bool,
}

impl<'a> TreeBuilder<'a> {
    /// Create a new tree builder using the specified [`TreeSink`].
    #[must_use]
    pub fn new(sink: &'a mut dyn TreeSink) -> Self {
        Self {
            sink,
            open_elements: Vec::new(),
            html_node: None,
            head_node: None,
            body_node: None,
            in_head: false,
        }
    }

    fn current_parent(&self) -> dom::NodeId {
        if let Some(open) = self.open_elements.last() {
            return open.node;
        }
        if let Some(body) = self.body_node {
            return body;
        }
        if let Some(html) = self.html_node {
            return html;
        }
        self.sink.root_node()
    }

    fn ensure_html_element(&mut self) -> Result<dom::NodeId, HtmlError> {
        if let Some(node) = self.html_node {
            return Ok(node);
        }

        let empty_attrs = AttributeList::new();
        let node = self.sink.create_element("html", &empty_attrs)?;
        let root = self.sink.root_node();
        self.sink.append_child(root, node)?;
        self.html_node = Some(node);
        self.open_elements.push(OpenElement {
            tag: "html".to_string(),
            node,
        });
        Ok(node)
    }

    fn ensure_body_element(&mut self) -> Result<dom::NodeId, HtmlError> {
        if let Some(node) = self.body_node {
            return Ok(node);
        }

        self.ensure_html_element()?;
        if self.in_head {
            self.pop_head();
        }

        let html = self.html_node.unwrap_or_else(|| self.sink.root_node());
        let empty_attrs = AttributeList::new();
        let node = self.sink.create_element("body", &empty_attrs)?;
        self.sink.append_child(html, node)?;
        self.body_node = Some(node);
        self.open_elements.push(OpenElement {
            tag: "body".to_string(),
            node,
        });
        Ok(node)
    }

    fn pop_head(&mut self) {
        while let Some(index) = self.open_elements.iter().rposition(|e| e.tag == "head") {
            self.open_elements.truncate(index);
        }
        self.in_head = false;
    }

    fn handle_start_tag(&mut self, tag: &TagToken) -> Result<TokenSinkResult, HtmlError> {
        let name = tag.name().to_string();
        if name == "html" {
            return self.process_html_start_tag(tag);
        }
        if name == "head" {
            return self.process_head_start_tag(tag);
        }
        if name == "body" {
            return self.process_body_start_tag(tag);
        }

        if self.in_head
            && !matches!(
                name.as_str(),
                "title" | "meta" | "style" | "link" | "script" | "noscript"
            )
        {
            self.pop_head();
        }

        if !self.in_head && self.body_node.is_none() {
            self.ensure_body_element()?;
        }

        self.apply_omission_rules(&name);

        let parent = self.current_parent();
        let node = self.sink.create_element(&name, tag.attributes())?;
        self.sink.append_child(parent, node)?;

        let is_void = is_void_tag(&name) || tag.is_self_closing();
        if !is_void {
            self.open_elements.push(OpenElement {
                tag: name.clone(),
                node,
            });
        }

        if name == "script" {
            return Ok(TokenSinkResult::SwitchToRawText(RawKind::Script));
        }
        if name == "style" {
            return Ok(TokenSinkResult::SwitchToRawText(RawKind::Style));
        }

        Ok(TokenSinkResult::Continue)
    }

    fn process_html_start_tag(&mut self, tag: &TagToken) -> Result<TokenSinkResult, HtmlError> {
        if self.html_node.is_some() {
            return Ok(TokenSinkResult::Continue);
        }
        let root = self.sink.root_node();
        let node = self.sink.create_element("html", tag.attributes())?;
        self.sink.append_child(root, node)?;
        self.html_node = Some(node);
        self.open_elements.push(OpenElement {
            tag: "html".to_string(),
            node,
        });
        Ok(TokenSinkResult::Continue)
    }

    fn process_head_start_tag(&mut self, tag: &TagToken) -> Result<TokenSinkResult, HtmlError> {
        self.ensure_html_element()?;
        if self.head_node.is_some() {
            return Ok(TokenSinkResult::Continue);
        }
        let parent = self.current_parent();
        let node = self.sink.create_element("head", tag.attributes())?;
        self.sink.append_child(parent, node)?;
        self.head_node = Some(node);
        self.in_head = true;
        self.open_elements.push(OpenElement {
            tag: "head".to_string(),
            node,
        });
        Ok(TokenSinkResult::Continue)
    }

    fn process_body_start_tag(&mut self, tag: &TagToken) -> Result<TokenSinkResult, HtmlError> {
        self.ensure_html_element()?;
        if self.in_head {
            self.pop_head();
        }
        if self.body_node.is_some() {
            return Ok(TokenSinkResult::Continue);
        }
        let parent = self.html_node.unwrap_or_else(|| self.sink.root_node());
        let node = self.sink.create_element("body", tag.attributes())?;
        self.sink.append_child(parent, node)?;
        self.body_node = Some(node);
        self.open_elements.push(OpenElement {
            tag: "body".to_string(),
            node,
        });
        Ok(TokenSinkResult::Continue)
    }

    fn apply_omission_rules(&mut self, tag_name: &str) {
        if closes_paragraph(tag_name) {
            self.pop_matching_tag("p");
        }
        if closes_list_item(tag_name) {
            self.pop_matching_tag("li");
        }
    }

    fn pop_matching_tag(&mut self, target_tag: &str) {
        if let Some(pos) = self.open_elements.iter().rposition(|e| e.tag == target_tag) {
            self.open_elements.truncate(pos);
        }
    }

    fn handle_end_tag(&mut self, tag: &TagToken) {
        let name = tag.name();
        if name == "head" {
            self.pop_head();
            return;
        }

        if let Some(pos) = self.open_elements.iter().rposition(|e| e.tag == name) {
            self.open_elements.truncate(pos);
        }
    }

    fn handle_character(&mut self, content: &str) -> Result<(), HtmlError> {
        if content.is_empty() {
            return Ok(());
        }

        let is_all_whitespace = content.chars().all(char::is_whitespace);
        if is_all_whitespace && self.body_node.is_none() && !self.in_head {
            return Ok(());
        }

        if !self.in_head && self.body_node.is_none() {
            self.ensure_body_element()?;
        }

        let parent = self.current_parent();
        let node = self.sink.create_text(content)?;
        self.sink.append_child(parent, node)?;
        Ok(())
    }

    fn handle_comment(&mut self, content: &str) -> Result<(), HtmlError> {
        let parent = self.current_parent();
        let node = self.sink.create_comment(content)?;
        self.sink.append_child(parent, node)?;
        Ok(())
    }
}

impl TokenSink for TreeBuilder<'_> {
    fn process_token(&mut self, token: Token) -> Result<TokenSinkResult, HtmlError> {
        match token {
            Token::StartTag(ref tag) => self.handle_start_tag(tag),
            Token::EndTag(ref tag) => {
                self.handle_end_tag(tag);
                Ok(TokenSinkResult::Continue)
            }
            Token::Character(ref content) => {
                self.handle_character(content)?;
                Ok(TokenSinkResult::Continue)
            }
            Token::Comment(ref content) => {
                self.handle_comment(content)?;
                Ok(TokenSinkResult::Continue)
            }
            Token::Doctype(_) | Token::EndOfFile => Ok(TokenSinkResult::Continue),
        }
    }

    fn finish(&mut self) -> Result<(), HtmlError> {
        self.open_elements.clear();
        Ok(())
    }
}

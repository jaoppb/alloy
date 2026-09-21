//! HTML5 tree builder implementing the [`TokenSink`] port on top of a [`TreeSink`].

use crate::application::ports::{RawKind, TokenSink, TokenSinkResult, TreeSink};
use crate::domain::error::HtmlError;
use crate::domain::tag::TagName;
use crate::domain::token::{AttributeList, TagToken, Token};

struct OpenElement {
    tag: TagName,
    node: dom::NodeId,
}

/// Builds a tree structure by consuming tokens and applying HTML5 tree construction rules.
pub struct TreeBuilder<'a, S: TreeSink + ?Sized> {
    sink: &'a mut S,
    open_elements: Vec<OpenElement>,
    html_node: Option<dom::NodeId>,
    head_node: Option<dom::NodeId>,
    body_node: Option<dom::NodeId>,
    in_head: bool,
}

impl<'a, S: TreeSink + ?Sized> TreeBuilder<'a, S> {
    /// Create a new tree builder using the specified [`TreeSink`].
    #[must_use]
    pub const fn new(sink: &'a mut S) -> Self {
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
        let node = self
            .sink
            .create_element(TagName::html().as_str(), &empty_attrs)?;
        let root = self.sink.root_node();
        self.sink.append_child(root, node)?;
        self.html_node = Some(node);
        self.open_elements.push(OpenElement {
            tag: TagName::html(),
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
        let node = self
            .sink
            .create_element(TagName::body().as_str(), &empty_attrs)?;
        self.sink.append_child(html, node)?;
        self.body_node = Some(node);
        self.open_elements.push(OpenElement {
            tag: TagName::body(),
            node,
        });
        Ok(node)
    }

    fn pop_head(&mut self) {
        while let Some(index) = self.open_elements.iter().rposition(|e| e.tag.is_head()) {
            self.open_elements.truncate(index);
        }
        self.in_head = false;
    }

    fn handle_start_tag(&mut self, tag: &TagToken) -> Result<TokenSinkResult, HtmlError> {
        let tag_name = tag.tag();
        match tag_name {
            TagName::Html => return self.process_html_start_tag(tag),
            TagName::Head => return self.process_head_start_tag(tag),
            TagName::Body => return self.process_body_start_tag(tag),
            _ => {}
        }

        if self.in_head && !tag_name.is_head_content() {
            self.pop_head();
        }

        if !self.in_head && self.body_node.is_none() {
            self.ensure_body_element()?;
        }

        self.apply_omission_rules(tag_name);

        let parent = self.current_parent();
        let node = self
            .sink
            .create_element(tag_name.as_str(), tag.attributes())?;
        self.sink.append_child(parent, node)?;

        let is_void = tag_name.is_void() || tag.is_self_closing();
        if !is_void {
            self.open_elements.push(OpenElement {
                tag: tag_name.clone(),
                node,
            });
        }

        if matches!(tag_name, TagName::Script) {
            return Ok(TokenSinkResult::SwitchToRawText(RawKind::Script));
        }
        if matches!(tag_name, TagName::Style) {
            return Ok(TokenSinkResult::SwitchToRawText(RawKind::Style));
        }

        Ok(TokenSinkResult::Continue)
    }

    fn process_html_start_tag(&mut self, tag: &TagToken) -> Result<TokenSinkResult, HtmlError> {
        if self.html_node.is_some() {
            return Ok(TokenSinkResult::Continue);
        }
        let root = self.sink.root_node();
        let node = self
            .sink
            .create_element(tag.tag().as_str(), tag.attributes())?;
        self.sink.append_child(root, node)?;
        self.html_node = Some(node);
        self.open_elements.push(OpenElement {
            tag: tag.tag().clone(),
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
        let node = self
            .sink
            .create_element(tag.tag().as_str(), tag.attributes())?;
        self.sink.append_child(parent, node)?;
        self.head_node = Some(node);
        self.in_head = true;
        self.open_elements.push(OpenElement {
            tag: tag.tag().clone(),
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
        let node = self
            .sink
            .create_element(tag.tag().as_str(), tag.attributes())?;
        self.sink.append_child(parent, node)?;
        self.body_node = Some(node);
        self.open_elements.push(OpenElement {
            tag: tag.tag().clone(),
            node,
        });
        Ok(TokenSinkResult::Continue)
    }

    fn apply_omission_rules(&mut self, tag: &TagName) {
        if tag.closes_paragraph() {
            self.pop_matching_tag(&TagName::P);
        }
        if tag.closes_list_item() {
            self.pop_matching_tag(&TagName::Li);
        }
    }

    fn pop_matching_tag(&mut self, target_tag: &TagName) {
        if let Some(pos) = self
            .open_elements
            .iter()
            .rposition(|e| &e.tag == target_tag)
        {
            self.open_elements.truncate(pos);
        }
    }

    fn handle_end_tag(&mut self, tag: &TagToken) {
        let tag_name = tag.tag();
        if tag_name.is_head() {
            self.pop_head();
            return;
        }

        if let Some(pos) = self.open_elements.iter().rposition(|e| &e.tag == tag_name) {
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

impl<S: TreeSink + ?Sized> TokenSink for TreeBuilder<'_, S> {
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

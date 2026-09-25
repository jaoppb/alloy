//! HTML5 tree builder implementing the [`TokenSink`] port on top of a [`TreeSink`].

use crate::application::ports::{RawKind, TokenSink, TokenSinkResult, TreeSink};
use crate::domain::attribute::AttributeList;
use crate::domain::error::HtmlError;
use crate::domain::handle::NodeHandle;
use crate::domain::tag::{closes_list_item, closes_paragraph, is_void_tag};
use crate::domain::tag_name::TagName;
use crate::domain::text::Text;
use crate::domain::token::{TagToken, Token};

struct OpenElement {
    tag: TagName,
    handle: NodeHandle,
}

/// Builds a tree structure by consuming tokens and applying HTML5 tree construction rules.
pub struct TreeBuilder<'a> {
    sink: &'a mut dyn TreeSink,
    open_elements: Vec<OpenElement>,
    html_handle: Option<NodeHandle>,
    head_handle: Option<NodeHandle>,
    body_handle: Option<NodeHandle>,
    in_head: bool,
}

impl<'a> TreeBuilder<'a> {
    /// Create a new tree builder using the specified [`TreeSink`].
    #[must_use]
    pub fn new(sink: &'a mut dyn TreeSink) -> Self {
        Self {
            sink,
            open_elements: Vec::new(),
            html_handle: None,
            head_handle: None,
            body_handle: None,
            in_head: false,
        }
    }

    fn current_parent(&self) -> NodeHandle {
        if let Some(open) = self.open_elements.last() {
            return open.handle;
        }
        if let Some(body) = self.body_handle {
            return body;
        }
        if let Some(html) = self.html_handle {
            return html;
        }
        self.sink.root_node()
    }

    fn ensure_html_element(&mut self) -> Result<NodeHandle, HtmlError> {
        if let Some(handle) = self.html_handle {
            return Ok(handle);
        }

        let empty_attributes = AttributeList::new();
        let tag = TagName::new_unchecked("html");
        let handle = self.sink.create_element(tag.clone(), &empty_attributes)?;
        let root = self.sink.root_node();
        self.sink.append_child(root, handle)?;
        self.html_handle = Some(handle);
        self.open_elements.push(OpenElement { tag, handle });
        Ok(handle)
    }

    fn ensure_body_element(&mut self) -> Result<NodeHandle, HtmlError> {
        if let Some(handle) = self.body_handle {
            return Ok(handle);
        }

        self.ensure_html_element()?;
        if self.in_head {
            self.pop_head();
        }

        let html = self.html_handle.unwrap_or_else(|| self.sink.root_node());
        let empty_attributes = AttributeList::new();
        let tag = TagName::new_unchecked("body");
        let handle = self.sink.create_element(tag.clone(), &empty_attributes)?;
        self.sink.append_child(html, handle)?;
        self.body_handle = Some(handle);
        self.open_elements.push(OpenElement { tag, handle });
        Ok(handle)
    }

    fn pop_head(&mut self) {
        while let Some(index) = self.open_elements.iter().rposition(|e| e.tag == "head") {
            self.open_elements.truncate(index);
        }
        self.in_head = false;
    }

    fn handle_start_tag(&mut self, tag: &TagToken) -> Result<TokenSinkResult, HtmlError> {
        let tag_str = tag.name();
        if tag_str == "html" {
            return self.process_html_start_tag(tag);
        }
        if tag_str == "head" {
            return self.process_head_start_tag(tag);
        }
        if tag_str == "body" {
            return self.process_body_start_tag(tag);
        }

        if self.in_head
            && !matches!(
                tag_str,
                "title" | "meta" | "style" | "link" | "script" | "noscript"
            )
        {
            self.pop_head();
        }

        if !self.in_head && self.body_handle.is_none() {
            self.ensure_body_element()?;
        }

        self.apply_omission_rules(tag_str);

        let parent = self.current_parent();
        let handle = self
            .sink
            .create_element(tag.tag_name().clone(), tag.attributes())?;
        self.sink.append_child(parent, handle)?;

        let is_void = is_void_tag(tag_str) || tag.is_self_closing();
        if !is_void {
            self.open_elements.push(OpenElement {
                tag: tag.tag_name().clone(),
                handle,
            });
        }

        if tag_str == "script" {
            return Ok(TokenSinkResult::SwitchToRawText(RawKind::Script));
        }
        if tag_str == "style" {
            return Ok(TokenSinkResult::SwitchToRawText(RawKind::Style));
        }

        Ok(TokenSinkResult::Continue)
    }

    fn process_html_start_tag(&mut self, tag: &TagToken) -> Result<TokenSinkResult, HtmlError> {
        if self.html_handle.is_some() {
            return Ok(TokenSinkResult::Continue);
        }
        let root = self.sink.root_node();
        let handle = self
            .sink
            .create_element(tag.tag_name().clone(), tag.attributes())?;
        self.sink.append_child(root, handle)?;
        self.html_handle = Some(handle);
        self.open_elements.push(OpenElement {
            tag: tag.tag_name().clone(),
            handle,
        });
        Ok(TokenSinkResult::Continue)
    }

    fn process_head_start_tag(&mut self, tag: &TagToken) -> Result<TokenSinkResult, HtmlError> {
        self.ensure_html_element()?;
        if self.head_handle.is_some() {
            return Ok(TokenSinkResult::Continue);
        }
        let parent = self.current_parent();
        let handle = self
            .sink
            .create_element(tag.tag_name().clone(), tag.attributes())?;
        self.sink.append_child(parent, handle)?;
        self.head_handle = Some(handle);
        self.in_head = true;
        self.open_elements.push(OpenElement {
            tag: tag.tag_name().clone(),
            handle,
        });
        Ok(TokenSinkResult::Continue)
    }

    fn process_body_start_tag(&mut self, tag: &TagToken) -> Result<TokenSinkResult, HtmlError> {
        self.ensure_html_element()?;
        if self.in_head {
            self.pop_head();
        }
        if self.body_handle.is_some() {
            return Ok(TokenSinkResult::Continue);
        }
        let parent = self.html_handle.unwrap_or_else(|| self.sink.root_node());
        let handle = self
            .sink
            .create_element(tag.tag_name().clone(), tag.attributes())?;
        self.sink.append_child(parent, handle)?;
        self.body_handle = Some(handle);
        self.open_elements.push(OpenElement {
            tag: tag.tag_name().clone(),
            handle,
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

    fn handle_character(&mut self, text: &Text) -> Result<(), HtmlError> {
        if text.is_empty() {
            return Ok(());
        }

        let is_all_whitespace = text.as_str().chars().all(char::is_whitespace);
        if is_all_whitespace && self.body_handle.is_none() && !self.in_head {
            return Ok(());
        }

        if !self.in_head && self.body_handle.is_none() {
            self.ensure_body_element()?;
        }

        let parent = self.current_parent();
        let handle = self.sink.create_text(text)?;
        self.sink.append_child(parent, handle)?;
        Ok(())
    }

    fn handle_comment(&mut self, text: &Text) -> Result<(), HtmlError> {
        let parent = self.current_parent();
        let handle = self.sink.create_comment(text)?;
        self.sink.append_child(parent, handle)?;
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
            Token::Character(ref text) => {
                self.handle_character(text)?;
                Ok(TokenSinkResult::Continue)
            }
            Token::Comment(ref text) => {
                self.handle_comment(text)?;
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

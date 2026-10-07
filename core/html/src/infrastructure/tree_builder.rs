//! HTML5 tree builder implementing the [`TokenSink`] port on top of a [`TreeSink`].

use crate::application::ports::{RawKind, TokenSink, TokenSinkResult, TreeSink};
use crate::domain::attribute::AttributeList;
use crate::domain::diagnostic::{ParseDiagnostic, ParseErrorCode};
use crate::domain::error::HtmlError;
use crate::domain::handle::NodeHandle;
use crate::domain::location::SourceLocation;
use crate::domain::tag::TagName;
use crate::domain::text::Text;
use crate::domain::token::{DoctypeToken, TagToken, Token};

struct OpenElement {
    tag: TagName,
    handle: NodeHandle,
}

/// Builds a tree structure by consuming tokens and applying HTML5 tree construction rules.
pub struct TreeBuilder<'a, S: TreeSink + ?Sized> {
    sink: &'a mut S,
    open_elements: Vec<OpenElement>,
    html_handle: Option<NodeHandle>,
    head_handle: Option<NodeHandle>,
    body_handle: Option<NodeHandle>,
    in_head: bool,
}

impl<'a, S: TreeSink + ?Sized> TreeBuilder<'a, S> {
    /// Create a new tree builder using the specified [`TreeSink`].
    #[must_use]
    pub const fn new(sink: &'a mut S) -> Self {
        Self {
            sink,
            open_elements: Vec::new(),
            html_handle: None,
            head_handle: None,
            body_handle: None,
            in_head: false,
        }
    }

    fn report(&mut self, code: ParseErrorCode, location: SourceLocation) {
        self.sink.parse_error(ParseDiagnostic::new(code, location));
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
        let tag = TagName::Html;
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
        let tag = TagName::Body;
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

        self.apply_omission_rules(tag.tag_name(), tag.location());

        let parent = self.current_parent();
        let handle = self
            .sink
            .create_element(tag.tag_name().clone(), tag.attributes())?;
        self.sink.append_child(parent, handle)?;

        let is_void = tag.tag_name().is_void() || tag.is_self_closing();
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
        if let Some(existing) = self.html_handle {
            return self.merge_repeated_start_tag(
                existing,
                ParseErrorCode::UnexpectedHtmlStartTag,
                tag,
            );
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
            self.report(ParseErrorCode::UnexpectedHeadStartTag, tag.location());
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
        if let Some(existing) = self.body_handle {
            return self.merge_repeated_start_tag(
                existing,
                ParseErrorCode::UnexpectedBodyStartTag,
                tag,
            );
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

    /// A repeated `<html>`/`<body>`: reported, and its attributes are merged onto the existing
    /// element (WHATWG §13.2.6.4.7) instead of being dropped.
    fn merge_repeated_start_tag(
        &mut self,
        existing: NodeHandle,
        code: ParseErrorCode,
        tag: &TagToken,
    ) -> Result<TokenSinkResult, HtmlError> {
        self.report(code, tag.location());
        if !tag.attributes().is_empty() {
            self.sink
                .add_attributes_if_missing(existing, tag.attributes())?;
        }
        Ok(TokenSinkResult::Continue)
    }

    fn apply_omission_rules(&mut self, tag_name: &TagName, location: SourceLocation) {
        if tag_name.closes_paragraph() {
            self.pop_matching_tag("p", ParseErrorCode::ElementClosedImplicitly, location);
        }
        if tag_name.closes_list_item() {
            self.close_list_item(location);
        }
        if tag_name.closes_definition_item() {
            self.close_definition_item(location);
        }
    }

    fn find_scoped_omission_target(&self, is_target: impl Fn(&TagName) -> bool) -> Option<usize> {
        for (index, element) in self.open_elements.iter().enumerate().rev() {
            if is_target(&element.tag) {
                return Some(index);
            }
            if element.tag.is_list_item_scope_boundary() {
                return None;
            }
        }
        None
    }

    fn close_list_item(&mut self, location: SourceLocation) {
        let target = self.find_scoped_omission_target(|tag| *tag == TagName::Li);
        if let Some(position) = target {
            self.close_through(position, ParseErrorCode::ElementClosedImplicitly, location);
        }
    }

    fn close_definition_item(&mut self, location: SourceLocation) {
        let target = self.find_scoped_omission_target(TagName::closes_definition_item);
        if let Some(position) = target {
            self.close_through(position, ParseErrorCode::ElementClosedImplicitly, location);
        }
    }

    fn pop_matching_tag(
        &mut self,
        target_tag: &str,
        code: ParseErrorCode,
        location: SourceLocation,
    ) {
        if let Some(position) = self.open_elements.iter().rposition(|e| e.tag == target_tag) {
            self.close_through(position, code, location);
        }
    }

    /// Closes the element at `position` and everything above it. Anything above it that the spec
    /// does not let an end tag imply-close is reported once (WHATWG "generate implied end tags").
    fn close_through(&mut self, position: usize, code: ParseErrorCode, location: SourceLocation) {
        let closes_non_implied = self
            .open_elements
            .iter()
            .skip(position.saturating_add(1))
            .any(|open| !open.tag.is_implied_end_tag());
        if closes_non_implied {
            self.report(code, location);
        }
        self.open_elements.truncate(position);
    }

    fn handle_end_tag(&mut self, tag: &TagToken) {
        let name = tag.name();
        let location = tag.location();
        if name == "head" && self.in_head {
            self.pop_head();
            return;
        }

        let matched = self.open_elements.iter().rposition(|e| e.tag == name);
        let Some(position) = matched else {
            self.report(ParseErrorCode::StrayEndTag, location);
            return;
        };
        self.close_through(
            position,
            ParseErrorCode::EndTagDoesNotMatchCurrentNode,
            location,
        );
    }

    /// The doctype is consumed by design: the DOM has no `DocumentType` node and no quirks mode
    /// (issue #36 non-goal). A quirks-forcing doctype is still made visible.
    fn handle_doctype(&mut self, doctype: &DoctypeToken) {
        if doctype.force_quirks() {
            self.report(ParseErrorCode::QuirksModeDoctype, doctype.location());
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

impl<S: TreeSink + ?Sized> TokenSink for TreeBuilder<'_, S> {
    fn process_token(&mut self, token: Token) -> Result<TokenSinkResult, HtmlError> {
        match token {
            Token::StartTag(ref tag) => self.handle_start_tag(tag),
            Token::EndTag(ref tag) => {
                self.handle_end_tag(tag);
                Ok(TokenSinkResult::Continue)
            }
            Token::ParseError(diagnostic) => {
                self.sink.parse_error(diagnostic);
                Ok(TokenSinkResult::Continue)
            }
            Token::Doctype(ref doctype) => {
                self.handle_doctype(doctype);
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
            Token::EndOfFile => Ok(TokenSinkResult::Continue),
        }
    }

    fn finish(&mut self) -> Result<(), HtmlError> {
        self.open_elements.clear();
        Ok(())
    }
}

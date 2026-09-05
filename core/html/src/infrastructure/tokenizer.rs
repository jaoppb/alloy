//! HTML5 tokenizer state machine implementing the WHATWG HTML §13.2.5 standard.

use crate::application::ports::{RawKind, TokenSink, TokenSinkResult};
use crate::domain::error::HtmlError;
use crate::domain::token::{AttributeEntry, AttributeList, DoctypeToken, TagToken, Token};

/// Internal tokenizer state machine states.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    Data,
    TagOpen,
    EndTagOpen,
    TagName,
    EndTagName,
    AfterEndTagName,
    BeforeAttributeName,
    AttributeName,
    AfterAttributeName,
    BeforeAttributeValue,
    AttributeValueDoubleQuoted,
    AttributeValueSingleQuoted,
    AttributeValueUnquoted,
    AfterAttributeValueQuoted,
    SelfClosingStartTag,
    MarkupDeclarationOpen,
    Comment,
    BogusComment,
    Doctype,
    RawText(RawKind),
}

/// A streaming HTML5 tokenizer.
pub struct Tokenizer<'a> {
    chars: core::str::Chars<'a>,
    state: State,
    buffer: String,
    current_tag_name: String,
    current_attributes: AttributeList,
    current_attr_name: String,
    current_attr_value: String,
    is_self_closing: bool,
    pending_token: Option<Token>,
}

impl<'a> Tokenizer<'a> {
    /// Create a new tokenizer over the UTF-8 HTML string slice.
    #[must_use]
    pub fn new(input: &'a str) -> Self {
        Self {
            chars: input.chars(),
            state: State::Data,
            buffer: String::new(),
            current_tag_name: String::new(),
            current_attributes: AttributeList::new(),
            current_attr_name: String::new(),
            current_attr_value: String::new(),
            is_self_closing: false,
            pending_token: None,
        }
    }

    /// Run the tokenizer to completion, pumping tokens into `sink`.
    pub fn run(mut self, sink: &mut dyn TokenSink) -> Result<(), HtmlError> {
        let mut done = false;
        while !done {
            let token = self.pump_next_token()?;
            done = token == Token::EndOfFile;
            let result = sink.process_token(token)?;
            self.handle_sink_result(result);
        }
        sink.finish()
    }

    const fn handle_sink_result(&mut self, result: TokenSinkResult) {
        match result {
            TokenSinkResult::Continue => {}
            TokenSinkResult::SwitchToRawText(kind) => {
                self.state = State::RawText(kind);
            }
        }
    }

    fn pump_next_token(&mut self) -> Result<Token, HtmlError> {
        if let Some(token) = self.pending_token.take() {
            return Ok(token);
        }

        loop {
            match self.state {
                State::Data => {
                    if let Some(token) = self.handle_data_state() {
                        return Ok(token);
                    }
                }
                State::TagOpen => self.handle_tag_open_state(),
                State::EndTagOpen => self.handle_end_tag_open_state(),
                State::TagName => {
                    if let Some(token) = self.handle_tag_name_state()? {
                        return Ok(token);
                    }
                }
                State::EndTagName => {
                    if let Some(token) = self.handle_end_tag_name_state()? {
                        return Ok(token);
                    }
                }
                State::AfterEndTagName => {
                    if let Some(token) = self.handle_after_end_tag_name_state()? {
                        return Ok(token);
                    }
                }
                State::BeforeAttributeName => {
                    if let Some(token) = self.handle_before_attr_name_state()? {
                        return Ok(token);
                    }
                }
                State::AttributeName => {
                    if let Some(token) = self.handle_attr_name_state()? {
                        return Ok(token);
                    }
                }
                State::AfterAttributeName => {
                    if let Some(token) = self.handle_after_attr_name_state()? {
                        return Ok(token);
                    }
                }
                State::BeforeAttributeValue => {
                    if let Some(token) = self.handle_before_attr_value_state()? {
                        return Ok(token);
                    }
                }
                State::AttributeValueDoubleQuoted => {
                    self.handle_attr_value_quoted('"');
                }
                State::AttributeValueSingleQuoted => {
                    self.handle_attr_value_quoted('\'');
                }
                State::AttributeValueUnquoted => {
                    if let Some(token) = self.handle_attr_value_unquoted()? {
                        return Ok(token);
                    }
                }
                State::AfterAttributeValueQuoted => {
                    if let Some(token) = self.handle_after_attr_value_quoted()? {
                        return Ok(token);
                    }
                }
                State::SelfClosingStartTag => {
                    if let Some(token) = self.handle_self_closing_state()? {
                        return Ok(token);
                    }
                }
                State::MarkupDeclarationOpen => self.handle_markup_declaration_state(),
                State::Comment => return Ok(self.handle_comment_state()),
                State::BogusComment => return Ok(self.handle_bogus_comment_state()),
                State::Doctype => return Ok(self.handle_doctype_state()),
                State::RawText(kind) => return Ok(self.consume_rawtext(kind)),
            }
        }
    }

    fn handle_data_state(&mut self) -> Option<Token> {
        let mut text = String::new();
        while let Some(ch) = self.chars.next() {
            if ch == '<' {
                self.state = State::TagOpen;
                if text.is_empty() {
                    return None;
                }
                return Some(Token::Character(text));
            }
            if ch == '&' {
                Self::consume_character_reference(&mut self.chars, &mut text);
                continue;
            }
            text.push(ch);
        }
        if !text.is_empty() {
            return Some(Token::Character(text));
        }
        Some(Token::EndOfFile)
    }

    fn consume_character_reference(chars: &mut core::str::Chars<'a>, text: &mut String) {
        let mut entity_name = String::new();
        let clone_chars = chars.clone();

        for (idx, ch) in clone_chars.enumerate() {
            if ch == ';' {
                if let Some(resolved) = resolve_entity(&entity_name) {
                    text.push_str(&resolved);
                    for _ in 0..=idx {
                        chars.next();
                    }
                    return;
                }
                break;
            }
            if !ch.is_alphanumeric() && ch != '#' {
                break;
            }
            entity_name.push(ch);
            if entity_name.len() > 16 {
                break;
            }
        }
        text.push('&');
    }

    fn handle_tag_open_state(&mut self) {
        let Some(ch) = self.chars.next() else {
            self.state = State::Data;
            return;
        };

        if ch == '!' {
            self.state = State::MarkupDeclarationOpen;
            return;
        }
        if ch == '/' {
            self.state = State::EndTagOpen;
            return;
        }
        if ch.is_ascii_alphabetic() {
            self.state = State::TagName;
            self.current_tag_name = ch.to_ascii_lowercase().to_string();
            self.current_attributes = AttributeList::new();
            self.is_self_closing = false;
            return;
        }
        if ch == '?' {
            self.state = State::BogusComment;
            self.buffer.clear();
            return;
        }

        self.state = State::Data;
    }

    fn handle_end_tag_open_state(&mut self) {
        let Some(ch) = self.chars.next() else {
            self.state = State::Data;
            return;
        };

        if ch.is_ascii_alphabetic() {
            self.state = State::EndTagName;
            self.current_tag_name = ch.to_ascii_lowercase().to_string();
            return;
        }
        if ch == '>' {
            self.state = State::Data;
            return;
        }

        self.state = State::BogusComment;
        self.buffer.clear();
        self.buffer.push(ch);
    }

    fn handle_tag_name_state(&mut self) -> Result<Option<Token>, HtmlError> {
        while let Some(ch) = self.chars.next() {
            if ch.is_whitespace() {
                self.state = State::BeforeAttributeName;
                return Ok(None);
            }
            if ch == '/' {
                self.state = State::SelfClosingStartTag;
                return Ok(None);
            }
            if ch == '>' {
                self.state = State::Data;
                return Ok(Some(self.build_start_tag()?));
            }
            self.current_tag_name.push(ch.to_ascii_lowercase());
        }
        self.state = State::Data;
        Ok(Some(self.build_start_tag()?))
    }

    fn handle_end_tag_name_state(&mut self) -> Result<Option<Token>, HtmlError> {
        while let Some(ch) = self.chars.next() {
            if ch.is_whitespace() {
                self.state = State::AfterEndTagName;
                return Ok(None);
            }
            if ch == '>' {
                self.state = State::Data;
                return Ok(Some(self.build_end_tag()?));
            }
            self.current_tag_name.push(ch.to_ascii_lowercase());
        }
        self.state = State::Data;
        Ok(Some(self.build_end_tag()?))
    }

    fn handle_after_end_tag_name_state(&mut self) -> Result<Option<Token>, HtmlError> {
        for ch in self.chars.by_ref() {
            if ch.is_whitespace() {
                continue;
            }
            if ch == '>' {
                self.state = State::Data;
                return Ok(Some(self.build_end_tag()?));
            }
        }
        self.state = State::Data;
        Ok(Some(self.build_end_tag()?))
    }

    fn handle_before_attr_name_state(&mut self) -> Result<Option<Token>, HtmlError> {
        while let Some(ch) = self.chars.next() {
            if ch.is_whitespace() {
                continue;
            }
            if ch == '/' {
                self.state = State::SelfClosingStartTag;
                return Ok(None);
            }
            if ch == '>' {
                self.state = State::Data;
                return Ok(Some(self.build_start_tag()?));
            }
            self.state = State::AttributeName;
            self.current_attr_name = ch.to_ascii_lowercase().to_string();
            self.current_attr_value.clear();
            return Ok(None);
        }
        self.state = State::Data;
        Ok(Some(self.build_start_tag()?))
    }

    fn handle_attr_name_state(&mut self) -> Result<Option<Token>, HtmlError> {
        while let Some(ch) = self.chars.next() {
            if ch.is_whitespace() {
                self.state = State::AfterAttributeName;
                return Ok(None);
            }
            if ch == '=' {
                self.state = State::BeforeAttributeValue;
                return Ok(None);
            }
            if ch == '/' {
                self.commit_attribute()?;
                self.state = State::SelfClosingStartTag;
                return Ok(None);
            }
            if ch == '>' {
                self.commit_attribute()?;
                self.state = State::Data;
                return Ok(Some(self.build_start_tag()?));
            }
            self.current_attr_name.push(ch.to_ascii_lowercase());
        }
        self.commit_attribute()?;
        self.state = State::Data;
        Ok(Some(self.build_start_tag()?))
    }

    fn handle_after_attr_name_state(&mut self) -> Result<Option<Token>, HtmlError> {
        while let Some(ch) = self.chars.next() {
            if ch.is_whitespace() {
                continue;
            }
            if ch == '=' {
                self.state = State::BeforeAttributeValue;
                return Ok(None);
            }
            if ch == '/' {
                self.commit_attribute()?;
                self.state = State::SelfClosingStartTag;
                return Ok(None);
            }
            if ch == '>' {
                self.commit_attribute()?;
                self.state = State::Data;
                return Ok(Some(self.build_start_tag()?));
            }
            self.commit_attribute()?;
            self.state = State::AttributeName;
            self.current_attr_name = ch.to_ascii_lowercase().to_string();
            self.current_attr_value.clear();
            return Ok(None);
        }
        self.commit_attribute()?;
        self.state = State::Data;
        Ok(Some(self.build_start_tag()?))
    }

    fn handle_before_attr_value_state(&mut self) -> Result<Option<Token>, HtmlError> {
        while let Some(ch) = self.chars.next() {
            if ch.is_whitespace() {
                continue;
            }
            if ch == '"' {
                self.state = State::AttributeValueDoubleQuoted;
                self.current_attr_value.clear();
                return Ok(None);
            }
            if ch == '\'' {
                self.state = State::AttributeValueSingleQuoted;
                self.current_attr_value.clear();
                return Ok(None);
            }
            if ch == '>' {
                self.commit_attribute()?;
                self.state = State::Data;
                return Ok(Some(self.build_start_tag()?));
            }
            self.state = State::AttributeValueUnquoted;
            self.current_attr_value = ch.to_string();
            return Ok(None);
        }
        self.commit_attribute()?;
        self.state = State::Data;
        Ok(Some(self.build_start_tag()?))
    }

    fn handle_attr_value_quoted(&mut self, quote: char) {
        while let Some(ch) = self.chars.next() {
            if ch == quote {
                self.state = State::AfterAttributeValueQuoted;
                return;
            }
            if ch == '&' {
                Self::consume_character_reference(&mut self.chars, &mut self.current_attr_value);
                continue;
            }
            self.current_attr_value.push(ch);
        }
        self.state = State::AfterAttributeValueQuoted;
    }

    fn handle_attr_value_unquoted(&mut self) -> Result<Option<Token>, HtmlError> {
        while let Some(ch) = self.chars.next() {
            if ch.is_whitespace() {
                self.commit_attribute()?;
                self.state = State::BeforeAttributeName;
                return Ok(None);
            }
            if ch == '/' {
                self.commit_attribute()?;
                self.state = State::SelfClosingStartTag;
                return Ok(None);
            }
            if ch == '>' {
                self.commit_attribute()?;
                self.state = State::Data;
                return Ok(Some(self.build_start_tag()?));
            }
            self.current_attr_value.push(ch);
        }
        self.commit_attribute()?;
        self.state = State::Data;
        Ok(Some(self.build_start_tag()?))
    }

    fn handle_after_attr_value_quoted(&mut self) -> Result<Option<Token>, HtmlError> {
        self.commit_attribute()?;
        let Some(ch) = self.chars.next() else {
            self.state = State::Data;
            return Ok(Some(self.build_start_tag()?));
        };
        if ch.is_whitespace() {
            self.state = State::BeforeAttributeName;
            return Ok(None);
        }
        if ch == '/' {
            self.state = State::SelfClosingStartTag;
            return Ok(None);
        }
        if ch == '>' {
            self.state = State::Data;
            return Ok(Some(self.build_start_tag()?));
        }
        self.state = State::BeforeAttributeName;
        Ok(None)
    }

    fn handle_self_closing_state(&mut self) -> Result<Option<Token>, HtmlError> {
        while let Some(ch) = self.chars.next() {
            if ch == '>' {
                self.is_self_closing = true;
                self.state = State::Data;
                return Ok(Some(self.build_start_tag()?));
            }
            if ch.is_whitespace() {
                continue;
            }
            self.state = State::BeforeAttributeName;
            return Ok(None);
        }
        self.state = State::Data;
        Ok(Some(self.build_start_tag()?))
    }

    fn handle_markup_declaration_state(&mut self) {
        let remaining: String = self.chars.clone().take(7).collect();
        let upper = remaining.to_ascii_uppercase();

        if remaining.starts_with("--") {
            self.chars.next();
            self.chars.next();
            self.state = State::Comment;
            self.buffer.clear();
            return;
        }
        if upper.starts_with("DOCTYPE") {
            for _ in 0..7 {
                self.chars.next();
            }
            self.state = State::Doctype;
            self.buffer.clear();
            return;
        }

        self.state = State::BogusComment;
        self.buffer.clear();
    }

    fn handle_comment_state(&mut self) -> Token {
        let mut dashes: usize = 0;
        for ch in self.chars.by_ref() {
            if ch == '-' {
                dashes = dashes.saturating_add(1);
                continue;
            }
            if ch == '>' && dashes >= 2 {
                let comment = self.buffer.clone();
                self.buffer.clear();
                self.state = State::Data;
                return Token::Comment(comment);
            }
            for _ in 0..dashes {
                self.buffer.push('-');
            }
            dashes = 0;
            self.buffer.push(ch);
        }
        let comment = self.buffer.clone();
        self.buffer.clear();
        self.state = State::Data;
        Token::Comment(comment)
    }

    fn handle_bogus_comment_state(&mut self) -> Token {
        for ch in self.chars.by_ref() {
            if ch == '>' {
                break;
            }
            self.buffer.push(ch);
        }
        let comment = self.buffer.clone();
        self.buffer.clear();
        self.state = State::Data;
        Token::Comment(comment)
    }

    fn handle_doctype_state(&mut self) -> Token {
        for ch in self.chars.by_ref() {
            if ch == '>' {
                break;
            }
            self.buffer.push(ch);
        }
        let content = self.buffer.trim();
        let mut words = content.split_whitespace();
        let name = words.next().map(ToString::to_string);
        self.buffer.clear();
        self.state = State::Data;
        Token::Doctype(DoctypeToken::new(name, None, None, false))
    }

    fn consume_rawtext(&mut self, kind: RawKind) -> Token {
        let (end_marker, tag_name) = match kind {
            RawKind::Script => ("</script", "script"),
            RawKind::Style => ("</style", "style"),
        };
        let prefix_len = end_marker.len().saturating_sub(1);

        let mut content = String::new();
        while let Some(ch) = self.chars.next() {
            if ch == '<' {
                let peek: String = core::iter::once('<')
                    .chain(self.chars.clone().take(prefix_len))
                    .collect();
                if peek.eq_ignore_ascii_case(end_marker) {
                    for _ in 0..prefix_len {
                        self.chars.next();
                    }
                    for closing_ch in self.chars.by_ref() {
                        if closing_ch == '>' {
                            break;
                        }
                    }
                    self.state = State::Data;
                    if let Ok(end_tag) = TagToken::new(tag_name, AttributeList::new(), false) {
                        self.pending_token = Some(Token::EndTag(end_tag));
                    }
                    return Token::Character(content);
                }
            }
            content.push(ch);
        }

        self.state = State::Data;
        Token::Character(content)
    }

    fn commit_attribute(&mut self) -> Result<(), HtmlError> {
        if self.current_attr_name.is_empty() {
            return Ok(());
        }
        let entry = AttributeEntry::new(
            core::mem::take(&mut self.current_attr_name),
            core::mem::take(&mut self.current_attr_value),
        )?;
        self.current_attributes.push(entry);
        Ok(())
    }

    fn build_start_tag(&mut self) -> Result<Token, HtmlError> {
        let tag = TagToken::new(
            core::mem::take(&mut self.current_tag_name),
            core::mem::take(&mut self.current_attributes),
            self.is_self_closing,
        )?;
        self.is_self_closing = false;
        Ok(Token::StartTag(tag))
    }

    fn build_end_tag(&mut self) -> Result<Token, HtmlError> {
        let tag = TagToken::new(
            core::mem::take(&mut self.current_tag_name),
            AttributeList::new(),
            false,
        )?;
        Ok(Token::EndTag(tag))
    }
}

fn resolve_entity(name: &str) -> Option<String> {
    if let Some(entity) = dom::HtmlEntity::from_name(name) {
        return Some(entity.as_char().to_string());
    }
    if let Some(stripped) = name.strip_prefix("#x").or_else(|| name.strip_prefix("#X")) {
        let code = u32::from_str_radix(stripped, 16).ok()?;
        return char::from_u32(code).map(String::from);
    }
    if let Some(stripped) = name.strip_prefix('#') {
        let code = stripped.parse::<u32>().ok()?;
        return char::from_u32(code).map(String::from);
    }
    None
}

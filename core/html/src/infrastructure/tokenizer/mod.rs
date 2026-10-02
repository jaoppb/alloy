//! HTML5 streaming tokenizer complying with WHATWG HTML §13.2.5.

pub mod attribute_state;
pub mod cursor;
pub mod doctype;
pub mod entity;
pub mod rawtext;
pub mod state;
pub mod tag_state;

use crate::application::ports::{TokenSink, TokenSinkResult};
use crate::domain::attribute::AttributeList;
use crate::domain::error::HtmlError;
use crate::domain::text::Text;
use crate::domain::token::Token;
use attribute_state::{
    handle_after_attribute_name, handle_after_attribute_value_quoted, handle_attribute_name,
    handle_attribute_value_quoted, handle_attribute_value_unquoted, handle_before_attribute_name,
    handle_before_attribute_value,
};
use cursor::Cursor;
use doctype::handle_doctype;
use entity::consume_character_reference;
use rawtext::consume_rawtext;
use state::State;
use std::borrow::Cow;
use tag_state::{
    handle_after_end_tag_name, handle_end_tag_name, handle_end_tag_open, handle_self_closing,
    handle_tag_name, handle_tag_open,
};

/// Result of a resumable tokenizer execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TokenizerRunResult {
    /// Tokenizer processed all input to completion.
    Completed,
    /// Tokenizer suspended execution at specified source offset.
    Suspended {
        /// Byte offset where execution was suspended.
        resume_at: usize,
    },
}

/// A streaming HTML5 tokenizer.
pub struct Tokenizer<'a> {
    cursor: Cursor<'a>,
    state: State,
    buffer: String,
    current_tag_name: String,
    current_attributes: AttributeList,
    current_attribute_name: String,
    current_attribute_value: String,
    is_self_closing: bool,
    pending_token: Option<Token>,
}

impl<'a> Tokenizer<'a> {
    /// Create a new tokenizer over the UTF-8 HTML string slice.
    #[must_use]
    pub const fn new(input: &'a str) -> Self {
        Self {
            cursor: Cursor::new(input),
            state: State::Data,
            buffer: String::new(),
            current_tag_name: String::new(),
            current_attributes: AttributeList::new(),
            current_attribute_name: String::new(),
            current_attribute_value: String::new(),
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
            self.handle_sink_result(&result);
        }
        sink.finish()
    }

    /// Run the tokenizer with support for suspension (PRD-008 §3.4).
    pub fn run_resumable(
        &mut self,
        sink: &mut dyn TokenSink,
    ) -> Result<TokenizerRunResult, HtmlError> {
        loop {
            let token = self.pump_next_token()?;
            let is_eof = token == Token::EndOfFile;
            let result = sink.process_token(token)?;
            match result {
                TokenSinkResult::Suspend => {
                    return Ok(TokenizerRunResult::Suspended {
                        resume_at: self.cursor.byte_offset(),
                    });
                }
                TokenSinkResult::Continue
                | TokenSinkResult::SwitchToRawText(_)
                | TokenSinkResult::Script(_) => {
                    self.handle_sink_result(&result);
                }
            }
            if is_eof {
                sink.finish()?;
                return Ok(TokenizerRunResult::Completed);
            }
        }
    }

    /// Resume tokenization with injected input (e.g. from `document.write`).
    pub fn resume(
        &mut self,
        extra_input: &str,
        sink: &mut dyn TokenSink,
    ) -> Result<TokenizerRunResult, HtmlError> {
        if !extra_input.is_empty() {
            let mut combined = extra_input.to_string();
            combined.push_str(self.cursor.remaining());
            self.cursor = Cursor::from_cow(
                Cow::Owned(combined),
                self.cursor.location().line(),
                self.cursor.location().column(),
            );
        }
        self.run_resumable(sink)
    }

    const fn handle_sink_result(&mut self, result: &TokenSinkResult) {
        match result {
            TokenSinkResult::SwitchToRawText(kind) => {
                self.state = State::RawText(*kind);
            }
            TokenSinkResult::Continue | TokenSinkResult::Suspend | TokenSinkResult::Script(_) => {}
        }
    }

    /// Pumps and returns the next token from the stream.
    pub fn pump_next_token(&mut self) -> Result<Token, HtmlError> {
        if let Some(token) = self.pending_token.take() {
            return Ok(token);
        }

        loop {
            if let Some(token) = self.step_state()? {
                return Ok(token);
            }
        }
    }

    fn step_state(&mut self) -> Result<Option<Token>, HtmlError> {
        match self.state {
            State::Data => Ok(self.handle_data_state()),
            State::TagOpen
            | State::EndTagOpen
            | State::TagName
            | State::EndTagName
            | State::AfterEndTagName
            | State::SelfClosingStartTag => self.step_tag_state(),
            State::BeforeAttributeName
            | State::AttributeName
            | State::AfterAttributeName
            | State::BeforeAttributeValue
            | State::AttributeValueDoubleQuoted
            | State::AttributeValueSingleQuoted
            | State::AttributeValueUnquoted
            | State::AfterAttributeValueQuoted => self.step_attribute_state(),
            State::MarkupDeclarationOpen
            | State::Comment
            | State::BogusComment
            | State::Doctype
            | State::RawText(_) => Ok(self.step_markup_state()),
        }
    }

    fn step_tag_state(&mut self) -> Result<Option<Token>, HtmlError> {
        match self.state {
            State::TagOpen => Ok(handle_tag_open(
                &mut self.cursor,
                &mut self.state,
                &mut self.current_tag_name,
                &mut self.current_attributes,
                &mut self.is_self_closing,
            )),
            State::EndTagOpen => {
                handle_end_tag_open(
                    &mut self.cursor,
                    &mut self.state,
                    &mut self.current_tag_name,
                    &mut self.buffer,
                );
                Ok(None)
            }
            State::TagName => handle_tag_name(
                &mut self.cursor,
                &mut self.state,
                &mut self.current_tag_name,
                &mut self.current_attributes,
                &mut self.is_self_closing,
            ),
            State::EndTagName => handle_end_tag_name(
                &mut self.cursor,
                &mut self.state,
                &mut self.current_tag_name,
            ),
            State::AfterEndTagName => handle_after_end_tag_name(
                &mut self.cursor,
                &mut self.state,
                &mut self.current_tag_name,
            ),
            State::SelfClosingStartTag => handle_self_closing(
                &mut self.cursor,
                &mut self.state,
                &mut self.current_tag_name,
                &mut self.current_attributes,
                &mut self.is_self_closing,
            ),
            _ => Ok(None),
        }
    }

    fn step_attribute_state(&mut self) -> Result<Option<Token>, HtmlError> {
        match self.state {
            State::BeforeAttributeName => handle_before_attribute_name(
                &mut self.cursor,
                &mut self.state,
                &mut self.current_attribute_name,
                &mut self.current_attribute_value,
                &mut self.current_tag_name,
                &mut self.current_attributes,
                &mut self.is_self_closing,
            ),
            State::AttributeName => handle_attribute_name(
                &mut self.cursor,
                &mut self.state,
                &mut self.current_attribute_name,
                &mut self.current_attribute_value,
                &mut self.current_attributes,
                &mut self.current_tag_name,
                &mut self.is_self_closing,
            ),
            State::AfterAttributeName => handle_after_attribute_name(
                &mut self.cursor,
                &mut self.state,
                &mut self.current_attribute_name,
                &mut self.current_attribute_value,
                &mut self.current_attributes,
                &mut self.current_tag_name,
                &mut self.is_self_closing,
            ),
            State::BeforeAttributeValue => handle_before_attribute_value(
                &mut self.cursor,
                &mut self.state,
                &mut self.current_attribute_value,
                &mut self.current_attribute_name,
                &mut self.current_attributes,
                &mut self.current_tag_name,
                &mut self.is_self_closing,
            ),
            State::AttributeValueDoubleQuoted => {
                handle_attribute_value_quoted(
                    &mut self.cursor,
                    &mut self.state,
                    '"',
                    &mut self.current_attribute_value,
                );
                Ok(None)
            }
            State::AttributeValueSingleQuoted => {
                handle_attribute_value_quoted(
                    &mut self.cursor,
                    &mut self.state,
                    '\'',
                    &mut self.current_attribute_value,
                );
                Ok(None)
            }
            State::AttributeValueUnquoted => handle_attribute_value_unquoted(
                &mut self.cursor,
                &mut self.state,
                &mut self.current_attribute_name,
                &mut self.current_attribute_value,
                &mut self.current_attributes,
                &mut self.current_tag_name,
                &mut self.is_self_closing,
            ),
            State::AfterAttributeValueQuoted => handle_after_attribute_value_quoted(
                &mut self.cursor,
                &mut self.state,
                &mut self.current_attribute_name,
                &mut self.current_attribute_value,
                &mut self.current_attributes,
                &mut self.current_tag_name,
                &mut self.is_self_closing,
            ),
            _ => Ok(None),
        }
    }

    fn step_markup_state(&mut self) -> Option<Token> {
        match self.state {
            State::MarkupDeclarationOpen => {
                self.handle_markup_declaration_state();
                None
            }
            State::Comment => Some(self.handle_comment_state()),
            State::BogusComment => Some(self.handle_bogus_comment_state()),
            State::Doctype => Some(handle_doctype(&mut self.cursor, &mut self.state)),
            State::RawText(kind) => Some(consume_rawtext(
                &mut self.cursor,
                &mut self.state,
                kind,
                &mut self.pending_token,
            )),
            _ => None,
        }
    }

    fn handle_data_state(&mut self) -> Option<Token> {
        let mut text = String::new();
        while let Some(character) = self.cursor.next_char() {
            if character == '<' {
                self.state = State::TagOpen;
                if text.is_empty() {
                    return None;
                }
                return Some(Token::Character(Text::new(text)));
            }
            if character == '&' {
                consume_character_reference(&mut self.cursor, &mut text);
                continue;
            }
            text.push(character);
        }
        if !text.is_empty() {
            return Some(Token::Character(Text::new(text)));
        }
        Some(Token::EndOfFile)
    }

    fn handle_markup_declaration_state(&mut self) {
        let remaining = self.cursor.remaining();
        if remaining.starts_with("--") {
            self.cursor.next_char();
            self.cursor.next_char();
            self.state = State::Comment;
            self.buffer.clear();
            return;
        }

        let upper = remaining.to_ascii_uppercase();
        if upper.starts_with("DOCTYPE") {
            for _ in 0..7 {
                self.cursor.next_char();
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
        while let Some(character) = self.cursor.next_char() {
            if character == '-' {
                dashes = dashes.saturating_add(1);
                continue;
            }
            if character == '>' && dashes >= 2 {
                let comment = core::mem::take(&mut self.buffer);
                self.state = State::Data;
                return Token::Comment(Text::new(comment));
            }
            for _ in 0..dashes {
                self.buffer.push('-');
            }
            dashes = 0;
            self.buffer.push(character);
        }
        let comment = core::mem::take(&mut self.buffer);
        self.state = State::Data;
        Token::Comment(Text::new(comment))
    }

    fn handle_bogus_comment_state(&mut self) -> Token {
        while let Some(character) = self.cursor.next_char() {
            if character == '>' {
                break;
            }
            self.buffer.push(character);
        }
        let comment = core::mem::take(&mut self.buffer);
        self.state = State::Data;
        Token::Comment(Text::new(comment))
    }
}

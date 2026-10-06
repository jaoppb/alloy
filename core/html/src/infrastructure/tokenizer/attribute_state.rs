//! Handlers for attribute names, values, and quoting states.

use crate::domain::diagnostic::ParseErrorCode;
use crate::domain::token::Token;
use crate::infrastructure::tokenizer::cursor::Cursor;
use crate::infrastructure::tokenizer::entity::{ReferenceContext, consume_character_reference};
use crate::infrastructure::tokenizer::pending_tag::PendingTag;
use crate::infrastructure::tokenizer::state::State;
use crate::infrastructure::tokenizer::tag_state::{emit_tag, eof_in_tag};

/// Commits the pending attribute, then finishes the tag.
fn commit_and_emit(
    cursor: &mut Cursor<'_>,
    state: &mut State,
    tag: &mut PendingTag,
) -> Option<Token> {
    tag.commit_attribute(cursor);
    emit_tag(cursor, state, tag)
}

/// Commits the pending attribute and moves to `SelfClosingStartTag`.
fn commit_and_self_close(cursor: &mut Cursor<'_>, state: &mut State, tag: &mut PendingTag) {
    tag.commit_attribute(cursor);
    *state = State::SelfClosingStartTag;
}

/// Starts an attribute at the character just read.
fn begin_attribute_here(
    cursor: &Cursor<'_>,
    state: &mut State,
    tag: &mut PendingTag,
    character: char,
) {
    *state = State::AttributeName;
    tag.begin_attribute(character, cursor.last_location());
}

/// Processes `BeforeAttributeName` state.
pub fn handle_before_attribute_name(
    cursor: &mut Cursor<'_>,
    state: &mut State,
    tag: &mut PendingTag,
) -> Option<Token> {
    while let Some(character) = cursor.next_char() {
        if character.is_ascii_whitespace() {
            continue;
        }
        if character == '/' {
            *state = State::SelfClosingStartTag;
            return None;
        }
        if character == '>' {
            return emit_tag(cursor, state, tag);
        }
        if character == '=' {
            cursor.report(
                ParseErrorCode::UnexpectedEqualsSignBeforeAttributeName,
                cursor.last_location(),
            );
        }
        begin_attribute_here(cursor, state, tag, character);
        return None;
    }
    eof_in_tag(cursor, state, tag)
}

/// Processes `AttributeName` state.
pub fn handle_attribute_name(
    cursor: &mut Cursor<'_>,
    state: &mut State,
    tag: &mut PendingTag,
) -> Option<Token> {
    while let Some(character) = cursor.next_char() {
        if character.is_ascii_whitespace() {
            *state = State::AfterAttributeName;
            return None;
        }
        if character == '=' {
            *state = State::BeforeAttributeValue;
            return None;
        }
        if character == '/' {
            commit_and_self_close(cursor, state, tag);
            return None;
        }
        if character == '>' {
            return commit_and_emit(cursor, state, tag);
        }
        if matches!(character, '"' | '\'' | '<') {
            // WHATWG §13.2.5.33: reported, and the character stays part of the name.
            cursor.report(
                ParseErrorCode::UnexpectedCharacterInAttributeName,
                cursor.last_location(),
            );
        }
        tag.push_attribute_name_character(character);
    }
    eof_in_tag(cursor, state, tag)
}

/// Processes `AfterAttributeName` state.
pub fn handle_after_attribute_name(
    cursor: &mut Cursor<'_>,
    state: &mut State,
    tag: &mut PendingTag,
) -> Option<Token> {
    while let Some(character) = cursor.next_char() {
        if character.is_ascii_whitespace() {
            continue;
        }
        if character == '=' {
            *state = State::BeforeAttributeValue;
            return None;
        }
        if character == '/' {
            commit_and_self_close(cursor, state, tag);
            return None;
        }
        if character == '>' {
            return commit_and_emit(cursor, state, tag);
        }
        tag.commit_attribute(cursor);
        begin_attribute_here(cursor, state, tag, character);
        return None;
    }
    eof_in_tag(cursor, state, tag)
}

/// Processes `BeforeAttributeValue` state.
pub fn handle_before_attribute_value(
    cursor: &mut Cursor<'_>,
    state: &mut State,
    tag: &mut PendingTag,
) -> Option<Token> {
    while let Some(character) = cursor.next_char() {
        if character.is_ascii_whitespace() {
            continue;
        }
        if character == '"' {
            *state = State::AttributeValueDoubleQuoted;
            tag.start_value();
            return None;
        }
        if character == '\'' {
            *state = State::AttributeValueSingleQuoted;
            tag.start_value();
            return None;
        }
        if character == '>' {
            cursor.report(
                ParseErrorCode::MissingAttributeValue,
                cursor.last_location(),
            );
            return commit_and_emit(cursor, state, tag);
        }
        *state = State::AttributeValueUnquoted;
        tag.start_value();
        tag.push_value_character(character);
        return None;
    }
    eof_in_tag(cursor, state, tag)
}

/// Processes quoted attribute values (`"` or `'`).
pub fn handle_attribute_value_quoted(
    cursor: &mut Cursor<'_>,
    state: &mut State,
    quote: char,
    tag: &mut PendingTag,
) -> Option<Token> {
    while let Some(character) = cursor.next_char() {
        if character == quote {
            *state = State::AfterAttributeValueQuoted;
            return None;
        }
        if character == '&' {
            consume_character_reference(
                cursor,
                tag.value_buffer(),
                ReferenceContext::AttributeValue,
            );
            continue;
        }
        tag.push_value_character(character);
    }
    eof_in_tag(cursor, state, tag)
}

/// Processes `AttributeValueUnquoted` state.
pub fn handle_attribute_value_unquoted(
    cursor: &mut Cursor<'_>,
    state: &mut State,
    tag: &mut PendingTag,
) -> Option<Token> {
    while let Some(character) = cursor.next_char() {
        if character.is_ascii_whitespace() {
            tag.commit_attribute(cursor);
            *state = State::BeforeAttributeName;
            return None;
        }
        if character == '/' {
            commit_and_self_close(cursor, state, tag);
            return None;
        }
        if character == '>' {
            return commit_and_emit(cursor, state, tag);
        }
        if character == '&' {
            consume_character_reference(
                cursor,
                tag.value_buffer(),
                ReferenceContext::AttributeValue,
            );
            continue;
        }
        report_unexpected_unquoted_character(cursor, character);
        tag.push_value_character(character);
    }
    eof_in_tag(cursor, state, tag)
}

/// §13.2.5.38: these characters are kept in an unquoted value but are almost certainly a typo.
fn report_unexpected_unquoted_character(cursor: &mut Cursor<'_>, character: char) {
    if matches!(character, '"' | '\'' | '<' | '=' | '`') {
        cursor.report(
            ParseErrorCode::UnexpectedCharacterInUnquotedAttributeValue,
            cursor.last_location(),
        );
    }
}

/// Processes `AfterAttributeValueQuoted` state.
pub fn handle_after_attribute_value_quoted(
    cursor: &mut Cursor<'_>,
    state: &mut State,
    tag: &mut PendingTag,
) -> Option<Token> {
    tag.commit_attribute(cursor);
    let Some(character) = cursor.next_char() else {
        return eof_in_tag(cursor, state, tag);
    };

    if character.is_ascii_whitespace() {
        *state = State::BeforeAttributeName;
        return None;
    }
    if character == '/' {
        *state = State::SelfClosingStartTag;
        return None;
    }
    if character == '>' {
        return emit_tag(cursor, state, tag);
    }

    cursor.report(
        ParseErrorCode::MissingWhitespaceBetweenAttributes,
        cursor.last_location(),
    );
    *state = State::BeforeAttributeName;
    cursor.reconsume(character);
    None
}

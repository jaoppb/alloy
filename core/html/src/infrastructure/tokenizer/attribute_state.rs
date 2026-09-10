//! Handlers for attribute names, values, and quoting states.

use crate::domain::attribute::{AttributeEntry, AttributeList, AttributeName, AttributeValue};
use crate::domain::error::HtmlError;
use crate::domain::token::Token;
use crate::infrastructure::tokenizer::cursor::Cursor;
use crate::infrastructure::tokenizer::entity::consume_character_reference;
use crate::infrastructure::tokenizer::state::State;
use crate::infrastructure::tokenizer::tag_state::build_start_tag;

/// Commits pending attribute name and value into the attribute list.
pub fn commit_attribute(
    cursor: &Cursor<'_>,
    attributes: &mut AttributeList,
    attribute_name: &mut String,
    attribute_value: &mut String,
) -> Result<(), HtmlError> {
    if attribute_name.is_empty() {
        return Ok(());
    }
    let name_str = core::mem::take(attribute_name);
    let value_str = core::mem::take(attribute_value);
    let name = AttributeName::new(name_str, cursor.location())?;
    let value = AttributeValue::new(value_str);
    attributes.push(AttributeEntry::new(name, value));
    Ok(())
}

/// Processes `BeforeAttributeName` state.
pub fn handle_before_attribute_name(
    cursor: &mut Cursor<'_>,
    state: &mut State,
    attribute_name: &mut String,
    attribute_value: &mut String,
    tag_name: &mut String,
    attributes: &mut AttributeList,
    is_self_closing: &mut bool,
) -> Result<Option<Token>, HtmlError> {
    while let Some(character) = cursor.next_char() {
        if character.is_ascii_whitespace() {
            continue;
        }
        if character == '/' {
            *state = State::SelfClosingStartTag;
            return Ok(None);
        }
        if character == '>' {
            *state = State::Data;
            return Ok(Some(build_start_tag(
                cursor,
                tag_name,
                attributes,
                is_self_closing,
            )?));
        }
        *state = State::AttributeName;
        attribute_name.clear();
        attribute_name.push(character.to_ascii_lowercase());
        attribute_value.clear();
        return Ok(None);
    }
    *state = State::Data;
    Ok(Some(build_start_tag(
        cursor,
        tag_name,
        attributes,
        is_self_closing,
    )?))
}

/// Processes `AttributeName` state.
pub fn handle_attribute_name(
    cursor: &mut Cursor<'_>,
    state: &mut State,
    attribute_name: &mut String,
    attribute_value: &mut String,
    attributes: &mut AttributeList,
    tag_name: &mut String,
    is_self_closing: &mut bool,
) -> Result<Option<Token>, HtmlError> {
    while let Some(character) = cursor.next_char() {
        if character.is_ascii_whitespace() {
            *state = State::AfterAttributeName;
            return Ok(None);
        }
        if character == '=' {
            *state = State::BeforeAttributeValue;
            return Ok(None);
        }
        if character == '/' {
            commit_attribute(cursor, attributes, attribute_name, attribute_value)?;
            *state = State::SelfClosingStartTag;
            return Ok(None);
        }
        if character == '>' {
            commit_attribute(cursor, attributes, attribute_name, attribute_value)?;
            *state = State::Data;
            return Ok(Some(build_start_tag(
                cursor,
                tag_name,
                attributes,
                is_self_closing,
            )?));
        }
        attribute_name.push(character.to_ascii_lowercase());
    }
    commit_attribute(cursor, attributes, attribute_name, attribute_value)?;
    *state = State::Data;
    Ok(Some(build_start_tag(
        cursor,
        tag_name,
        attributes,
        is_self_closing,
    )?))
}

/// Processes `AfterAttributeName` state.
pub fn handle_after_attribute_name(
    cursor: &mut Cursor<'_>,
    state: &mut State,
    attribute_name: &mut String,
    attribute_value: &mut String,
    attributes: &mut AttributeList,
    tag_name: &mut String,
    is_self_closing: &mut bool,
) -> Result<Option<Token>, HtmlError> {
    while let Some(character) = cursor.next_char() {
        if character.is_ascii_whitespace() {
            continue;
        }
        if character == '=' {
            *state = State::BeforeAttributeValue;
            return Ok(None);
        }
        if character == '/' {
            commit_attribute(cursor, attributes, attribute_name, attribute_value)?;
            *state = State::SelfClosingStartTag;
            return Ok(None);
        }
        if character == '>' {
            commit_attribute(cursor, attributes, attribute_name, attribute_value)?;
            *state = State::Data;
            return Ok(Some(build_start_tag(
                cursor,
                tag_name,
                attributes,
                is_self_closing,
            )?));
        }
        commit_attribute(cursor, attributes, attribute_name, attribute_value)?;
        *state = State::AttributeName;
        attribute_name.push(character.to_ascii_lowercase());
        attribute_value.clear();
        return Ok(None);
    }
    commit_attribute(cursor, attributes, attribute_name, attribute_value)?;
    *state = State::Data;
    Ok(Some(build_start_tag(
        cursor,
        tag_name,
        attributes,
        is_self_closing,
    )?))
}

/// Processes `BeforeAttributeValue` state.
pub fn handle_before_attribute_value(
    cursor: &mut Cursor<'_>,
    state: &mut State,
    attribute_value: &mut String,
    attribute_name: &mut String,
    attributes: &mut AttributeList,
    tag_name: &mut String,
    is_self_closing: &mut bool,
) -> Result<Option<Token>, HtmlError> {
    while let Some(character) = cursor.next_char() {
        if character.is_ascii_whitespace() {
            continue;
        }
        if character == '"' {
            *state = State::AttributeValueDoubleQuoted;
            attribute_value.clear();
            return Ok(None);
        }
        if character == '\'' {
            *state = State::AttributeValueSingleQuoted;
            attribute_value.clear();
            return Ok(None);
        }
        if character == '>' {
            commit_attribute(cursor, attributes, attribute_name, attribute_value)?;
            *state = State::Data;
            return Ok(Some(build_start_tag(
                cursor,
                tag_name,
                attributes,
                is_self_closing,
            )?));
        }
        *state = State::AttributeValueUnquoted;
        attribute_value.clear();
        attribute_value.push(character);
        return Ok(None);
    }
    commit_attribute(cursor, attributes, attribute_name, attribute_value)?;
    *state = State::Data;
    Ok(Some(build_start_tag(
        cursor,
        tag_name,
        attributes,
        is_self_closing,
    )?))
}

/// Processes quoted attribute values (`"` or `'`).
pub fn handle_attribute_value_quoted(
    cursor: &mut Cursor<'_>,
    state: &mut State,
    quote: char,
    attribute_value: &mut String,
) {
    while let Some(character) = cursor.next_char() {
        if character == quote {
            *state = State::AfterAttributeValueQuoted;
            return;
        }
        if character == '&' {
            consume_character_reference(cursor, attribute_value);
            continue;
        }
        attribute_value.push(character);
    }
    *state = State::AfterAttributeValueQuoted;
}

/// Processes `AttributeValueUnquoted` state.
pub fn handle_attribute_value_unquoted(
    cursor: &mut Cursor<'_>,
    state: &mut State,
    attribute_name: &mut String,
    attribute_value: &mut String,
    attributes: &mut AttributeList,
    tag_name: &mut String,
    is_self_closing: &mut bool,
) -> Result<Option<Token>, HtmlError> {
    while let Some(character) = cursor.next_char() {
        if character.is_ascii_whitespace() {
            commit_attribute(cursor, attributes, attribute_name, attribute_value)?;
            *state = State::BeforeAttributeName;
            return Ok(None);
        }
        if character == '/' {
            commit_attribute(cursor, attributes, attribute_name, attribute_value)?;
            *state = State::SelfClosingStartTag;
            return Ok(None);
        }
        if character == '>' {
            commit_attribute(cursor, attributes, attribute_name, attribute_value)?;
            *state = State::Data;
            return Ok(Some(build_start_tag(
                cursor,
                tag_name,
                attributes,
                is_self_closing,
            )?));
        }
        attribute_value.push(character);
    }
    commit_attribute(cursor, attributes, attribute_name, attribute_value)?;
    *state = State::Data;
    Ok(Some(build_start_tag(
        cursor,
        tag_name,
        attributes,
        is_self_closing,
    )?))
}

/// Processes `AfterAttributeValueQuoted` state.
pub fn handle_after_attribute_value_quoted(
    cursor: &mut Cursor<'_>,
    state: &mut State,
    attribute_name: &mut String,
    attribute_value: &mut String,
    attributes: &mut AttributeList,
    tag_name: &mut String,
    is_self_closing: &mut bool,
) -> Result<Option<Token>, HtmlError> {
    commit_attribute(cursor, attributes, attribute_name, attribute_value)?;
    let Some(character) = cursor.next_char() else {
        *state = State::Data;
        return Ok(Some(build_start_tag(
            cursor,
            tag_name,
            attributes,
            is_self_closing,
        )?));
    };

    if character.is_ascii_whitespace() {
        *state = State::BeforeAttributeName;
        return Ok(None);
    }
    if character == '/' {
        *state = State::SelfClosingStartTag;
        return Ok(None);
    }
    if character == '>' {
        *state = State::Data;
        return Ok(Some(build_start_tag(
            cursor,
            tag_name,
            attributes,
            is_self_closing,
        )?));
    }

    *state = State::BeforeAttributeName;
    cursor.reconsume(character);
    Ok(None)
}

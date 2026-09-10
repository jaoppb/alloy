//! Handlers for tag opening, naming, closing, and self-closing states.

use crate::domain::attribute::AttributeList;
use crate::domain::error::HtmlError;
use crate::domain::tag_name::TagName;
use crate::domain::text::Text;
use crate::domain::token::{TagToken, Token};
use crate::infrastructure::tokenizer::cursor::Cursor;
use crate::infrastructure::tokenizer::state::State;

/// Processes the `TagOpen` state.
pub fn handle_tag_open(
    cursor: &mut Cursor<'_>,
    state: &mut State,
    tag_name_buffer: &mut String,
    attributes: &mut AttributeList,
    is_self_closing: &mut bool,
) -> Option<Token> {
    let Some(character) = cursor.next_char() else {
        *state = State::Data;
        return Some(Token::Character(Text::new("<")));
    };

    if character == '!' {
        *state = State::MarkupDeclarationOpen;
        return None;
    }
    if character == '/' {
        *state = State::EndTagOpen;
        return None;
    }
    if character.is_ascii_alphabetic() {
        *state = State::TagName;
        tag_name_buffer.clear();
        tag_name_buffer.push(character.to_ascii_lowercase());
        *attributes = AttributeList::new();
        *is_self_closing = false;
        return None;
    }
    if character == '?' {
        *state = State::BogusComment;
        return None;
    }

    // WHATWG §13.2.5.6: Emit `<` character token and reconsume character in Data state.
    *state = State::Data;
    cursor.reconsume(character);
    Some(Token::Character(Text::new("<")))
}

/// Processes the `EndTagOpen` state.
pub fn handle_end_tag_open(
    cursor: &mut Cursor<'_>,
    state: &mut State,
    tag_name_buffer: &mut String,
    buffer: &mut String,
) {
    let Some(character) = cursor.next_char() else {
        *state = State::Data;
        return;
    };

    if character.is_ascii_alphabetic() {
        *state = State::EndTagName;
        tag_name_buffer.clear();
        tag_name_buffer.push(character.to_ascii_lowercase());
        return;
    }
    if character == '>' {
        *state = State::Data;
        return;
    }

    *state = State::BogusComment;
    buffer.clear();
    buffer.push(character);
}

/// Processes the `TagName` state.
pub fn handle_tag_name(
    cursor: &mut Cursor<'_>,
    state: &mut State,
    tag_name_buffer: &mut String,
    attributes: &mut AttributeList,
    is_self_closing: &mut bool,
) -> Result<Option<Token>, HtmlError> {
    while let Some(character) = cursor.next_char() {
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
                tag_name_buffer,
                attributes,
                is_self_closing,
            )?));
        }
        tag_name_buffer.push(character.to_ascii_lowercase());
    }

    *state = State::Data;
    Ok(Some(build_start_tag(
        cursor,
        tag_name_buffer,
        attributes,
        is_self_closing,
    )?))
}

/// Processes the `EndTagName` state.
pub fn handle_end_tag_name(
    cursor: &mut Cursor<'_>,
    state: &mut State,
    tag_name_buffer: &mut String,
) -> Result<Option<Token>, HtmlError> {
    while let Some(character) = cursor.next_char() {
        if character.is_ascii_whitespace() {
            *state = State::AfterEndTagName;
            return Ok(None);
        }
        if character == '>' {
            *state = State::Data;
            return Ok(Some(build_end_tag(cursor, tag_name_buffer)?));
        }
        tag_name_buffer.push(character.to_ascii_lowercase());
    }

    *state = State::Data;
    Ok(Some(build_end_tag(cursor, tag_name_buffer)?))
}

/// Processes the `AfterEndTagName` state.
pub fn handle_after_end_tag_name(
    cursor: &mut Cursor<'_>,
    state: &mut State,
    tag_name_buffer: &mut String,
) -> Result<Option<Token>, HtmlError> {
    while let Some(character) = cursor.next_char() {
        if character.is_ascii_whitespace() {
            continue;
        }
        if character == '>' {
            *state = State::Data;
            return Ok(Some(build_end_tag(cursor, tag_name_buffer)?));
        }
    }
    *state = State::Data;
    Ok(Some(build_end_tag(cursor, tag_name_buffer)?))
}

/// Processes the `SelfClosingStartTag` state.
pub fn handle_self_closing(
    cursor: &mut Cursor<'_>,
    state: &mut State,
    tag_name_buffer: &mut String,
    attributes: &mut AttributeList,
    is_self_closing: &mut bool,
) -> Result<Option<Token>, HtmlError> {
    while let Some(character) = cursor.next_char() {
        if character == '>' {
            *is_self_closing = true;
            *state = State::Data;
            return Ok(Some(build_start_tag(
                cursor,
                tag_name_buffer,
                attributes,
                is_self_closing,
            )?));
        }
        if character.is_ascii_whitespace() {
            continue;
        }
        *state = State::BeforeAttributeName;
        cursor.reconsume(character);
        return Ok(None);
    }
    *state = State::Data;
    Ok(Some(build_start_tag(
        cursor,
        tag_name_buffer,
        attributes,
        is_self_closing,
    )?))
}

/// Builds a start tag token from accumulated tag name and attributes.
pub fn build_start_tag(
    cursor: &Cursor<'_>,
    tag_name_buffer: &mut String,
    attributes: &mut AttributeList,
    is_self_closing: &mut bool,
) -> Result<Token, HtmlError> {
    let name_str = core::mem::take(tag_name_buffer);
    let tag_name = TagName::new(name_str, cursor.location())?;
    let tag = TagToken::new(tag_name, core::mem::take(attributes), *is_self_closing);
    *is_self_closing = false;
    Ok(Token::StartTag(tag))
}

/// Builds an end tag token from accumulated tag name buffer.
pub fn build_end_tag(
    cursor: &Cursor<'_>,
    tag_name_buffer: &mut String,
) -> Result<Token, HtmlError> {
    let name_str = core::mem::take(tag_name_buffer);
    let tag_name = TagName::new(name_str, cursor.location())?;
    let tag = TagToken::new(tag_name, AttributeList::new(), false);
    Ok(Token::EndTag(tag))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unexpected_char_after_less_than_emits_character_and_reconsumes() {
        let mut cursor = Cursor::new("3 world");
        let mut state = State::TagOpen;
        let mut tag_name = String::new();
        let mut attrs = AttributeList::new();
        let mut self_closing = false;

        let token = handle_tag_open(
            &mut cursor,
            &mut state,
            &mut tag_name,
            &mut attrs,
            &mut self_closing,
        );

        assert_eq!(token, Some(Token::Character(Text::new("<"))));
        assert_eq!(state, State::Data);
        // The character '3' must be reconsumed!
        assert_eq!(cursor.next_char(), Some('3'));
    }
}

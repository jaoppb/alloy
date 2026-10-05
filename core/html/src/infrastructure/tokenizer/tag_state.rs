//! Handlers for tag opening, naming, closing, and self-closing states.

use crate::domain::diagnostic::ParseErrorCode;
use crate::domain::text::Text;
use crate::domain::token::Token;
use crate::infrastructure::tokenizer::cursor::Cursor;
use crate::infrastructure::tokenizer::pending_tag::{PendingTag, TagKind};
use crate::infrastructure::tokenizer::state::State;

/// WHATWG `eof-in-tag`: the unfinished tag is dropped and tokenization resumes in `Data`.
pub fn eof_in_tag(
    cursor: &mut Cursor<'_>,
    state: &mut State,
    tag: &mut PendingTag,
) -> Option<Token> {
    cursor.report(ParseErrorCode::EofInTag, cursor.location());
    tag.discard();
    *state = State::Data;
    None
}

/// Finishes the pending tag in `Data`; `None` when it was invalid and therefore dropped.
pub fn emit_tag(cursor: &mut Cursor<'_>, state: &mut State, tag: &mut PendingTag) -> Option<Token> {
    *state = State::Data;
    tag.finish(cursor)
}

/// Processes the `TagOpen` state.
pub fn handle_tag_open(
    cursor: &mut Cursor<'_>,
    state: &mut State,
    tag: &mut PendingTag,
) -> Option<Token> {
    let Some(character) = cursor.next_char() else {
        cursor.report(ParseErrorCode::EofBeforeTagName, cursor.location());
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
        tag.begin(TagKind::Start, character);
        return None;
    }
    if character == '?' {
        cursor.report(
            ParseErrorCode::UnexpectedQuestionMarkInsteadOfTagName,
            cursor.last_location(),
        );
        *state = State::BogusComment;
        return None;
    }

    // WHATWG §13.2.5.6: Emit `<` character token and reconsume character in Data state.
    cursor.report(
        ParseErrorCode::InvalidFirstCharacterOfTagName,
        cursor.last_location(),
    );
    *state = State::Data;
    cursor.reconsume(character);
    Some(Token::Character(Text::new("<")))
}

/// Processes the `EndTagOpen` state.
pub fn handle_end_tag_open(
    cursor: &mut Cursor<'_>,
    state: &mut State,
    tag: &mut PendingTag,
    buffer: &mut String,
) -> Option<Token> {
    let Some(character) = cursor.next_char() else {
        cursor.report(ParseErrorCode::EofBeforeTagName, cursor.location());
        *state = State::Data;
        return Some(Token::Character(Text::new("</")));
    };

    if character.is_ascii_alphabetic() {
        *state = State::TagName;
        tag.begin(TagKind::End, character);
        return None;
    }
    if character == '>' {
        cursor.report(ParseErrorCode::MissingEndTagName, cursor.last_location());
        *state = State::Data;
        return None;
    }

    cursor.report(
        ParseErrorCode::InvalidFirstCharacterOfTagName,
        cursor.last_location(),
    );
    *state = State::BogusComment;
    buffer.clear();
    buffer.push(character);
    None
}

/// Processes the `TagName` state (start and end tags share the name rules).
pub fn handle_tag_name(
    cursor: &mut Cursor<'_>,
    state: &mut State,
    tag: &mut PendingTag,
) -> Option<Token> {
    while let Some(character) = cursor.next_char() {
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
        tag.push_name_character(character);
    }
    eof_in_tag(cursor, state, tag)
}

/// Processes the `SelfClosingStartTag` state.
pub fn handle_self_closing(
    cursor: &mut Cursor<'_>,
    state: &mut State,
    tag: &mut PendingTag,
) -> Option<Token> {
    while let Some(character) = cursor.next_char() {
        if character == '>' {
            tag.mark_self_closing();
            return emit_tag(cursor, state, tag);
        }
        if character.is_ascii_whitespace() {
            continue;
        }
        cursor.report(
            ParseErrorCode::UnexpectedSolidusInTag,
            cursor.last_location(),
        );
        *state = State::BeforeAttributeName;
        cursor.reconsume(character);
        return None;
    }
    eof_in_tag(cursor, state, tag)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::diagnostic::ParseDiagnostic;

    #[test]
    fn unexpected_char_after_less_than_emits_character_and_reconsumes() {
        let mut cursor = Cursor::new("3 world");
        let mut state = State::TagOpen;
        let mut tag = PendingTag::new();

        let token = handle_tag_open(&mut cursor, &mut state, &mut tag);

        assert_eq!(token, Some(Token::Character(Text::new("<"))));
        assert_eq!(state, State::Data);
        // The character '3' must be reconsumed!
        assert_eq!(cursor.next_char(), Some('3'));
        let reported = cursor.take_diagnostics();
        assert_eq!(
            reported.first().map(ParseDiagnostic::code),
            Some(ParseErrorCode::InvalidFirstCharacterOfTagName)
        );
        assert_eq!(reported.first().map(|d| d.location().column()), Some(1));
    }
}

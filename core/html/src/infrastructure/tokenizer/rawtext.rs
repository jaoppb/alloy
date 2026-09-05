//! Handler for RAWTEXT / script-data tokenizer mode.

use crate::application::ports::RawKind;
use crate::domain::attribute::AttributeList;
use crate::domain::tag_name::TagName;
use crate::domain::text::Text;
use crate::domain::token::{TagToken, Token};
use crate::infrastructure::tokenizer::cursor::Cursor;
use crate::infrastructure::tokenizer::state::State;

/// Consumes raw text content until an appropriate closing tag is reached.
pub fn consume_rawtext(
    cursor: &mut Cursor<'_>,
    state: &mut State,
    kind: RawKind,
    pending_token: &mut Option<Token>,
) -> Token {
    let tag_name_str = match kind {
        RawKind::Script => "script",
        RawKind::Style => "style",
    };

    let mut content = String::new();

    while let Some(character) = cursor.next_char() {
        if character == '<' && is_appropriate_end_tag(cursor, tag_name_str) {
            consume_end_tag_chars(cursor, tag_name_str);
            *state = State::Data;
            let tag_name = TagName::new_unchecked(tag_name_str);
            let end_tag = TagToken::new(tag_name, AttributeList::new(), false);
            *pending_token = Some(Token::EndTag(end_tag));
            return Token::Character(Text::new(content));
        }
        content.push(character);
    }

    *state = State::Data;
    Token::Character(Text::new(content))
}

fn is_appropriate_end_tag(cursor: &Cursor<'_>, expected_tag: &str) -> bool {
    let remaining = cursor.remaining();
    let Some(after_slash) = remaining.strip_prefix('/') else {
        return false;
    };

    let len = expected_tag.len();
    if after_slash.len() < len {
        return false;
    }

    let Some(candidate) = after_slash.get(..len) else {
        return false;
    };
    if !candidate.eq_ignore_ascii_case(expected_tag) {
        return false;
    }

    // Verify next char is delimiter (whitespace, '>', '/', or EOF)
    let next_char = after_slash.get(len..).and_then(|s| s.chars().next());
    matches!(next_char, None | Some('>' | '/' | ' ' | '\t' | '\n' | '\r'))
}

fn consume_end_tag_chars(cursor: &mut Cursor<'_>, expected_tag: &str) {
    // Consume '/'
    cursor.next_char();
    // Consume tag name
    for _ in 0..expected_tag.len() {
        cursor.next_char();
    }
    // Consume until '>'
    while let Some(character) = cursor.next_char() {
        if character == '>' {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn does_not_close_on_prefixed_tag() {
        let cursor = Cursor::new("/scripture>test");
        assert!(!is_appropriate_end_tag(&cursor, "script"));
    }

    #[test]
    fn closes_on_exact_end_tag() {
        let cursor = Cursor::new("/script>test");
        assert!(is_appropriate_end_tag(&cursor, "script"));
    }
}

//! DOCTYPE declaration tokenizer handler.

use crate::domain::diagnostic::ParseErrorCode;
use crate::domain::location::SourceLocation;
use crate::domain::token::{DoctypeToken, Token};
use crate::infrastructure::tokenizer::cursor::Cursor;
use crate::infrastructure::tokenizer::state::State;

/// Processes the `Doctype` state; `location` is where the declaration's `<` was read.
pub fn handle_doctype(
    cursor: &mut Cursor<'_>,
    state: &mut State,
    location: SourceLocation,
) -> Token {
    let mut buffer = String::new();
    let mut closed = false;
    while let Some(character) = cursor.next_char() {
        if character == '>' {
            closed = true;
            break;
        }
        buffer.push(character);
    }
    if !closed {
        cursor.report(ParseErrorCode::EofInDoctype, cursor.location());
    }
    *state = State::Data;

    let content = buffer.trim();
    let Some(name_str) = content.split_whitespace().next() else {
        cursor.report(ParseErrorCode::MissingDoctypeName, location);
        return Token::Doctype(DoctypeToken::new(None, None, None, true, location));
    };

    let mut public_id = None;
    let mut system_id = None;
    let upper = content.to_ascii_uppercase();

    if upper.contains("PUBLIC") {
        public_id = extract_quoted_value(content, "PUBLIC");
    }
    if upper.contains("SYSTEM") {
        system_id = extract_quoted_value(content, "SYSTEM");
    }

    let force_quirks = !name_str.eq_ignore_ascii_case("html");
    Token::Doctype(DoctypeToken::new(
        Some(name_str.to_owned()),
        public_id,
        system_id,
        force_quirks,
        location,
    ))
}

fn extract_quoted_value(content: &str, keyword: &str) -> Option<String> {
    let lower_keyword = keyword.to_ascii_lowercase();
    let lower_content = content.to_ascii_lowercase();
    let index = lower_content.find(&lower_keyword)?;
    let start_remainder = index.checked_add(keyword.len())?;
    let remainder = content.get(start_remainder..)?;

    let start_quote = remainder.find(['"', '\''])?;
    let quote_char = remainder.chars().nth(start_quote)?;
    let after_quote_index = start_quote.checked_add(1)?;
    let after_quote = remainder.get(after_quote_index..)?;
    let end_quote = after_quote.find(quote_char)?;

    after_quote.get(..end_quote).map(ToString::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::diagnostic::ParseDiagnostic;

    #[test]
    fn parse_html5_doctype() {
        let mut cursor = Cursor::new("html>");
        let mut state = State::Doctype;
        let token = handle_doctype(&mut cursor, &mut state, SourceLocation::initial());
        if let Token::Doctype(doctype) = token {
            assert_eq!(doctype.name(), Some("html"));
            assert_eq!(doctype.public_id(), None);
            assert_eq!(doctype.system_id(), None);
            assert!(!doctype.force_quirks());
        } else {
            panic!("expected doctype token");
        }
    }

    #[test]
    fn parse_public_doctype() {
        let mut cursor = Cursor::new("html PUBLIC \"-//W3C//DTD HTML 4.01//EN\">");
        let mut state = State::Doctype;
        let token = handle_doctype(&mut cursor, &mut state, SourceLocation::initial());
        if let Token::Doctype(doctype) = token {
            assert_eq!(doctype.name(), Some("html"));
            assert_eq!(doctype.public_id(), Some("-//W3C//DTD HTML 4.01//EN"));
        } else {
            panic!("expected doctype token");
        }
    }

    #[test]
    fn a_nameless_doctype_reports_and_forces_quirks() {
        let mut cursor = Cursor::new(">");
        let mut state = State::Doctype;
        let token = handle_doctype(&mut cursor, &mut state, SourceLocation::new(1, 1, 0));
        let Token::Doctype(doctype) = token else {
            panic!("expected doctype token");
        };
        assert_eq!(doctype.name(), None);
        assert!(doctype.force_quirks());
        let codes: Vec<_> = cursor
            .take_diagnostics()
            .iter()
            .map(ParseDiagnostic::code)
            .collect();
        assert_eq!(codes, [ParseErrorCode::MissingDoctypeName]);
    }

    #[test]
    fn an_unterminated_doctype_reports_eof() {
        let mut cursor = Cursor::new("html");
        let mut state = State::Doctype;
        handle_doctype(&mut cursor, &mut state, SourceLocation::initial());
        let codes: Vec<_> = cursor
            .take_diagnostics()
            .iter()
            .map(ParseDiagnostic::code)
            .collect();
        assert_eq!(codes, [ParseErrorCode::EofInDoctype]);
    }
}

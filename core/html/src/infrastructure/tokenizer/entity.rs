//! HTML entity resolution for named, decimal, and hexadecimal character references.

use crate::infrastructure::tokenizer::cursor::Cursor;

/// Resolves an HTML character reference from the cursor stream.
pub fn consume_character_reference(cursor: &mut Cursor<'_>, output: &mut String) {
    let mut candidate = String::new();
    let mut cloned_cursor = cursor.clone();
    let mut matched_chars = 0_usize;

    while let Some(character) = cloned_cursor.next_char() {
        matched_chars = matched_chars.saturating_add(1);
        if character == ';' {
            if let Some(resolved) = resolve_entity(&candidate) {
                output.push_str(&resolved);
                for _ in 0..matched_chars {
                    cursor.next_char();
                }
                return;
            }
            break;
        }
        if !character.is_ascii_alphanumeric() && character != '#' {
            break;
        }
        candidate.push(character);
        if candidate.len() > 16 {
            break;
        }
    }

    output.push('&');
}

/// Resolves a named or numeric entity name.
#[must_use]
pub fn resolve_entity(name: &str) -> Option<String> {
    if let Some(stripped) = name.strip_prefix("#x").or_else(|| name.strip_prefix("#X")) {
        let code = u32::from_str_radix(stripped, 16).ok()?;
        return char::from_u32(code).map(String::from);
    }
    if let Some(stripped) = name.strip_prefix('#') {
        let code = stripped.parse::<u32>().ok()?;
        return char::from_u32(code).map(String::from);
    }

    match name {
        "amp" => Some("&".to_string()),
        "lt" => Some("<".to_string()),
        "gt" => Some(">".to_string()),
        "quot" => Some("\"".to_string()),
        "apos" => Some("'".to_string()),
        "copy" => Some("©".to_string()),
        "reg" => Some("®".to_string()),
        "nbsp" => Some("\u{00A0}".to_string()),
        "mdash" => Some("—".to_string()),
        "ndash" => Some("–".to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_named_entities() {
        assert_eq!(resolve_entity("amp").as_deref(), Some("&"));
        assert_eq!(resolve_entity("copy").as_deref(), Some("©"));
        assert_eq!(resolve_entity("unknown"), None);
    }

    #[test]
    fn resolve_numeric_entities() {
        assert_eq!(resolve_entity("#60").as_deref(), Some("<"));
        assert_eq!(resolve_entity("#x3e").as_deref(), Some(">"));
        assert_eq!(resolve_entity("#X3E").as_deref(), Some(">"));
    }
}

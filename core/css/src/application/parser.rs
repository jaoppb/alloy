use crate::domain::declaration::{Declaration, DeclarationList};
use crate::domain::error::CssError;
use crate::domain::property::{Color, PropertyName, PropertyValue, Px};
use crate::domain::rule::{Rule, RuleSet};
use crate::domain::selector::Selector;
use crate::domain::stylesheet::StyleSheet;
use dom::TagName;

/// Application parser converting raw CSS text into a `StyleSheet`.
pub struct CssParser;

impl CssParser {
    /// Parses a CSS string slice into a `StyleSheet`.
    ///
    /// # Errors
    /// Returns `CssError` if a critical syntax error is encountered.
    pub fn parse_stylesheet(css: &str) -> Result<StyleSheet, CssError> {
        let cleaned = strip_css_comments(css);
        let mut rules = RuleSet::new();
        let mut chars = cleaned.chars().peekable();

        while chars.peek().is_some() {
            skip_whitespace(&mut chars);
            if chars.peek().is_none() {
                break;
            }

            // 1. Read selector chunk up to '{'
            let mut selector_buf = String::new();
            while let Some(&ch) = chars.peek() {
                if ch == '{' {
                    chars.next();
                    break;
                }
                selector_buf.push(ch);
                chars.next();
            }

            let selectors = parse_selectors(&selector_buf)?;
            if selectors.is_empty() {
                // Skip declarations block if no valid selector
                skip_block(&mut chars);
                continue;
            }

            // 2. Read declarations chunk up to '}'
            let mut decl_buf = String::new();
            while let Some(&ch) = chars.peek() {
                if ch == '}' {
                    chars.next();
                    break;
                }
                decl_buf.push(ch);
                chars.next();
            }

            let declarations = parse_declarations(&decl_buf);
            rules.push(Rule::new(selectors, declarations));
        }

        Ok(StyleSheet::new(rules))
    }
}

/// Convenience function parsing a CSS string into a `StyleSheet`.
///
/// # Errors
/// Returns `CssError` if parsing fails.
pub fn parse_css(css: &str) -> Result<StyleSheet, CssError> {
    CssParser::parse_stylesheet(css)
}

fn strip_css_comments(css: &str) -> String {
    let mut out = String::with_capacity(css.len());
    let mut chars = css.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '/' && chars.peek() == Some(&'*') {
            chars.next(); // Skip '*'
            while let Some(c) = chars.next() {
                if c == '*' && chars.peek() == Some(&'/') {
                    chars.next(); // Skip '/'
                    break;
                }
            }
            continue;
        }
        out.push(ch);
    }

    out
}

fn parse_selectors(raw: &str) -> Result<Vec<Selector>, CssError> {
    let mut selectors = Vec::new();
    for part in raw.split(',') {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }

        let tokens: Vec<&str> = trimmed.split_ascii_whitespace().collect();
        if tokens.is_empty() {
            continue;
        }

        let mut current_sel = parse_simple_selector(tokens[0])?;
        for &next_token in &tokens[1..] {
            let next_sel = parse_simple_selector(next_token)?;
            current_sel = Selector::Descendant(Box::new(current_sel), Box::new(next_sel));
        }
        selectors.push(current_sel);
    }
    Ok(selectors)
}

fn parse_simple_selector(token: &str) -> Result<Selector, CssError> {
    if token == "*" {
        return Ok(Selector::Universal);
    }

    if let Some(id) = token.strip_prefix('#') {
        return Ok(Selector::Id(id.to_string()));
    }

    if let Some(class) = token.strip_prefix('.') {
        return Ok(Selector::Class(class.to_string()));
    }

    let tag = TagName::new(token).map_err(|e| CssError::InvalidSelector(e.to_string()))?;
    Ok(Selector::Tag(tag))
}

fn parse_declarations(raw: &str) -> DeclarationList {
    let mut list = DeclarationList::new();
    for entry in raw.split(';') {
        let trimmed = entry.trim();
        if trimmed.is_empty() {
            continue;
        }

        let mut parts = trimmed.splitn(2, ':');
        let Some(name_str) = parts.next() else {
            continue;
        };
        let Some(val_str) = parts.next() else {
            continue;
        };

        let prop_name = PropertyName::new(name_str);
        let prop_value = parse_property_value(val_str.trim());
        list.push(Declaration::new(prop_name, prop_value));
    }
    list
}

fn parse_property_value(val: &str) -> PropertyValue {
    if let Some(color) = Color::parse(val) {
        return PropertyValue::Color(color);
    }

    if let Some(px_str) = val.strip_suffix("px") {
        if let Ok(num) = px_str.trim().parse::<f32>() {
            return PropertyValue::Length(Px::new(num));
        }
    }

    match val {
        "block" | "inline" | "none" | "flex" | "auto" | "inherit" => {
            PropertyValue::Keyword(val.to_string())
        }
        _ => PropertyValue::Raw(val.to_string()),
    }
}

fn skip_whitespace(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) {
    while let Some(&ch) = chars.peek() {
        if !ch.is_ascii_whitespace() {
            break;
        }
        chars.next();
    }
}

fn skip_block(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) {
    for ch in chars.by_ref() {
        if ch == '}' {
            break;
        }
    }
}

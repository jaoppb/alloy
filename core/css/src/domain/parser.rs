use crate::domain::declaration::{Declaration, DeclarationList};
use crate::domain::error::CssError;
use crate::domain::property::{Color, CssKeyword, PropertyName, PropertyValue, Px};
use crate::domain::rule::{Rule, RuleSet};
use crate::domain::selector::{AttributeMatcher, PseudoClass, Selector};
use crate::domain::stylesheet::StyleSheet;
use dom::{AttributeName, TagName};

/// Parser converting raw CSS stylesheet strings into strongly typed `StyleSheet` aggregates.
pub struct CssParser;

impl CssParser {
    /// Parses a raw CSS stylesheet string into a `StyleSheet`.
    ///
    /// # Errors
    /// Returns `CssError` if syntax errors are encountered.
    pub fn parse_stylesheet(css: &str) -> Result<StyleSheet, CssError> {
        let stripped = strip_css_comments(css);
        let mut rules = Vec::new();

        let mut chars = stripped.chars().peekable();
        while chars.peek().is_some() {
            // Skip whitespace
            while let Some(&ch) = chars.peek() {
                if ch.is_whitespace() {
                    chars.next();
                } else {
                    break;
                }
            }

            if chars.peek().is_none() {
                break;
            }

            // Parse selectors up to '{'
            let mut selector_buf = String::new();
            let mut found_brace = false;
            while let Some(&ch) = chars.peek() {
                if ch == '{' {
                    chars.next();
                    found_brace = true;
                    break;
                }
                selector_buf.push(ch);
                chars.next();
            }

            if !found_brace {
                break;
            }

            let selectors = parse_selectors(&selector_buf)?;

            // Parse declarations up to '}'
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
            rules.push(Rule::new(selectors, declarations)?);
        }

        Ok(StyleSheet::new(RuleSet::from_vec(rules)))
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

pub(crate) fn parse_selectors(raw: &str) -> Result<Vec<Selector>, CssError> {
    let mut selectors = Vec::new();
    for part in raw.split(',') {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }
        selectors.push(parse_single_selector_sequence(trimmed)?);
    }
    if selectors.is_empty() {
        return Err(CssError::InvalidSelector("No valid selectors found".into()));
    }
    Ok(selectors)
}

#[derive(Debug, PartialEq)]
enum Combinator {
    Descendant,
    Child,
    AdjacentSibling,
    GeneralSibling,
}

fn parse_single_selector_sequence(raw: &str) -> Result<Selector, CssError> {
    let mut chars = raw.chars().peekable();
    let mut tokens: Vec<(Option<Combinator>, String)> = Vec::new();
    let mut current_chunk = String::new();
    let mut pending_combinator: Option<Combinator> = None;

    while let Some(ch) = chars.next() {
        if ch == '[' {
            current_chunk.push('[');
            for c in chars.by_ref() {
                current_chunk.push(c);
                if c == ']' {
                    break;
                }
            }
            continue;
        }

        if ch == '>' {
            if !current_chunk.trim().is_empty() {
                tokens.push((pending_combinator, current_chunk.trim().to_string()));
                current_chunk.clear();
            }
            pending_combinator = Some(Combinator::Child);
            continue;
        }

        if ch == '+' {
            if !current_chunk.trim().is_empty() {
                tokens.push((pending_combinator, current_chunk.trim().to_string()));
                current_chunk.clear();
            }
            pending_combinator = Some(Combinator::AdjacentSibling);
            continue;
        }

        if ch == '~' {
            if !current_chunk.trim().is_empty() {
                tokens.push((pending_combinator, current_chunk.trim().to_string()));
                current_chunk.clear();
            }
            pending_combinator = Some(Combinator::GeneralSibling);
            continue;
        }

        if ch.is_whitespace() {
            if !current_chunk.trim().is_empty() {
                tokens.push((pending_combinator, current_chunk.trim().to_string()));
                current_chunk.clear();
                pending_combinator = Some(Combinator::Descendant);
            }
            continue;
        }

        current_chunk.push(ch);
    }

    if !current_chunk.trim().is_empty() {
        tokens.push((pending_combinator, current_chunk.trim().to_string()));
    }

    if tokens.is_empty() {
        return Err(CssError::InvalidSelector("Empty selector sequence".into()));
    }

    let mut iter = tokens.into_iter();
    let (_, first_chunk) = iter.next().unwrap();
    let mut current_sel = parse_compound_selector(&first_chunk)?;

    for (comb, chunk) in iter {
        let next_sel = parse_compound_selector(&chunk)?;
        let c = comb.unwrap_or(Combinator::Descendant);
        current_sel = match c {
            Combinator::Descendant => {
                Selector::Descendant(Box::new(current_sel), Box::new(next_sel))
            }
            Combinator::Child => Selector::Child(Box::new(current_sel), Box::new(next_sel)),
            Combinator::AdjacentSibling => {
                Selector::AdjacentSibling(Box::new(current_sel), Box::new(next_sel))
            }
            Combinator::GeneralSibling => {
                Selector::GeneralSibling(Box::new(current_sel), Box::new(next_sel))
            }
        };
    }

    Ok(current_sel)
}

fn parse_compound_selector(chunk: &str) -> Result<Selector, CssError> {
    let mut simple_selectors = Vec::new();
    let mut chars = chunk.chars().peekable();

    while let Some(&ch) = chars.peek() {
        if ch == '.' {
            chars.next();
            let mut class_name = String::new();
            while let Some(&c) = chars.peek() {
                if c == '.' || c == '#' || c == '[' || c == ':' {
                    break;
                }
                class_name.push(chars.next().unwrap());
            }
            if class_name.is_empty() {
                return Err(CssError::InvalidSelector("Empty class name".into()));
            }
            simple_selectors.push(Selector::Class(class_name));
        } else if ch == '#' {
            chars.next();
            let mut id_name = String::new();
            while let Some(&c) = chars.peek() {
                if c == '.' || c == '#' || c == '[' || c == ':' {
                    break;
                }
                id_name.push(chars.next().unwrap());
            }
            if id_name.is_empty() {
                return Err(CssError::InvalidSelector("Empty ID name".into()));
            }
            simple_selectors.push(Selector::Id(id_name));
        } else if ch == '[' {
            chars.next();
            let mut attr_content = String::new();
            for c in chars.by_ref() {
                if c == ']' {
                    break;
                }
                attr_content.push(c);
            }
            let attr_sel = parse_attribute_selector(&attr_content)?;
            simple_selectors.push(attr_sel);
        } else if ch == ':' {
            chars.next();
            let mut pseudo_name = String::new();
            while let Some(&c) = chars.peek() {
                if c == '.' || c == '#' || c == '[' || c == ':' {
                    break;
                }
                pseudo_name.push(chars.next().unwrap());
            }
            let pseudo = match pseudo_name.as_str() {
                "root" => PseudoClass::Root,
                "first-child" => PseudoClass::FirstChild,
                "last-child" => PseudoClass::LastChild,
                "only-child" => PseudoClass::OnlyChild,
                "empty" => PseudoClass::Empty,
                _ => {
                    return Err(CssError::InvalidSelector(format!(
                        "Unsupported pseudo-class :{pseudo_name}"
                    )));
                }
            };
            simple_selectors.push(Selector::PseudoClass(pseudo));
        } else {
            let mut tag_str = String::new();
            while let Some(&c) = chars.peek() {
                if c == '.' || c == '#' || c == '[' || c == ':' {
                    break;
                }
                tag_str.push(chars.next().unwrap());
            }
            if tag_str == "*" {
                simple_selectors.push(Selector::Universal);
            } else {
                let tag =
                    TagName::new(tag_str).map_err(|e| CssError::InvalidSelector(e.to_string()))?;
                simple_selectors.push(Selector::Tag(tag));
            }
        }
    }

    if simple_selectors.is_empty() {
        return Err(CssError::InvalidSelector("Empty compound selector".into()));
    }

    if simple_selectors.len() == 1 {
        Ok(simple_selectors.remove(0))
    } else {
        Ok(Selector::Compound(simple_selectors))
    }
}

type AttrOp = (&'static str, fn(String) -> AttributeMatcher);

fn parse_attribute_selector(content: &str) -> Result<Selector, CssError> {
    const OPERATORS: [AttrOp; 6] = [
        ("~=", AttributeMatcher::Includes),
        ("|=", AttributeMatcher::DashMatch),
        ("^=", AttributeMatcher::Prefix),
        ("$=", AttributeMatcher::Suffix),
        ("*=", AttributeMatcher::Substring),
        ("=", AttributeMatcher::Exact),
    ];

    let (name_part, matcher) = OPERATORS
        .iter()
        .find_map(|(op, ctor)| {
            content
                .split_once(op)
                .map(|(name, val)| (name.trim(), ctor(clean_attr_val(val))))
        })
        .unwrap_or_else(|| (content.trim(), AttributeMatcher::Exists));

    if name_part.is_empty() {
        return Err(CssError::InvalidSelector(
            "Empty attribute name in selector".into(),
        ));
    }

    let attr_name = AttributeName::new(name_part);

    Ok(Selector::Attribute {
        name: attr_name,
        matcher,
    })
}

fn clean_attr_val(val: &str) -> String {
    let trimmed = val.trim();
    if (trimmed.starts_with('"') && trimmed.ends_with('"'))
        || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
    {
        trimmed[1..trimmed.len() - 1].to_string()
    } else {
        trimmed.to_string()
    }
}

fn parse_declarations(raw: &str) -> DeclarationList {
    let mut list = DeclarationList::new();
    for decl in raw.split(';') {
        let trimmed = decl.trim();
        if trimmed.is_empty() {
            continue;
        }

        let Some((prop_raw, val_raw)) = trimmed.split_once(':') else {
            continue;
        };

        let prop_name = PropertyName::new(prop_raw);
        let val_trimmed = val_raw.trim();

        let prop_val = parse_property_value(val_trimmed);
        list.push(Declaration::new(prop_name, prop_val));
    }
    list
}

fn parse_property_value(raw: &str) -> PropertyValue {
    if let Some(c) = Color::parse(raw) {
        return PropertyValue::Color(c);
    }

    if let Some(px_str) = raw.strip_suffix("px") {
        if let Ok(val) = px_str.trim().parse::<f32>() {
            return PropertyValue::Length(Px::new(val));
        }
    }

    if let Ok(val) = raw.parse::<f32>() {
        return PropertyValue::Length(Px::new(val));
    }

    let kw = CssKeyword::parse(raw);
    match kw {
        CssKeyword::Custom(_) => PropertyValue::Raw(raw.to_string()),
        standard_kw => PropertyValue::Keyword(standard_kw),
    }
}

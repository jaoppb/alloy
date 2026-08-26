/// Strongly typed W3C HTML named character references (C-35).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HtmlEntity {
    Amp,
    Lt,
    Gt,
    Quot,
    Apos,
    Nbsp,
    Copy,
    Reg,
    Trade,
    Cent,
    Pound,
    Yen,
    Euro,
}

impl HtmlEntity {
    /// Parses an entity name without enclosing '&' and ';'.
    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "amp" => Some(Self::Amp),
            "lt" => Some(Self::Lt),
            "gt" => Some(Self::Gt),
            "quot" => Some(Self::Quot),
            "apos" | "#39" => Some(Self::Apos),
            "nbsp" => Some(Self::Nbsp),
            "copy" => Some(Self::Copy),
            "reg" => Some(Self::Reg),
            "trade" => Some(Self::Trade),
            "cent" => Some(Self::Cent),
            "pound" => Some(Self::Pound),
            "yen" => Some(Self::Yen),
            "euro" => Some(Self::Euro),
            _ => None,
        }
    }

    /// Returns the character replacement for this entity.
    #[must_use]
    pub const fn as_char(self) -> char {
        match self {
            Self::Amp => '&',
            Self::Lt => '<',
            Self::Gt => '>',
            Self::Quot => '"',
            Self::Apos => '\'',
            Self::Nbsp => '\u{00A0}',
            Self::Copy => '©',
            Self::Reg => '®',
            Self::Trade => '™',
            Self::Cent => '¢',
            Self::Pound => '£',
            Self::Yen => '¥',
            Self::Euro => '€',
        }
    }
}

/// Decodes HTML entity references (both named references and numeric codes) into characters.
#[must_use]
pub fn decode_html_entities(input: &str) -> String {
    if !input.contains('&') {
        return input.to_string();
    }

    let mut output = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '&' {
            let mut entity = String::new();
            let mut found_semicolon = false;

            while let Some(&next_ch) = chars.peek() {
                if next_ch == ';' {
                    chars.next();
                    found_semicolon = true;
                    break;
                }
                if next_ch == '&' || next_ch.is_whitespace() || entity.len() > 10 {
                    break;
                }
                entity.push(chars.next().unwrap());
            }

            if found_semicolon {
                if let Some(entity_enum) = HtmlEntity::parse(&entity) {
                    output.push(entity_enum.as_char());
                } else if let Some(num_str) = entity
                    .strip_prefix("#x")
                    .or_else(|| entity.strip_prefix("#X"))
                {
                    if let Ok(code) = u32::from_str_radix(num_str, 16) {
                        if let Some(ch) = char::from_u32(code) {
                            output.push(ch);
                        } else {
                            output.push('&');
                            output.push_str(&entity);
                            output.push(';');
                        }
                    } else {
                        output.push('&');
                        output.push_str(&entity);
                        output.push(';');
                    }
                } else if let Some(num_str) = entity.strip_prefix('#') {
                    if let Ok(code) = num_str.parse::<u32>() {
                        if let Some(ch) = char::from_u32(code) {
                            output.push(ch);
                        } else {
                            output.push('&');
                            output.push_str(&entity);
                            output.push(';');
                        }
                    } else {
                        output.push('&');
                        output.push_str(&entity);
                        output.push(';');
                    }
                } else {
                    output.push('&');
                    output.push_str(&entity);
                    output.push(';');
                }
            } else {
                output.push('&');
                output.push_str(&entity);
            }
        } else {
            output.push(ch);
        }
    }

    output
}

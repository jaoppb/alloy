/// Decodes common HTML entity references into their corresponding characters.
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
                match entity.as_str() {
                    "amp" => output.push('&'),
                    "lt" => output.push('<'),
                    "gt" => output.push('>'),
                    "quot" => output.push('"'),
                    "apos" | "#39" => output.push('\''),
                    "nbsp" => output.push('\u{00A0}'),
                    _ => {
                        output.push('&');
                        output.push_str(&entity);
                        output.push(';');
                    }
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

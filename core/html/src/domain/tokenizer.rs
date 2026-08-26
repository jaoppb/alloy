use crate::domain::entities::decode_html_entities;
use crate::domain::token::{HtmlError, HtmlToken};
use dom::{AttributeMap, AttributeName, AttributeValue, TagName};

/// Stateful tokenizer converting raw HTML characters into `HtmlToken` items.
pub struct HtmlTokenizer<'a> {
    chars: &'a [u8],
    pos: usize,
}

impl<'a> HtmlTokenizer<'a> {
    /// Creates a new tokenizer over an HTML string slice.
    #[must_use]
    pub const fn new(input: &'a str) -> Self {
        Self {
            chars: input.as_bytes(),
            pos: 0,
        }
    }

    /// Pulls the next token from the HTML stream.
    ///
    /// # Errors
    /// Returns `HtmlError` if a tag is malformed.
    pub fn next_token(&mut self) -> Result<HtmlToken, HtmlError> {
        if self.is_eof() {
            return Ok(HtmlToken::Eof);
        }

        if self.starts_with(b"<!--") {
            return self.parse_comment();
        }

        if self.starts_with_case_insensitive(b"<!doctype") {
            return self.parse_doctype();
        }

        if self.starts_with(b"</") {
            return self.parse_end_tag();
        }

        if self.starts_with(b"<") {
            return self.parse_start_tag();
        }

        self.parse_text()
    }

    fn parse_comment(&mut self) -> Result<HtmlToken, HtmlError> {
        self.pos += 4; // Skip '<!--'
        let start = self.pos;

        while !self.is_eof() {
            if self.starts_with(b"-->") {
                let comment_text = self.slice_str(start, self.pos);
                self.pos += 3;
                return Ok(HtmlToken::Comment(comment_text.to_string()));
            }
            self.pos += 1;
        }

        let comment_text = self.slice_str(start, self.pos);
        Ok(HtmlToken::Comment(comment_text.to_string()))
    }

    fn parse_doctype(&mut self) -> Result<HtmlToken, HtmlError> {
        self.pos += 9; // Skip '<!doctype'
        let start = self.pos;

        while !self.is_eof() && self.chars[self.pos] != b'>' {
            self.pos += 1;
        }

        let content = self.slice_str(start, self.pos).trim();
        if !self.is_eof() && self.chars[self.pos] == b'>' {
            self.pos += 1;
        }

        Ok(HtmlToken::Doctype(content.to_string()))
    }

    fn parse_end_tag(&mut self) -> Result<HtmlToken, HtmlError> {
        self.pos += 2; // Skip '</'
        self.skip_whitespace();

        let start = self.pos;
        while !self.is_eof()
            && self.chars[self.pos] != b'>'
            && !self.chars[self.pos].is_ascii_whitespace()
        {
            self.pos += 1;
        }

        let raw_tag = self.slice_str(start, self.pos);
        let tag_name =
            TagName::new(raw_tag).map_err(|e| HtmlError::InvalidTagName(e.to_string()))?;

        while !self.is_eof() && self.chars[self.pos] != b'>' {
            self.pos += 1;
        }

        if !self.is_eof() && self.chars[self.pos] == b'>' {
            self.pos += 1;
        }

        Ok(HtmlToken::EndTag(tag_name))
    }

    fn parse_start_tag(&mut self) -> Result<HtmlToken, HtmlError> {
        self.pos += 1; // Skip '<'
        self.skip_whitespace();

        let start = self.pos;
        while !self.is_eof()
            && self.chars[self.pos] != b'>'
            && self.chars[self.pos] != b'/'
            && !self.chars[self.pos].is_ascii_whitespace()
        {
            self.pos += 1;
        }

        let raw_tag = self.slice_str(start, self.pos);
        let tag_name =
            TagName::new(raw_tag).map_err(|e| HtmlError::InvalidTagName(e.to_string()))?;

        let mut attributes = AttributeMap::new();
        let mut self_closing = false;

        loop {
            self.skip_whitespace();
            if self.is_eof() {
                break;
            }

            if self.chars[self.pos] == b'>' {
                self.pos += 1;
                break;
            }

            if self.starts_with(b"/>") {
                self.pos += 2;
                self_closing = true;
                break;
            }

            if self.chars[self.pos] == b'/' {
                self.pos += 1;
                self_closing = true;
                continue;
            }

            self.parse_attribute_entry(&mut attributes)?;
        }

        Ok(HtmlToken::StartTag {
            name: tag_name,
            attributes,
            self_closing,
        })
    }

    fn parse_attribute_entry(&mut self, attributes: &mut AttributeMap) -> Result<(), HtmlError> {
        let name_start = self.pos;
        while !self.is_eof()
            && self.chars[self.pos] != b'='
            && self.chars[self.pos] != b'>'
            && self.chars[self.pos] != b'/'
            && !self.chars[self.pos].is_ascii_whitespace()
        {
            self.pos += 1;
        }

        let attr_name = self.slice_str(name_start, self.pos).trim();
        if attr_name.is_empty() {
            return Ok(());
        }

        self.skip_whitespace();
        let mut attr_value = String::new();

        if !self.is_eof() && self.chars[self.pos] == b'=' {
            self.pos += 1; // Skip '='
            self.skip_whitespace();
            attr_value = self.parse_attribute_value();
        }

        attributes.insert(
            AttributeName::new(attr_name),
            AttributeValue::new(decode_html_entities(&attr_value)),
        );

        Ok(())
    }

    fn parse_attribute_value(&mut self) -> String {
        if self.is_eof() {
            return String::new();
        }

        let quote = self.chars[self.pos];
        if quote == b'"' || quote == b'\'' {
            self.pos += 1;
            let val_start = self.pos;
            while !self.is_eof() && self.chars[self.pos] != quote {
                self.pos += 1;
            }
            let val = self.slice_str(val_start, self.pos).to_string();
            if !self.is_eof() && self.chars[self.pos] == quote {
                self.pos += 1;
            }
            return val;
        }

        let val_start = self.pos;
        while !self.is_eof()
            && self.chars[self.pos] != b'>'
            && self.chars[self.pos] != b'/'
            && !self.chars[self.pos].is_ascii_whitespace()
        {
            self.pos += 1;
        }
        self.slice_str(val_start, self.pos).to_string()
    }

    fn parse_text(&mut self) -> Result<HtmlToken, HtmlError> {
        let start = self.pos;
        while !self.is_eof() && self.chars[self.pos] != b'<' {
            self.pos += 1;
        }

        let raw_text = self.slice_str(start, self.pos);
        let decoded = decode_html_entities(raw_text);
        Ok(HtmlToken::Character(decoded))
    }

    fn skip_whitespace(&mut self) {
        while !self.is_eof() && self.chars[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    const fn is_eof(&self) -> bool {
        self.pos >= self.chars.len()
    }

    fn starts_with(&self, prefix: &[u8]) -> bool {
        if self.pos + prefix.len() > self.chars.len() {
            return false;
        }
        &self.chars[self.pos..self.pos + prefix.len()] == prefix
    }

    fn starts_with_case_insensitive(&self, prefix: &[u8]) -> bool {
        if self.pos + prefix.len() > self.chars.len() {
            return false;
        }
        for (i, &b) in prefix.iter().enumerate() {
            if !self.chars[self.pos + i].eq_ignore_ascii_case(&b) {
                return false;
            }
        }
        true
    }

    fn slice_str(&self, start: usize, end: usize) -> &'a str {
        std::str::from_utf8(&self.chars[start..end]).unwrap_or("")
    }
}

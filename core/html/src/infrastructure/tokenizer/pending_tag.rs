//! The tag the tokenizer is currently assembling, and the rules that finish it.

use crate::domain::attribute::{AttributeEntry, AttributeList, AttributeName, AttributeValue};
use crate::domain::diagnostic::ParseErrorCode;
use crate::domain::location::SourceLocation;
use crate::domain::tag::TagName;
use crate::domain::token::{TagToken, Token};
use crate::infrastructure::tokenizer::cursor::Cursor;

/// Whether the pending tag opens or closes an element.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TagKind {
    /// `<name ...>`
    #[default]
    Start,
    /// `</name>`
    End,
}

/// Scratch state for one tag, grouped so the state handlers share one parameter instead of seven.
#[derive(Debug, Default)]
pub struct PendingTag {
    kind: TagKind,
    open_location: SourceLocation,
    name: String,
    attributes: AttributeList,
    attribute_name: String,
    attribute_value: String,
    attribute_location: SourceLocation,
    self_closing: bool,
}

impl PendingTag {
    /// Creates empty scratch state.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            kind: TagKind::Start,
            open_location: SourceLocation::initial(),
            name: String::new(),
            attributes: AttributeList::new(),
            attribute_name: String::new(),
            attribute_value: String::new(),
            attribute_location: SourceLocation::initial(),
            self_closing: false,
        }
    }

    /// Records where the `<` that opens the next construct was read.
    pub const fn mark_open(&mut self, location: SourceLocation) {
        self.open_location = location;
    }

    /// Where the most recent `<` was read.
    #[must_use]
    pub const fn open_location(&self) -> SourceLocation {
        self.open_location
    }

    /// Starts a new tag named by `first_character`.
    pub fn begin(&mut self, kind: TagKind, first_character: char) {
        self.discard();
        self.kind = kind;
        self.name.push(first_character.to_ascii_lowercase());
    }

    /// Appends to the tag name.
    pub fn push_name_character(&mut self, character: char) {
        self.name.push(character.to_ascii_lowercase());
    }

    /// Starts a new attribute whose name begins with `first_character` at `location`.
    pub fn begin_attribute(&mut self, first_character: char, location: SourceLocation) {
        self.attribute_name.clear();
        self.attribute_value.clear();
        self.attribute_name
            .push(first_character.to_ascii_lowercase());
        self.attribute_location = location;
    }

    /// Appends to the attribute name.
    pub fn push_attribute_name_character(&mut self, character: char) {
        self.attribute_name.push(character.to_ascii_lowercase());
    }

    /// Appends to the attribute value.
    pub fn push_value_character(&mut self, character: char) {
        self.attribute_value.push(character);
    }

    /// Clears the value, as every value-start transition does.
    pub fn start_value(&mut self) {
        self.attribute_value.clear();
    }

    /// The value buffer, for character-reference resolution.
    pub const fn value_buffer(&mut self) -> &mut String {
        &mut self.attribute_value
    }

    /// Marks the tag as self-closing (`/>`).
    pub const fn mark_self_closing(&mut self) {
        self.self_closing = true;
    }

    /// Moves the pending attribute into the list; a repeated name is reported and dropped.
    pub fn commit_attribute(&mut self, cursor: &mut Cursor<'_>) {
        if self.attribute_name.is_empty() {
            return;
        }
        let raw_name = core::mem::take(&mut self.attribute_name);
        let value = AttributeValue::new(core::mem::take(&mut self.attribute_value));
        // `raw_name` is non-empty, the only thing `AttributeName::new` rejects (strictness is #28).
        let Ok(name) = AttributeName::new(raw_name, self.attribute_location) else {
            return;
        };
        let entry = AttributeEntry::new(name, value, self.attribute_location);
        if let Err(duplicate) = self.attributes.insert(entry) {
            cursor.report(
                ParseErrorCode::DuplicateAttribute,
                duplicate.entry().location(),
            );
        }
    }

    /// Drops everything collected so far (an unfinished or invalid tag).
    pub fn discard(&mut self) {
        self.name.clear();
        self.attributes = AttributeList::new();
        self.attribute_name.clear();
        self.attribute_value.clear();
        self.self_closing = false;
    }

    /// Builds the token for the finished tag, or reports why there is none.
    pub fn finish(&mut self, cursor: &mut Cursor<'_>) -> Option<Token> {
        let name = core::mem::take(&mut self.name);
        let Ok(tag_name) = TagName::new(&name, self.open_location) else {
            cursor.report(ParseErrorCode::InvalidTagName, self.open_location);
            self.discard();
            return None;
        };
        let attributes = core::mem::take(&mut self.attributes);
        let self_closing = core::mem::take(&mut self.self_closing);
        match self.kind {
            TagKind::Start => Some(Token::StartTag(TagToken::new(
                tag_name,
                attributes,
                self_closing,
                self.open_location,
            ))),
            TagKind::End => {
                if !attributes.is_empty() {
                    cursor.report(ParseErrorCode::EndTagWithAttributes, self.open_location);
                }
                Some(Token::EndTag(TagToken::new(
                    tag_name,
                    AttributeList::new(),
                    false,
                    self.open_location,
                )))
            }
        }
    }
}

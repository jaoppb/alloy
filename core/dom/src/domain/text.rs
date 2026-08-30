//! [`TextContent`] and [`CommentContent`] — thin newtypes over the character
//! data of a text or comment node. No validation: any string is legal content
//! (v0.2 report §2.2). Escaping is the serializer's concern, not the model's.

/// The character data of a `Text` node.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct TextContent(String);

impl TextContent {
    pub fn new(content: impl Into<String>) -> Self {
        Self(content.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The character data of a `Comment` node.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct CommentContent(String);

impl CommentContent {
    pub fn new(content: impl Into<String>) -> Self {
        Self(content.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Standard CSS formatting display modes (W3C CSS Display Module Level 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum DisplayType {
    /// Standard block-level formatting.
    #[default]
    Block,
    /// Inline-level formatting flow.
    Inline,
    /// Element generates no boxes and is hidden from layout.
    None,
    /// Flexbox formatting context.
    Flex,
}

impl DisplayType {
    /// Returns the CSS keyword string for this display mode.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Block => "block",
            Self::Inline => "inline",
            Self::None => "none",
            Self::Flex => "flex",
        }
    }
}

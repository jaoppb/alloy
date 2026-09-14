//! [`StyleSheetSet`] — parsed, ordered rules with an [`Origin`]
//! (`PRD-007:37-38`).
//!
//! B0 has no CSS parser (that is B1), so a [`StyleRule`] here is an honest
//! scaffold: a selector as text plus a [`DeclarationBlock`] of
//! `(property, value)` strings. B1 replaces these with parsed selector and
//! declaration types without changing the aggregate's shape.

use core::fmt;

/// Which stylesheet a rule came from. The cascade orders origins
/// UA < User < Author (`PRD-007:38`); B0's `UaCascade` only ever sees
/// `UserAgent`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Origin {
    /// The user-agent stylesheet.
    UserAgent,
    /// The user's stylesheet.
    User,
    /// The document author's stylesheets and `style=` attributes.
    Author,
}

impl Origin {
    /// The cascade precedence of this origin: lower sorts first (weaker).
    #[must_use]
    pub const fn precedence(self) -> u8 {
        match self {
            Self::UserAgent => 0,
            Self::User => 1,
            Self::Author => 2,
        }
    }

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::UserAgent => "user-agent",
            Self::User => "user",
            Self::Author => "author",
        }
    }
}

impl fmt::Display for Origin {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name())
    }
}

/// The declarations of one rule, in source order. A first-class collection —
/// no public `Vec`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DeclarationBlock {
    declarations: Vec<(String, String)>,
}

impl DeclarationBlock {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            declarations: Vec::new(),
        }
    }

    /// Appends a `property: value` declaration.
    pub fn declare(&mut self, property: impl Into<String>, value: impl Into<String>) {
        self.declarations.push((property.into(), value.into()));
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> + '_ {
        self.declarations
            .iter()
            .map(|(property, value)| (property.as_str(), value.as_str()))
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.declarations.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.declarations.is_empty()
    }
}

/// One rule: a selector as written, and its declarations.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct StyleRule {
    selector_text: String,
    declarations: DeclarationBlock,
}

impl StyleRule {
    #[must_use]
    pub fn new(selector_text: impl Into<String>, declarations: DeclarationBlock) -> Self {
        Self {
            selector_text: selector_text.into(),
            declarations,
        }
    }

    #[must_use]
    pub fn selector_text(&self) -> &str {
        &self.selector_text
    }

    #[must_use]
    pub const fn declarations(&self) -> &DeclarationBlock {
        &self.declarations
    }
}

/// A rule tagged with the origin it came from.
#[derive(Clone, Debug, PartialEq, Eq)]
struct OriginRule {
    origin: Origin,
    rule: StyleRule,
}

/// Every rule that could apply to a document, in cascade order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct StyleSheetSet {
    rules: Vec<OriginRule>,
}

impl StyleSheetSet {
    /// An empty set — the input B0's `UaCascade` is given (its rules are
    /// hard-coded in Rust).
    #[must_use]
    pub const fn new() -> Self {
        Self { rules: Vec::new() }
    }

    /// Appends a rule for `origin`.
    pub fn push_rule(&mut self, origin: Origin, rule: StyleRule) {
        self.rules.push(OriginRule { origin, rule });
    }

    pub fn rules(&self) -> impl Iterator<Item = (Origin, &StyleRule)> + '_ {
        self.rules.iter().map(|entry| (entry.origin, &entry.rule))
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.rules.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }
}

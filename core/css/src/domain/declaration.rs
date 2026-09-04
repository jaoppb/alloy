//! [`Declaration`] — one `property: value` pair, and the [`DeclarationBlock`]
//! that collects them.
//!
//! B0 held these as raw `(String, String)` pairs and said so
//! (`stylesheet_set.rs` doc-comment, B0): "B1 replaces these with parsed
//! selector and declaration types". This is that replacement. The value stays
//! **text** — turning it into a computed value is the cascade's job, and B2
//! owns the unit and colour vocabulary — but the property is a validated
//! [`Identifier`] and `!important` is preserved as [`Importance`] rather than
//! dropped, because a silently discarded `!important` is exactly the kind of
//! quiet shrinkage the declared cut forbids (`relatório §2.8:350-354`).

use core::fmt;

use crate::domain::identifier::Identifier;

/// Whether the author wrote `!important` (CSS Cascade L4 §6.2). B1 preserves
/// the flag; B2 is the phase that lets it win the cascade.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Importance {
    /// An ordinary declaration.
    #[default]
    Normal,
    /// `!important`.
    Important,
}

impl Importance {
    #[must_use]
    pub const fn is_important(self) -> bool {
        matches!(self, Self::Important)
    }
}

impl fmt::Display for Importance {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Normal => Ok(()),
            Self::Important => formatter.write_str(" !important"),
        }
    }
}

/// A declaration's value as written, with runs of whitespace collapsed to one
/// space and the ends trimmed — so `margin:  4px   8px` and `margin:4px 8px`
/// produce the same value and the same computed style.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct DeclarationValue {
    text: String,
}

impl DeclarationValue {
    #[must_use]
    pub fn new(text: &str) -> Self {
        Self {
            text: text.split_whitespace().collect::<Vec<&str>>().join(" "),
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.text
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.text.is_empty()
    }
}

impl fmt::Display for DeclarationValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.text)
    }
}

/// One declaration of a rule.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Declaration {
    property: Identifier,
    value: DeclarationValue,
    importance: Importance,
}

impl Declaration {
    #[must_use]
    pub const fn new(
        property: Identifier,
        value: DeclarationValue,
        importance: Importance,
    ) -> Self {
        Self {
            property,
            value,
            importance,
        }
    }

    #[must_use]
    pub const fn property(&self) -> &Identifier {
        &self.property
    }

    #[must_use]
    pub const fn value(&self) -> &DeclarationValue {
        &self.value
    }

    #[must_use]
    pub const fn importance(&self) -> Importance {
        self.importance
    }
}

impl fmt::Display for Declaration {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}: {}{}",
            self.property, self.value, self.importance
        )
    }
}

/// The declarations of one rule, in source order. A first-class collection —
/// no public `Vec` (`ADR-0010:129`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct DeclarationBlock {
    declarations: Vec<Declaration>,
}

impl DeclarationBlock {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            declarations: Vec::new(),
        }
    }

    pub fn push(&mut self, declaration: Declaration) {
        self.declarations.push(declaration);
    }

    /// Appends an ordinary `property: value` declaration, ignoring a property
    /// name no identifier can hold. The convenience a hand-built fixture and
    /// the UA sheet use; the parser builds [`Declaration`]s itself so it can
    /// attach a span to the refusal.
    pub fn declare(&mut self, property: &str, value: &str) {
        let Some(name) = Identifier::lowercased(property) else {
            return;
        };
        self.push(Declaration::new(
            name,
            DeclarationValue::new(value),
            Importance::Normal,
        ));
    }

    pub fn iter(&self) -> impl Iterator<Item = &Declaration> + '_ {
        self.declarations.iter()
    }

    /// The last declaration of `property`, which is the one that wins inside a
    /// single block (CSS Cascade L4 §6.4.4, "order of appearance").
    #[must_use]
    pub fn last_of(&self, property: &str) -> Option<&Declaration> {
        self.declarations
            .iter()
            .rev()
            .find(|declaration| declaration.property().as_str() == property)
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

impl fmt::Display for DeclarationBlock {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut separator = "";
        for declaration in &self.declarations {
            formatter.write_str(separator)?;
            declaration.fmt(formatter)?;
            separator = "; ";
        }
        Ok(())
    }
}

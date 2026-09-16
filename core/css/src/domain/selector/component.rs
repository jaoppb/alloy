//! The leaf components of a compound selector: the type selector, an attribute
//! selector, a pseudo-class, and the `an+b` formula `:nth-child()` carries.
//!
//! The cut is `docs/reports/IMPLEMENTACAO-DETALHADA-V0-5.md` §2.8 exactly —
//! `[attr]` and `[attr=v]` but no `^=` / `$=` / `*=`; `:hover` / `:active` /
//! `:focus` / `:first-child` / `:last-child` / `:nth-child()` but no `:has()`
//! and no pseudo-*element*. What is outside is refused by the parser with a
//! `CssError`, never accepted and ignored (`relatório §2.8:350-354`).

use core::fmt;

use crate::domain::identifier::Identifier;
use crate::domain::specificity::Specificity;

/// The element-name half of a compound selector.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum TypeSelector {
    /// `*` — matches every element and contributes nothing to specificity.
    #[default]
    Universal,
    /// A tag name, ASCII-lowercased for HTML matching.
    Named(Identifier),
}

impl TypeSelector {
    /// `(0,0,1)` for a named type, `(0,0,0)` for `*` (CSS Selectors L4 §17).
    #[must_use]
    pub const fn specificity(&self) -> Specificity {
        match self {
            Self::Universal => Specificity::ZERO,
            Self::Named(_) => Specificity::type_name(),
        }
    }
}

impl fmt::Display for TypeSelector {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Universal => formatter.write_str("*"),
            Self::Named(name) => name.fmt(formatter),
        }
    }
}

/// How an attribute selector compares. `#[non_exhaustive]`: `^=` / `$=` / `*=`
/// / `~=` are declared out of v0.5 and arrive behind this enum, not beside it.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AttributeMatch {
    /// `[attr]` — the attribute is present, whatever its value.
    Exists,
    /// `[attr=value]` — the attribute's value is exactly this string.
    Exact(String),
}

/// One `[…]` component.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AttributeSelector {
    name: Identifier,
    match_kind: AttributeMatch,
}

impl AttributeSelector {
    #[must_use]
    pub const fn new(name: Identifier, match_kind: AttributeMatch) -> Self {
        Self { name, match_kind }
    }

    #[must_use]
    pub const fn name(&self) -> &Identifier {
        &self.name
    }

    #[must_use]
    pub const fn match_kind(&self) -> &AttributeMatch {
        &self.match_kind
    }
}

impl fmt::Display for AttributeSelector {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.match_kind {
            AttributeMatch::Exists => write!(formatter, "[{}]", self.name),
            AttributeMatch::Exact(value) => write!(formatter, "[{}=\"{value}\"]", self.name),
        }
    }
}

/// The `an+b` formula of `:nth-child()` (CSS Selectors L4 §6.6.3).
///
/// `step` is `a`, `offset` is `b`. `odd` is `2n+1`, `even` is `2n`, a bare
/// integer `k` is `0n+k`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct NthFormula {
    step: i32,
    offset: i32,
}

impl NthFormula {
    #[must_use]
    pub const fn new(step: i32, offset: i32) -> Self {
        Self { step, offset }
    }

    #[must_use]
    pub const fn step(self) -> i32 {
        self.step
    }

    #[must_use]
    pub const fn offset(self) -> i32 {
        self.offset
    }

    /// Whether a 1-based element index satisfies `an+b` for some `n >= 0`.
    ///
    /// Every step is `checked_*`: `arithmetic_side_effects` is denied
    /// (`Cargo.toml:73`) and a stylesheet can name `:nth-child(-2147483648n+1)`.
    #[must_use]
    pub fn matches(self, index_one_based: u32) -> bool {
        let Ok(index) = i32::try_from(index_one_based) else {
            return false;
        };
        let Some(shifted) = index.checked_sub(self.offset) else {
            return false;
        };
        if self.step == 0 {
            return shifted == 0;
        }
        self.divides_evenly(shifted)
    }

    /// Whether `shifted` is a non-negative multiple of `step`.
    const fn divides_evenly(self, shifted: i32) -> bool {
        let Some(remainder) = shifted.checked_rem(self.step) else {
            return false;
        };
        let Some(quotient) = shifted.checked_div(self.step) else {
            return false;
        };
        remainder == 0 && quotient >= 0
    }
}

impl fmt::Display for NthFormula {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}n+{}", self.step, self.offset)
    }
}

/// The pseudo-classes inside the v0.5 cut.
///
/// `#[non_exhaustive]`: `:not()`, `:has()` and the rest arrive behind this
/// enum. The three interaction states parse and count towards specificity but
/// never match — a [`crate::DomSnapshot`] projects elements, attributes and
/// tree shape (`PRD-007:35-36`), not interaction state, which the engine does
/// not have until the window and event phases.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PseudoClass {
    /// `:hover` — parsed, never matched in v0.5.
    Hover,
    /// `:active` — parsed, never matched in v0.5.
    Active,
    /// `:focus` — parsed, never matched in v0.5.
    Focus,
    /// `:first-child`.
    FirstChild,
    /// `:last-child`.
    LastChild,
    /// `:nth-child(an+b)`.
    NthChild(NthFormula),
}

impl PseudoClass {
    /// The keyword as it appears in a stylesheet, without its arguments.
    #[must_use]
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::Hover => "hover",
            Self::Active => "active",
            Self::Focus => "focus",
            Self::FirstChild => "first-child",
            Self::LastChild => "last-child",
            Self::NthChild(_) => "nth-child",
        }
    }
}

impl fmt::Display for PseudoClass {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NthChild(formula) => write!(formatter, ":nth-child({formula})"),
            other => write!(formatter, ":{}", other.keyword()),
        }
    }
}

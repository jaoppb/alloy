//! [`ComplexSelector`] and [`SelectorList`] — compounds joined by combinators,
//! and the comma-separated list of them one rule carries.
//!
//! A complex selector is stored as a sequence of [`SelectorStep`]s, each a
//! combinator plus the compound that follows it. The **first** step's
//! combinator is always [`Combinator::Descendant`] and carries no meaning:
//! matching runs right to left and stops once step `0` is satisfied, so nothing
//! ever reads it. Storing it anyway keeps the sequence uniform and the matcher
//! free of a special case.

use core::fmt::{self, Display};

use crate::domain::selector::compound::CompoundSelector;
use crate::domain::specificity::Specificity;

/// How two adjacent compounds relate (CSS Selectors L4 §15) — the four of the
/// v0.5 cut (`relatório §2.8:342`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Combinator {
    /// `A B` — B is a descendant of A.
    #[default]
    Descendant,
    /// `A > B` — B is a child of A.
    Child,
    /// `A + B` — B is the next element sibling of A.
    NextSibling,
    /// `A ~ B` — B is a later element sibling of A.
    SubsequentSibling,
}

impl Combinator {
    /// The combinator as written, with the spacing a stylesheet uses.
    #[must_use]
    pub const fn symbol(self) -> &'static str {
        match self {
            Self::Descendant => " ",
            Self::Child => " > ",
            Self::NextSibling => " + ",
            Self::SubsequentSibling => " ~ ",
        }
    }
}

impl fmt::Display for Combinator {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.symbol())
    }
}

/// One compound and the combinator that binds it to the compound before it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct SelectorStep {
    combinator: Combinator,
    compound: CompoundSelector,
}

impl SelectorStep {
    #[must_use]
    pub const fn new(combinator: Combinator, compound: CompoundSelector) -> Self {
        Self {
            combinator,
            compound,
        }
    }

    #[must_use]
    pub const fn combinator(&self) -> Combinator {
        self.combinator
    }

    #[must_use]
    pub const fn compound(&self) -> &CompoundSelector {
        &self.compound
    }
}

/// A sequence of compounds joined by combinators — `nav > ul li.current`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct ComplexSelector {
    steps: Vec<SelectorStep>,
}

impl ComplexSelector {
    /// A selector made of the given steps, in source order.
    #[must_use]
    pub fn new(steps: impl IntoIterator<Item = SelectorStep>) -> Self {
        Self {
            steps: steps.into_iter().collect(),
        }
    }

    pub fn steps(&self) -> impl Iterator<Item = &SelectorStep> + '_ {
        self.steps.iter()
    }

    /// The step at `index`, counting from the left.
    #[must_use]
    pub fn step(&self, index: usize) -> Option<&SelectorStep> {
        self.steps.get(index)
    }

    /// The index of the rightmost step — the **subject** of the selector, the
    /// element a match is about.
    #[must_use]
    pub const fn subject_index(&self) -> Option<usize> {
        self.steps.len().checked_sub(1)
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.steps.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// The sum of every compound's specificity (CSS Selectors L4 §17).
    #[must_use]
    pub fn specificity(&self) -> Specificity {
        self.steps
            .iter()
            .map(|step| step.compound.specificity())
            .fold(Specificity::ZERO, Specificity::plus)
    }
}

impl fmt::Display for ComplexSelector {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, step) in self.steps.iter().enumerate() {
            write_step(index, step, formatter)?;
        }
        Ok(())
    }
}

/// The leading step prints bare; every later one is prefixed by its combinator.
fn write_step(
    index: usize,
    step: &SelectorStep,
    formatter: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    if index > 0 {
        step.combinator().fmt(formatter)?;
    }
    step.compound().fmt(formatter)
}

/// The comma-separated selectors of one rule — `h1, h2, .title`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct SelectorList {
    selectors: Vec<ComplexSelector>,
}

impl SelectorList {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            selectors: Vec::new(),
        }
    }

    pub fn push(&mut self, selector: ComplexSelector) {
        self.selectors.push(selector);
    }

    pub fn iter(&self) -> impl Iterator<Item = &ComplexSelector> + '_ {
        self.selectors.iter()
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.selectors.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.selectors.is_empty()
    }
}

impl FromIterator<ComplexSelector> for SelectorList {
    fn from_iter<I: IntoIterator<Item = ComplexSelector>>(selectors: I) -> Self {
        Self {
            selectors: selectors.into_iter().collect(),
        }
    }
}

impl fmt::Display for SelectorList {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut separator = "";
        for selector in &self.selectors {
            formatter.write_str(separator)?;
            selector.fmt(formatter)?;
            separator = ", ";
        }
        Ok(())
    }
}

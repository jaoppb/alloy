//! [`MediaQuery`] — the `@media` conditions of the v0.5 cut: `min-width` and
//! `max-width` (`relatório §2.8:344`).
//!
//! The query lives on a [`crate::StyleRule`], not on the cascade, because
//! [`crate::CascadeResolver::resolve`] is handed a `DomSnapshot` and a
//! `StyleSheetSet` and **no** viewport (`PRD-007:56-60`, frozen at I3).
//! Evaluating a media query is therefore the **producer's** job:
//! [`crate::StyleSheetSet::matching_viewport`] is the query that keeps the
//! rules whose conditions hold and drops the rest, and a resolver skips any
//! rule still carrying a condition — the safe default for a condition nobody
//! evaluated.

use core::fmt;

use graphics::Au;

use crate::domain::length::Length;
use crate::domain::viewport::ViewportConstraints;

/// The CSS `initial` `font-size`, `16px`, in [`Au`]. A media query is evaluated
/// against the initial font size: there is no element to inherit one from.
const INITIAL_FONT_SIZE: Au = match Au::from_whole_px(16) {
    Some(size) => size,
    None => Au::ZERO,
};

/// Which viewport dimension a condition constrains.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MediaFeature {
    /// `min-width` — holds when the viewport is at least this wide.
    MinWidth,
    /// `max-width` — holds when the viewport is at most this wide.
    MaxWidth,
}

impl MediaFeature {
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::MinWidth => "min-width",
            Self::MaxWidth => "max-width",
        }
    }
}

impl fmt::Display for MediaFeature {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name())
    }
}

/// One `(feature: length)` pair.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MediaCondition {
    feature: MediaFeature,
    length: Length,
}

impl MediaCondition {
    #[must_use]
    pub const fn new(feature: MediaFeature, length: Length) -> Self {
        Self { feature, length }
    }

    #[must_use]
    pub const fn feature(self) -> MediaFeature {
        self.feature
    }

    #[must_use]
    pub const fn length(self) -> Length {
        self.length
    }

    /// Whether this condition holds for `constraints`.
    ///
    /// A length that cannot resolve (a non-finite magnitude) makes the
    /// condition false rather than an error: a media query is a filter, and an
    /// unreadable filter selects nothing.
    #[must_use]
    pub fn matches(self, constraints: &ViewportConstraints) -> bool {
        let width = constraints.width();
        let Some(threshold) = self.length.resolve_to_au(INITIAL_FONT_SIZE, width) else {
            return false;
        };
        match self.feature {
            MediaFeature::MinWidth => width >= threshold,
            MediaFeature::MaxWidth => width <= threshold,
        }
    }
}

impl fmt::Display for MediaCondition {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "({}: {})", self.feature, self.length)
    }
}

/// The conditions a rule is gated on — a first-class collection, empty for a
/// rule written outside any `@media` block.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct MediaQuery {
    conditions: Vec<MediaCondition>,
}

impl MediaQuery {
    /// The unconditional query: a rule outside every `@media` block.
    #[must_use]
    pub const fn always() -> Self {
        Self {
            conditions: Vec::new(),
        }
    }

    pub fn push(&mut self, condition: MediaCondition) {
        self.conditions.push(condition);
    }

    pub fn iter(&self) -> impl Iterator<Item = MediaCondition> + '_ {
        self.conditions.iter().copied()
    }

    /// How many conditions the query joins. Named for the conditions rather
    /// than spelled `len`, because "empty" is not the interesting question here
    /// — [`MediaQuery::is_always`] is.
    #[must_use]
    pub const fn condition_count(&self) -> usize {
        self.conditions.len()
    }

    /// Whether the rule is unconditional. Named for what it means to the
    /// cascade, not for the emptiness of the vector behind it.
    #[must_use]
    pub const fn is_always(&self) -> bool {
        self.conditions.is_empty()
    }

    /// Whether **every** condition holds — `@media` joins them with `and`.
    #[must_use]
    pub fn matches(&self, constraints: &ViewportConstraints) -> bool {
        self.conditions
            .iter()
            .all(|condition| condition.matches(constraints))
    }
}

impl fmt::Display for MediaQuery {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut separator = "";
        for condition in &self.conditions {
            formatter.write_str(separator)?;
            condition.fmt(formatter)?;
            separator = " and ";
        }
        Ok(())
    }
}

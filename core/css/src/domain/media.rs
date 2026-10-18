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

use core::fmt::{self, Display};

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

/// The conditions a rule is gated on.
///
/// An enum modeling the three mutually exclusive states:
/// - [`MediaQuery::Always`]: unconditional rule outside every `@media` block, or `media="all"` / `media="screen"`.
/// - [`MediaQuery::Never`]: query that never matches, such as `media="print"`, `media="speech"`, or unparseable queries.
/// - [`MediaQuery::Conditions`]: rule gated on a sequence of conditions joined by `and`.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub enum MediaQuery {
    /// The unconditional query: a rule outside every `@media` block.
    #[default]
    Always,
    /// An impossible query that never matches: unparseable media expressions or
    /// media types outside screen/all (e.g. `print`, `speech`).
    Never,
    /// The rule is gated on a sequence of conditions joined by `and`.
    Conditions(Vec<MediaCondition>),
}

impl MediaQuery {
    /// The unconditional query: a rule outside every `@media` block.
    #[must_use]
    pub const fn always() -> Self {
        Self::Always
    }

    /// An impossible query that never matches: unparseable media expressions or
    /// media types outside screen/all (e.g. `print`, `speech`).
    #[must_use]
    pub const fn never() -> Self {
        Self::Never
    }

    pub fn push(&mut self, condition: MediaCondition) {
        match self {
            Self::Always => *self = Self::Conditions(vec![condition]),
            Self::Conditions(conditions) => conditions.push(condition),
            Self::Never => {}
        }
    }

    const fn conditions_slice(&self) -> &[MediaCondition] {
        match self {
            Self::Conditions(conditions) => conditions.as_slice(),
            Self::Always | Self::Never => &[],
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = MediaCondition> + '_ {
        self.conditions_slice().iter().copied()
    }

    /// How many conditions the query joins. Named for the conditions rather
    /// than spelled `len`, because "empty" is not the interesting question here
    /// — [`MediaQuery::is_always`] is.
    #[must_use]
    pub const fn condition_count(&self) -> usize {
        match self {
            Self::Conditions(conditions) => conditions.len(),
            Self::Always | Self::Never => 0,
        }
    }

    /// Whether the rule is unconditional. Named for what it means to the
    /// cascade, not for the emptiness of the vector behind it.
    #[must_use]
    pub const fn is_always(&self) -> bool {
        match self {
            Self::Always => true,
            Self::Conditions(conditions) => conditions.is_empty(),
            Self::Never => false,
        }
    }

    /// Whether the query never matches any environment.
    #[must_use]
    pub const fn is_never(&self) -> bool {
        matches!(self, Self::Never)
    }

    /// Joins `other` with `self`. Both queries must hold for the combined query to match.
    #[must_use]
    pub fn and(&self, other: &Self) -> Self {
        match (self, other) {
            (Self::Never, _) | (_, Self::Never) => Self::Never,
            (Self::Always, query) | (query, Self::Always) => query.clone(),
            (Self::Conditions(first), Self::Conditions(second)) => {
                combine_conditions(first, second)
            }
        }
    }

    /// Whether **every** condition holds — `@media` joins them with `and`.
    #[must_use]
    pub fn matches(&self, constraints: &ViewportConstraints) -> bool {
        match self {
            Self::Always => true,
            Self::Never => false,
            Self::Conditions(conditions) => conditions_match(conditions, *constraints),
        }
    }
}

fn combine_conditions(first: &[MediaCondition], second: &[MediaCondition]) -> MediaQuery {
    if first.is_empty() && second.is_empty() {
        return MediaQuery::Always;
    }
    if first.is_empty() {
        return MediaQuery::Conditions(second.to_vec());
    }
    if second.is_empty() {
        return MediaQuery::Conditions(first.to_vec());
    }
    let capacity = first.len().saturating_add(second.len());
    let mut combined = Vec::with_capacity(capacity);
    combined.extend_from_slice(first);
    combined.extend_from_slice(second);
    MediaQuery::Conditions(combined)
}

fn conditions_match(conditions: &[MediaCondition], constraints: ViewportConstraints) -> bool {
    conditions
        .iter()
        .all(|condition| condition.matches(&constraints))
}

impl fmt::Display for MediaQuery {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Never => formatter.write_str("not all"),
            Self::Always => Ok(()),
            Self::Conditions(conditions) => format_conditions(formatter, conditions),
        }
    }
}

fn format_conditions(
    formatter: &mut fmt::Formatter<'_>,
    conditions: &[MediaCondition],
) -> fmt::Result {
    let mut separator = "";
    for condition in conditions {
        formatter.write_str(separator)?;
        condition.fmt(formatter)?;
        separator = " and ";
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn px(pixels: i32) -> Au {
        Au::from_whole_px(pixels).unwrap()
    }

    #[test]
    fn always_and_never_predicates() {
        let always = MediaQuery::always();
        let never = MediaQuery::never();
        let constraints = ViewportConstraints::new(px(400), px(300));

        assert!(always.is_always());
        assert!(!always.is_never());
        assert!(always.matches(&constraints));
        assert_eq!(always, MediaQuery::Always);

        assert!(!never.is_always());
        assert!(never.is_never());
        assert!(!never.matches(&constraints));
        assert_eq!(never, MediaQuery::Never);
        assert_eq!(never.to_string(), "not all");
    }

    #[test]
    fn combining_queries_with_and() {
        let always = MediaQuery::always();
        let never = MediaQuery::never();
        let condition = MediaCondition::new(MediaFeature::MinWidth, Length::pixels(300.0));
        let mut query = MediaQuery::always();
        query.push(condition);

        assert_eq!(always.and(&query), query);
        assert_eq!(query.and(&always), query);
        assert_eq!(never.and(&query), never);
        assert_eq!(query.and(&never), never);

        let condition2 = MediaCondition::new(MediaFeature::MaxWidth, Length::pixels(600.0));
        let mut query2 = MediaQuery::always();
        query2.push(condition2);

        let combined = query.and(&query2);
        assert_eq!(combined.condition_count(), 2);
        assert!(!combined.is_always());
        assert!(!combined.is_never());

        let constraints_in = ViewportConstraints::new(px(400), px(300));
        let constraints_out = ViewportConstraints::new(px(200), px(300));
        assert!(combined.matches(&constraints_in));
        assert!(!combined.matches(&constraints_out));
    }

    #[test]
    fn enum_variants_and_iteration() {
        assert_eq!(MediaQuery::default(), MediaQuery::Always);

        let condition = MediaCondition::new(MediaFeature::MinWidth, Length::pixels(250.0));
        let query = MediaQuery::Conditions(vec![condition]);
        assert_eq!(query.condition_count(), 1);
        let items: Vec<_> = query.iter().collect();
        assert_eq!(items, vec![condition]);

        let mut never = MediaQuery::never();
        never.push(condition);
        assert_eq!(never, MediaQuery::Never);
    }
}

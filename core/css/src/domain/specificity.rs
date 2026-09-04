//! [`Specificity`] — the three-component weight the cascade sorts by
//! (`relatório §2.8:334`, CSS Selectors L4 §17).
//!
//! `(ids, classes, types)`, compared left to right. The field order **is** the
//! comparison order: `derive(Ord)` on a struct compares fields in declaration
//! order, which is exactly the lexicographic rule CSS defines, so the cascade
//! sorts by `Specificity` with no hand-written comparator to get wrong.

use core::fmt;

/// The weight of one selector: id count, class-like count, type count.
///
/// "Class-like" is classes, attribute selectors and pseudo-classes together —
/// CSS gives all three the same column. The universal selector `*` contributes
/// nothing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Specificity {
    ids: u16,
    classes: u16,
    types: u16,
}

impl Specificity {
    /// No weight at all — `*`, and the identity of [`Specificity::plus`].
    pub const ZERO: Self = Self {
        ids: 0,
        classes: 0,
        types: 0,
    };

    #[must_use]
    pub const fn new(ids: u16, classes: u16, types: u16) -> Self {
        Self {
            ids,
            classes,
            types,
        }
    }

    /// One `#id`.
    #[must_use]
    pub const fn id() -> Self {
        Self::new(1, 0, 0)
    }

    /// One `.class`, `[attr]` or `:pseudo-class`.
    #[must_use]
    pub const fn class() -> Self {
        Self::new(0, 1, 0)
    }

    /// One type selector.
    #[must_use]
    pub const fn type_name() -> Self {
        Self::new(0, 0, 1)
    }

    /// From three counts that may not fit a `u16`. A selector with more than
    /// 65535 classes saturates rather than wrapping — wrapping would make a
    /// pathological stylesheet *win* the cascade.
    #[must_use]
    pub fn from_counts(ids: usize, classes: usize, types: usize) -> Self {
        Self {
            ids: saturate(ids),
            classes: saturate(classes),
            types: saturate(types),
        }
    }

    /// Component-wise sum, saturating for the same reason as
    /// [`Specificity::from_counts`].
    #[must_use]
    pub const fn plus(self, other: Self) -> Self {
        Self {
            ids: self.ids.saturating_add(other.ids),
            classes: self.classes.saturating_add(other.classes),
            types: self.types.saturating_add(other.types),
        }
    }

    #[must_use]
    pub const fn ids(self) -> u16 {
        self.ids
    }

    #[must_use]
    pub const fn classes(self) -> u16 {
        self.classes
    }

    #[must_use]
    pub const fn types(self) -> u16 {
        self.types
    }
}

fn saturate(count: usize) -> u16 {
    u16::try_from(count).unwrap_or(u16::MAX)
}

impl fmt::Display for Specificity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "({},{},{})", self.ids, self.classes, self.types)
    }
}

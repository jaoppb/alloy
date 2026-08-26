use std::cmp::Ordering;
use std::ops::Add;

/// CSS selector specificity tuple: `(ID count, Class/Attribute count, Tag count)` (W3C CSS Cascading).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Specificity {
    ids: u32,
    classes: u32,
    tags: u32,
}

impl Specificity {
    /// Creates a new specificity tuple.
    #[must_use]
    pub const fn new(ids: u32, classes: u32, tags: u32) -> Self {
        Self { ids, classes, tags }
    }

    /// Returns the ID selector count.
    #[must_use]
    pub const fn ids(self) -> u32 {
        self.ids
    }

    /// Returns the class, attribute, and pseudo-class selector count.
    #[must_use]
    pub const fn classes(self) -> u32 {
        self.classes
    }

    /// Returns the element and pseudo-element selector count.
    #[must_use]
    pub const fn tags(self) -> u32 {
        self.tags
    }

    /// Returns the specificity as a 3-tuple `(a, b, c)`.
    #[must_use]
    pub const fn as_tuple(self) -> (u32, u32, u32) {
        (self.ids, self.classes, self.tags)
    }
}

impl PartialOrd for Specificity {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Specificity {
    fn cmp(&self, other: &Self) -> Ordering {
        self.ids
            .cmp(&other.ids)
            .then_with(|| self.classes.cmp(&other.classes))
            .then_with(|| self.tags.cmp(&other.tags))
    }
}

impl Add for Specificity {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            ids: self.ids + rhs.ids,
            classes: self.classes + rhs.classes,
            tags: self.tags + rhs.tags,
        }
    }
}

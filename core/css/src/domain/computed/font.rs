//! The computed value of `font-family` (v0.5 fonts increment).
//!
//! `font-family` is a **list** — an author writes `"Helvetica Neue", Arial,
//! sans-serif` and the first family that resolves to an installed or fetched
//! face wins. [`crate::ComputedStyle`] is `Copy` (and its `with_*` builders are
//! `const fn`), so the list here cannot be a `Vec`: [`FontFamilyList`] is a
//! fixed-capacity `Copy` collection and [`FamilyName`] is a fixed-capacity
//! `Copy` string. Two cuts, both documented in `tests/data/MANIFEST.md` beside
//! the Flexbox cuts:
//!
//! - at most [`FontFamilyList::CAPACITY`] families are kept (a longer fallback
//!   chain is truncated),
//! - a family name longer than [`FamilyName::CAPACITY`] bytes is truncated at a
//!   UTF-8 boundary.
//!
//! Neither is recorded as a `ParseNote`: the value is only parsed at cascade
//! time, which has no note channel, so — like the Flexbox simplifications —
//! they are declared once in the manifest rather than reported per occurrence.

use core::fmt;

/// One of CSS's generic font families.
///
/// `cursive` / `fantasy` / `system-ui` and the rest are outside the cut — an
/// author naming one gets a [`FamilyName`] that simply will not resolve, and
/// the next family in the list is tried.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum GenericFamily {
    /// `serif`.
    Serif,
    /// `sans-serif`.
    SansSerif,
    /// `monospace`.
    Monospace,
}

impl GenericFamily {
    /// The keyword as it appears in a stylesheet.
    #[must_use]
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::Serif => "serif",
            Self::SansSerif => "sans-serif",
            Self::Monospace => "monospace",
        }
    }
}

impl fmt::Display for GenericFamily {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.keyword())
    }
}

/// A `Copy` inline family name.
///
/// Handles `"Helvetica Neue"`, `Georgia`, a fetched `@font-face` family. Bytes
/// past [`Self::CAPACITY`] are dropped at a UTF-8 boundary; the unused tail is
/// zeroed so the derived `PartialEq` / `Hash` see two equal names as equal.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct FamilyName {
    bytes: [u8; Self::CAPACITY],
    len: u8,
}

impl FamilyName {
    /// The longest family name this cut stores, in bytes. Kept small on purpose:
    /// [`crate::ComputedStyle`] is a `Copy` aggregate copied per node during
    /// layout, and clippy's `large_types_passed_by_value` caps it at 256 bytes.
    pub const CAPACITY: usize = 23;

    /// The name, truncated to [`Self::CAPACITY`] bytes at a UTF-8 boundary.
    #[must_use]
    pub fn new(name: &str) -> Self {
        let kept = truncate_on_boundary(name, Self::CAPACITY);
        let mut bytes = [0_u8; Self::CAPACITY];
        for (slot, &byte) in bytes.iter_mut().zip(kept.as_bytes()) {
            *slot = byte;
        }
        Self {
            bytes,
            len: u8::try_from(kept.len()).unwrap_or(0),
        }
    }

    /// The stored name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        let end = usize::from(self.len);
        core::str::from_utf8(self.bytes.get(..end).unwrap_or(&[])).unwrap_or("")
    }
}

impl fmt::Display for FamilyName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl fmt::Debug for FamilyName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "FamilyName({:?})", self.as_str())
    }
}

/// The longest prefix of `name` that fits in `max` bytes and ends on a UTF-8
/// character boundary.
fn truncate_on_boundary(name: &str, max: usize) -> &str {
    if name.len() <= max {
        return name;
    }
    let mut end = max;
    while end > 0 && !name.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    name.get(..end).unwrap_or("")
}

/// One entry of a `font-family` list: a specific face, or a generic to hand to
/// the platform.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FontFamily {
    /// A named family — resolved against installed faces and `@font-face`.
    Named(FamilyName),
    /// A CSS generic — resolved against the platform's default for that class.
    Generic(GenericFamily),
}

impl fmt::Display for FontFamily {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Named(name) => name.fmt(formatter),
            Self::Generic(generic) => generic.fmt(formatter),
        }
    }
}

/// The computed `font-family`: an ordered, fixed-capacity `Copy` list. Empty
/// means "no author preference" — the provider picks its default.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct FontFamilyList {
    entries: [Option<FontFamily>; Self::CAPACITY],
}

impl FontFamilyList {
    /// The longest fallback chain this cut keeps. A real chain ends in a
    /// generic, and the provider's own default is a generic, so dropping the
    /// tail past three entries and falling back to that default is equivalent
    /// for the common case. Kept small for the same `Copy`-size reason as
    /// [`FamilyName::CAPACITY`].
    pub const CAPACITY: usize = 3;

    /// The `initial` value: no family, i.e. defer to the provider.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            entries: [None; Self::CAPACITY],
        }
    }

    /// The first [`Self::CAPACITY`] families of `families`, in order.
    #[must_use]
    pub fn from_families(families: impl IntoIterator<Item = FontFamily>) -> Self {
        let mut entries = [None; Self::CAPACITY];
        for (slot, family) in entries.iter_mut().zip(families) {
            *slot = Some(family);
        }
        Self { entries }
    }

    /// Every family in order, specific and generic alike.
    pub fn iter(&self) -> impl Iterator<Item = FontFamily> + '_ {
        self.entries.iter().filter_map(|entry| *entry)
    }

    /// The first family, if any.
    #[must_use]
    pub fn primary(&self) -> Option<FontFamily> {
        self.iter().next()
    }

    /// Whether the author expressed no preference.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.iter().next().is_none()
    }
}

impl fmt::Display for FontFamilyList {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;
        for family in self.iter() {
            if !first {
                formatter.write_str(", ")?;
            }
            family.fmt(formatter)?;
            first = false;
        }
        Ok(())
    }
}

//! [`FontWeight`] — font weight representation (CSS Fonts L4 §2.4).

use core::fmt;

/// The weight (or boldness) of the font glyphs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FontWeight(u16);

impl FontWeight {
    /// Thin weight (100).
    pub const THIN: Self = Self(100);
    /// Extra light weight (200).
    pub const EXTRA_LIGHT: Self = Self(200);
    /// Light weight (300).
    pub const LIGHT: Self = Self(300);
    /// Normal / book weight (400) — the CSS `initial` value.
    pub const NORMAL: Self = Self(400);
    /// Medium weight (500).
    pub const MEDIUM: Self = Self(500);
    /// Semi bold weight (600).
    pub const SEMI_BOLD: Self = Self(600);
    /// Bold weight (700).
    pub const BOLD: Self = Self(700);
    /// Extra bold weight (800).
    pub const EXTRA_BOLD: Self = Self(800);
    /// Black / heavy weight (900).
    pub const BLACK: Self = Self(900);

    /// Constructs a [`FontWeight`] if `weight` is in the CSS valid range 1..=1000.
    #[must_use]
    pub const fn new(weight: u16) -> Option<Self> {
        if weight >= 1 && weight <= 1000 {
            return Some(Self(weight));
        }
        None
    }

    /// The raw numeric weight value.
    #[must_use]
    pub const fn value(self) -> u16 {
        self.0
    }

    /// Whether this weight is considered bold (>= 700).
    #[must_use]
    pub const fn is_bold(self) -> bool {
        self.0 >= 700
    }

    /// Calculates relative `bolder` weight based on CSS Fonts 4 §2.4.
    #[must_use]
    pub const fn bolder(self) -> Self {
        if self.0 < 400 {
            return Self(400);
        }
        if self.0 <= 500 {
            return Self(700);
        }
        Self(900)
    }

    /// Calculates relative `lighter` weight based on CSS Fonts 4 §2.4.
    #[must_use]
    pub const fn lighter(self) -> Self {
        if self.0 < 600 {
            return Self(100);
        }
        if self.0 <= 700 {
            return Self(400);
        }
        Self(700)
    }
}

impl Default for FontWeight {
    fn default() -> Self {
        Self::NORMAL
    }
}

impl fmt::Display for FontWeight {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            400 => formatter.write_str("normal"),
            700 => formatter.write_str("bold"),
            other => write!(formatter, "{other}"),
        }
    }
}

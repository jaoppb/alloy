//! Named grid area and grid line identifier.

use core::fmt;

/// Identifier for a named grid area or named grid line (CSS Grid L1 §7.3).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct GridAreaName {
    bytes: [u8; Self::CAPACITY],
    len: u8,
}

impl GridAreaName {
    /// Maximum identifier length in bytes.
    pub const CAPACITY: usize = 32;

    /// Validates and constructs a grid area name.
    #[must_use]
    pub fn new(name: impl AsRef<str>) -> Option<Self> {
        let text = name.as_ref().trim();
        let is_reserved = text.is_empty()
            || text.eq_ignore_ascii_case("auto")
            || text.eq_ignore_ascii_case("span")
            || text.eq_ignore_ascii_case("none");
        if is_reserved || text.len() > Self::CAPACITY {
            return None;
        }
        let mut bytes = [0u8; Self::CAPACITY];
        let lower = text.to_ascii_lowercase();
        let slice = bytes.get_mut(..lower.len())?;
        slice.copy_from_slice(lower.as_bytes());
        let len = u8::try_from(lower.len()).ok()?;
        Some(Self { bytes, len })
    }

    /// The name as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        let len = usize::from(self.len);
        let Some(slice) = self.bytes.get(..len) else {
            return "";
        };
        core::str::from_utf8(slice).unwrap_or("")
    }
}

impl fmt::Debug for GridAreaName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "GridAreaName({:?})", self.as_str())
    }
}

impl fmt::Display for GridAreaName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

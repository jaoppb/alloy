//! Background image types (`background-image`).

use core::fmt;

/// A fixed-capacity URL string for background images.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ImageSource {
    bytes: [u8; Self::CAPACITY],
    len: u8,
}

impl ImageSource {
    /// Maximum byte length for an inline background image URL.
    pub const CAPACITY: usize = 63;

    /// Stores `url`, truncating at a UTF-8 character boundary if needed.
    #[must_use]
    pub fn new(url: &str) -> Self {
        let kept = truncate_boundary(url, Self::CAPACITY);
        let mut bytes = [0_u8; Self::CAPACITY];
        for (slot, &byte) in bytes.iter_mut().zip(kept.as_bytes()) {
            *slot = byte;
        }
        let len = u8::try_from(kept.len()).unwrap_or(0);
        Self { bytes, len }
    }

    /// The stored URL string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        let end = usize::from(self.len);
        let slice = self.bytes.get(..end).unwrap_or(&[]);
        core::str::from_utf8(slice).unwrap_or("")
    }
}

impl fmt::Debug for ImageSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "ImageSource({:?})", self.as_str())
    }
}

impl fmt::Display for ImageSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

fn truncate_boundary(text: &str, max_bytes: usize) -> &str {
    if text.len() <= max_bytes {
        return text;
    }
    let mut end = max_bytes;
    while end > 0 && !text.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    text.get(..end).unwrap_or("")
}

/// The computed value of `background-image` (CSS Backgrounds & Borders L3 §3.1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BackgroundImage {
    /// No background image — the `initial` value.
    #[default]
    None,
    /// An image resource specified by URL.
    Url(ImageSource),
}

impl BackgroundImage {
    /// Whether no image is set (`none`).
    #[must_use]
    pub const fn is_none(self) -> bool {
        matches!(self, Self::None)
    }

    /// Returns the image source if this is a URL image.
    #[must_use]
    pub const fn url(self) -> Option<ImageSource> {
        match self {
            Self::None => None,
            Self::Url(source) => Some(source),
        }
    }
}

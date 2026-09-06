//! First-class collection of explicit grid track sizes (CSS Grid L1 §7.2).

use core::fmt;

use super::track_size::TrackSize;

/// First-class collection of explicit grid track sizes (CSS Grid L1 §7.2).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrackList {
    tracks: [TrackSize; Self::CAPACITY],
    len: u8,
}

impl Default for TrackList {
    fn default() -> Self {
        Self::none()
    }
}

impl TrackList {
    /// Maximum track count supported in inline fixed capacity.
    pub const CAPACITY: usize = 16;

    /// Empty track list — CSS `none`.
    #[must_use]
    pub const fn none() -> Self {
        Self {
            tracks: [TrackSize::Auto; Self::CAPACITY],
            len: 0,
        }
    }

    /// Creates a track list from a slice of track sizes.
    #[must_use]
    pub fn from_tracks(tracks: &[TrackSize]) -> Self {
        let mut array = [TrackSize::Auto; Self::CAPACITY];
        let count = tracks.len().min(Self::CAPACITY);
        if let (Some(dest), Some(src)) = (array.get_mut(..count), tracks.get(..count)) {
            dest.copy_from_slice(src);
        }
        let len = u8::try_from(count).unwrap_or(0);
        Self { tracks: array, len }
    }

    /// How many explicit tracks this list contains.
    #[must_use]
    pub fn len(&self) -> usize {
        usize::from(self.len)
    }

    /// Whether the track list is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Whether the track list represents `none`.
    #[must_use]
    pub const fn is_none(&self) -> bool {
        self.len == 0
    }

    /// Retrieves a track size by 0-based index.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<TrackSize> {
        if index >= usize::from(self.len) {
            return None;
        }
        self.tracks.get(index).copied()
    }

    /// Slice of active track sizes.
    #[must_use]
    pub fn tracks(&self) -> &[TrackSize] {
        let len = usize::from(self.len);
        self.tracks.get(..len).unwrap_or(&[])
    }
}

impl fmt::Display for TrackList {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_empty() {
            return formatter.write_str("none");
        }
        let slice = self.tracks();
        let mut iter = slice.iter();
        let Some(first) = iter.next() else {
            return Ok(());
        };
        write!(formatter, "{first}")?;
        for item in iter {
            write!(formatter, " {item}")?;
        }
        Ok(())
    }
}

//! A handle into the host-owned decoded-image store.
//!
//! Like [`crate::FontId`], this is an identifier rather than pixels: decoding is
//! the host's job, and a display list stays cheap to clone and to serialize.
//! The v0.3 backend refuses `DrawImage` outright — decoding a hostile format is
//! a separate decision from encoding one (v0.3 report §2.7).

use core::fmt;

/// An image already decoded and registered with the host.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ImageId(u32);

impl ImageId {
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

impl fmt::Display for ImageId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "image #{}", self.0)
    }
}

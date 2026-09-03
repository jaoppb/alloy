//! [`Framebuffer`] — the pixels a backend produces, and what a golden image
//! actually compares.
//!
//! The golden gate compares *this*, not a PNG file: decoding the reference
//! image and comparing pixel by pixel keeps the determinism gate independent of
//! the encoder (v0.3 report §2.5). A first-class collection over the byte
//! buffer, so no caller ever indexes a row by hand.

use crate::domain::color::Color;
use crate::domain::geometry::SurfaceSize;

/// How many bytes one pixel occupies: R, G, B, A.
pub const BYTES_PER_PIXEL: usize = 4;

/// A straight-alpha RGBA8 image in row-major order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Framebuffer {
    size: SurfaceSize,
    pixels: Vec<u8>,
}

impl Framebuffer {
    /// A buffer of `size`, every pixel `color`.
    ///
    /// Returns `None` when the buffer would not be addressable on this
    /// platform — the only way this can fail.
    #[must_use]
    pub fn filled(size: SurfaceSize, color: Color) -> Option<Self> {
        let count = size.pixel_count()?;
        let bytes = count.checked_mul(BYTES_PER_PIXEL)?;
        let channels = color.to_rgba8();
        let mut pixels = Vec::with_capacity(bytes);
        for _ in 0..count {
            pixels.extend_from_slice(&channels);
        }
        Some(Self { size, pixels })
    }

    #[must_use]
    pub const fn size(&self) -> SurfaceSize {
        self.size
    }

    #[must_use]
    pub const fn width(&self) -> u32 {
        self.size.width()
    }

    #[must_use]
    pub const fn height(&self) -> u32 {
        self.size.height()
    }

    /// The colour at `(column, row)`, or `None` outside the buffer.
    #[must_use]
    pub fn pixel(&self, column: u32, row: u32) -> Option<Color> {
        let offset = self.byte_offset(column, row)?;
        let channels = self
            .pixels
            .get(offset..offset.checked_add(BYTES_PER_PIXEL)?)?;
        let [red, green, blue, alpha] = <[u8; BYTES_PER_PIXEL]>::try_from(channels).ok()?;
        Some(Color::rgba(red, green, blue, alpha))
    }

    /// The raw buffer, for an encoder or a comparison.
    #[must_use]
    pub fn as_rgba8(&self) -> &[u8] {
        &self.pixels
    }

    /// Where `(column, row)` starts in the buffer, or `None` when it is outside.
    fn byte_offset(&self, column: u32, row: u32) -> Option<usize> {
        if column >= self.size.width() || row >= self.size.height() {
            return None;
        }
        let column = usize::try_from(column).ok()?;
        let row = usize::try_from(row).ok()?;
        let width = usize::try_from(self.size.width()).ok()?;
        let index = row.checked_mul(width)?.checked_add(column)?;
        index.checked_mul(BYTES_PER_PIXEL)
    }
}

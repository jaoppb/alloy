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

    /// Constructs a framebuffer directly from RGBA8 pixel bytes.
    ///
    /// Returns `None` if `pixels.len()` does not exactly match
    /// `size.pixel_count() * BYTES_PER_PIXEL`.
    #[must_use]
    pub fn from_rgba8(size: SurfaceSize, pixels: Vec<u8>) -> Option<Self> {
        let count = size.pixel_count()?;
        let bytes = count.checked_mul(BYTES_PER_PIXEL)?;
        if pixels.len() != bytes {
            return None;
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

    /// Replaces the colour at `(column, row)`.
    ///
    /// A coordinate outside the buffer is a no-op, never a panic: the
    /// rasterizer clips before it draws, and a defect there should show up as a
    /// wrong picture a golden image catches, not as a crashed render.
    ///
    /// Public because composing a buffer by hand is a legitimate need — the
    /// golden-image difference map does exactly this — and because gating a
    /// method on the `software-backend` feature would make the type's surface
    /// depend on which adapter happens to be linked.
    pub fn set_pixel(&mut self, column: u32, row: u32, color: Color) {
        let Some(offset) = self.byte_offset(column, row) else {
            return;
        };
        let Some(end) = offset.checked_add(BYTES_PER_PIXEL) else {
            return;
        };
        let Some(slot) = self.pixels.get_mut(offset..end) else {
            return;
        };
        slot.copy_from_slice(&color.to_rgba8());
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

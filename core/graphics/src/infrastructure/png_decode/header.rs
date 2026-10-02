//! PNG image header (`IHDR`) validation and representation (RFC 2083 §4.1.1).

use crate::infrastructure::png_decode::chunk::read_be_u32;
use crate::infrastructure::png_decode::error::PngDecodeError;

/// Maximum allowed dimension on either axis to prevent memory exhaustion.
pub(super) const MAXIMUM_IMAGE_DIMENSION: u32 = 16_384;

/// Color type 2: Truecolor with RGB (3 bytes per pixel).
pub(super) const COLOR_TYPE_RGB: u8 = 2;

/// Color type 6: Truecolor with Alpha RGBA (4 bytes per pixel).
pub(super) const COLOR_TYPE_RGBA: u8 = 6;

/// Bit depth 8: 8 bits per channel.
const BIT_DEPTH_8: u8 = 8;

/// Expected byte length of the `IHDR` chunk payload.
const IHDR_PAYLOAD_LENGTH: usize = 13;

/// The validated metadata extracted from the `IHDR` chunk.
pub(super) struct ImageHeader {
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) color_type: u8,
}

/// Parses and strictly validates the 13-byte `IHDR` chunk payload.
pub(super) fn parse_ihdr(payload: &[u8]) -> Result<ImageHeader, PngDecodeError> {
    if payload.len() != IHDR_PAYLOAD_LENGTH {
        return Err(PngDecodeError::Truncated);
    }

    let width = read_be_u32(payload.get(0..4).ok_or(PngDecodeError::Truncated)?)?;
    let height = read_be_u32(payload.get(4..8).ok_or(PngDecodeError::Truncated)?)?;
    let bit_depth = *payload.get(8).ok_or(PngDecodeError::Truncated)?;
    let color_type = *payload.get(9).ok_or(PngDecodeError::Truncated)?;
    let compression_method = *payload.get(10).ok_or(PngDecodeError::Truncated)?;
    let filter_method = *payload.get(11).ok_or(PngDecodeError::Truncated)?;
    let interlace_method = *payload.get(12).ok_or(PngDecodeError::Truncated)?;

    if width == 0
        || height == 0
        || width > MAXIMUM_IMAGE_DIMENSION
        || height > MAXIMUM_IMAGE_DIMENSION
    {
        return Err(PngDecodeError::InvalidDimensions { width, height });
    }
    if bit_depth != BIT_DEPTH_8 || (color_type != COLOR_TYPE_RGB && color_type != COLOR_TYPE_RGBA) {
        return Err(PngDecodeError::UnsupportedColorType {
            color_type,
            bit_depth,
        });
    }
    if compression_method != 0 {
        return Err(PngDecodeError::UnsupportedCompression(compression_method));
    }
    if filter_method != 0 {
        return Err(PngDecodeError::UnsupportedFilterMethod(filter_method));
    }
    if interlace_method != 0 {
        return Err(PngDecodeError::UnsupportedInterlacing);
    }

    Ok(ImageHeader {
        width,
        height,
        color_type,
    })
}

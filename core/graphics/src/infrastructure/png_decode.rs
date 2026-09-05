//! A hostile-input-safe PNG decoder over `network::inflate` (v0.5 Phase X).
//!
//! Decodes PNG files (signature, `IHDR`, `IDAT`, `IEND`) with RGB (color type 2)
//! or RGBA (color type 6) at 8 bits per channel, unfiltering all five standard
//! filter types (None, Sub, Up, Average, Paeth).
//!
//! Under `#![forbid(unsafe_code)]`, zero panics, zero unwrap/expect, no `as` casts,
//! and integer-exact arithmetic throughout (`ADR-0016`, `ADR-0018`).

use crate::domain::framebuffer::Framebuffer;
use crate::domain::geometry::SurfaceSize;
use crate::infrastructure::png::crc32;
use network::inflate::{OutputLimit, zlib_decompress_within};

/// The eight bytes every PNG file begins with (RFC 2083 §3.1).
const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];

/// Maximum allowed dimension on either axis to prevent memory bombs.
const MAX_IMAGE_DIMENSION: u32 = 16_384;

/// Maximum uncompressed bytes allocated for a single decoded image (64 MiB).
const MAX_DECOMPRESSED_BYTES: usize = 64 * 1024 * 1024;

/// Color type 2: Truecolor with RGB (3 bytes per pixel).
const COLOR_TYPE_RGB: u8 = 2;

/// Color type 6: Truecolor with Alpha RGBA (4 bytes per pixel).
const COLOR_TYPE_RGBA: u8 = 6;

/// Bit depth 8: 8 bits per channel.
const BIT_DEPTH_8: u8 = 8;

/// Why a byte stream could not be decoded as a valid PNG image.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum PngDecodeError {
    /// The byte stream ended unexpectedly.
    #[error("the stream ended before a complete header, chunk or scanline could be read")]
    Truncated,

    /// The 8-byte PNG signature was missing or incorrect.
    #[error("the PNG 8-byte file signature was absent or corrupted")]
    InvalidSignature,

    /// A chunk's CRC-32 checksum did not match its contents.
    #[error(
        "chunk {kind} CRC-32 mismatch (expected {expected:#010x}, calculated {calculated:#010x})"
    )]
    ChunkCorrupt {
        /// Chunk type name.
        kind: String,
        /// The CRC-32 stored in the chunk.
        expected: u32,
        /// The calculated CRC-32.
        calculated: u32,
    },

    /// The first chunk was not `IHDR`.
    #[error("first chunk must be IHDR")]
    MissingIhdr,

    /// A chunk appeared after `IEND`.
    #[error("found unexpected chunk after IEND")]
    TrailingData,

    /// Color type and bit depth combination is outside the supported cut.
    #[error(
        "color type {color_type} with bit depth {bit_depth} is not supported (supported: 8-bit RGB and RGBA)"
    )]
    UnsupportedColorType {
        /// The color type byte from `IHDR`.
        color_type: u8,
        /// The bit depth byte from `IHDR`.
        bit_depth: u8,
    },

    /// Interlaced images (Adam7) are not supported in this cut.
    #[error("interlaced PNGs (Adam7) are not supported")]
    UnsupportedInterlacing,

    /// Compression method other than 0 (deflate).
    #[error("compression method {0} is not supported (expected 0)")]
    UnsupportedCompression(u8),

    /// Filter method other than 0 (adaptive).
    #[error("filter method {0} is not supported (expected 0)")]
    UnsupportedFilterMethod(u8),

    /// A scanline filter byte is invalid.
    #[error("filter type {filter} on row {row} is invalid (expected 0..=4)")]
    InvalidFilter {
        /// The invalid filter byte.
        filter: u8,
        /// The 0-based scanline row index.
        row: u32,
    },

    /// zlib / deflate decompression failed.
    #[error("zlib decompression failed: {0}")]
    DecompressionFailed(#[from] network::inflate::InflateError),

    /// Declared image dimensions are zero or exceed safety limits.
    #[error("image dimensions {width}x{height} exceed limits or are invalid")]
    InvalidDimensions {
        /// The width in pixels.
        width: u32,
        /// The height in pixels.
        height: u32,
    },

    /// Decompressed bytes do not match the expected row count and stride.
    #[error(
        "uncompressed scanline data length mismatch: expected {expected} bytes, got {actual} bytes"
    )]
    ScanlineLengthMismatch {
        /// Expected byte length.
        expected: usize,
        /// Actual byte length.
        actual: usize,
    },
}

/// Decodes an arbitrary hostile byte slice into a straight-alpha RGBA8 [`Framebuffer`].
///
/// # Errors
///
/// Returns a typed [`PngDecodeError`] naming the exact reason if decoding fails.
/// Never panics on hostile input.
pub fn decode_png(bytes: &[u8]) -> Result<Framebuffer, PngDecodeError> {
    if bytes.len() < PNG_SIGNATURE.len() {
        return Err(PngDecodeError::Truncated);
    }
    if bytes.get(..PNG_SIGNATURE.len()) != Some(&PNG_SIGNATURE) {
        return Err(PngDecodeError::InvalidSignature);
    }

    let mut cursor = bytes
        .get(PNG_SIGNATURE.len()..)
        .ok_or(PngDecodeError::Truncated)?;
    let mut header: Option<ImageHeader> = None;
    let mut idat_data = Vec::new();
    let mut seen_iend = false;

    while !cursor.is_empty() {
        let chunk = read_chunk(cursor)?;
        cursor = chunk.rest;

        if seen_iend {
            return Err(PngDecodeError::TrailingData);
        }

        match &chunk.kind {
            b"IHDR" => {
                if header.is_some() {
                    return Err(PngDecodeError::MissingIhdr);
                }
                header = Some(parse_ihdr(chunk.payload)?);
            }
            b"IDAT" => {
                if header.is_none() {
                    return Err(PngDecodeError::MissingIhdr);
                }
                idat_data.extend_from_slice(chunk.payload);
            }
            b"IEND" => {
                if header.is_none() {
                    return Err(PngDecodeError::MissingIhdr);
                }
                seen_iend = true;
                break;
            }
            _ => {
                if header.is_none() {
                    return Err(PngDecodeError::MissingIhdr);
                }
                // Ancillary chunks safely ignored.
            }
        }
    }

    if !seen_iend {
        return Err(PngDecodeError::Truncated);
    }
    let header = header.ok_or(PngDecodeError::MissingIhdr)?;
    if idat_data.is_empty() {
        return Err(PngDecodeError::Truncated);
    }

    let uncompressed = decompress_idat(&header, &idat_data)?;
    unfilter_scanlines(&header, &uncompressed)
}

struct ImageHeader {
    width: u32,
    height: u32,
    color_type: u8,
}

fn parse_ihdr(payload: &[u8]) -> Result<ImageHeader, PngDecodeError> {
    if payload.len() < 13 {
        return Err(PngDecodeError::Truncated);
    }
    let width = read_be_u32(payload.get(0..4).ok_or(PngDecodeError::Truncated)?)?;
    let height = read_be_u32(payload.get(4..8).ok_or(PngDecodeError::Truncated)?)?;
    let bit_depth = *payload.get(8).ok_or(PngDecodeError::Truncated)?;
    let color_type = *payload.get(9).ok_or(PngDecodeError::Truncated)?;
    let compression = *payload.get(10).ok_or(PngDecodeError::Truncated)?;
    let filter = *payload.get(11).ok_or(PngDecodeError::Truncated)?;
    let interlace = *payload.get(12).ok_or(PngDecodeError::Truncated)?;

    if width == 0 || height == 0 || width > MAX_IMAGE_DIMENSION || height > MAX_IMAGE_DIMENSION {
        return Err(PngDecodeError::InvalidDimensions { width, height });
    }
    if bit_depth != BIT_DEPTH_8 || (color_type != COLOR_TYPE_RGB && color_type != COLOR_TYPE_RGBA) {
        return Err(PngDecodeError::UnsupportedColorType {
            color_type,
            bit_depth,
        });
    }
    if compression != 0 {
        return Err(PngDecodeError::UnsupportedCompression(compression));
    }
    if filter != 0 {
        return Err(PngDecodeError::UnsupportedFilterMethod(filter));
    }
    if interlace != 0 {
        return Err(PngDecodeError::UnsupportedInterlacing);
    }

    Ok(ImageHeader {
        width,
        height,
        color_type,
    })
}

struct RawChunk<'a> {
    kind: [u8; 4],
    payload: &'a [u8],
    rest: &'a [u8],
}

fn read_chunk(bytes: &[u8]) -> Result<RawChunk<'_>, PngDecodeError> {
    if bytes.len() < 12 {
        return Err(PngDecodeError::Truncated);
    }
    let length = read_be_u32(bytes.get(0..4).ok_or(PngDecodeError::Truncated)?)?;
    let length_usize = usize::try_from(length).map_err(|_| PngDecodeError::Truncated)?;

    let type_bytes = bytes.get(4..8).ok_or(PngDecodeError::Truncated)?;
    let kind = <[u8; 4]>::try_from(type_bytes).map_err(|_| PngDecodeError::Truncated)?;

    let payload_end = 8_usize
        .checked_add(length_usize)
        .ok_or(PngDecodeError::Truncated)?;
    let payload = bytes.get(8..payload_end).ok_or(PngDecodeError::Truncated)?;

    let crc_end = payload_end
        .checked_add(4)
        .ok_or(PngDecodeError::Truncated)?;
    let crc_bytes = bytes
        .get(payload_end..crc_end)
        .ok_or(PngDecodeError::Truncated)?;
    let expected_crc = read_be_u32(crc_bytes)?;

    let mut checked = Vec::with_capacity(length_usize.saturating_add(4));
    checked.extend_from_slice(&kind);
    checked.extend_from_slice(payload);
    let calculated_crc = crc32(&checked);

    if calculated_crc != expected_crc {
        let kind_name = String::from_utf8_lossy(&kind).into_owned();
        return Err(PngDecodeError::ChunkCorrupt {
            kind: kind_name,
            expected: expected_crc,
            calculated: calculated_crc,
        });
    }

    let rest = bytes.get(crc_end..).ok_or(PngDecodeError::Truncated)?;
    Ok(RawChunk {
        kind,
        payload,
        rest,
    })
}

fn read_be_u32(bytes: &[u8]) -> Result<u32, PngDecodeError> {
    let arr = <[u8; 4]>::try_from(bytes).map_err(|_| PngDecodeError::Truncated)?;
    Ok(u32::from_be_bytes(arr))
}

fn decompress_idat(header: &ImageHeader, idat: &[u8]) -> Result<Vec<u8>, PngDecodeError> {
    let width_usize =
        usize::try_from(header.width).map_err(|_| PngDecodeError::InvalidDimensions {
            width: header.width,
            height: header.height,
        })?;
    let height_usize =
        usize::try_from(header.height).map_err(|_| PngDecodeError::InvalidDimensions {
            width: header.width,
            height: header.height,
        })?;
    let bpp = if header.color_type == COLOR_TYPE_RGB {
        3_usize
    } else {
        4_usize
    };

    let row_bytes = width_usize
        .checked_mul(bpp)
        .ok_or(PngDecodeError::InvalidDimensions {
            width: header.width,
            height: header.height,
        })?;
    let scanline_bytes = row_bytes
        .checked_add(1)
        .ok_or(PngDecodeError::InvalidDimensions {
            width: header.width,
            height: header.height,
        })?;
    let expected_uncompressed =
        scanline_bytes
            .checked_mul(height_usize)
            .ok_or(PngDecodeError::InvalidDimensions {
                width: header.width,
                height: header.height,
            })?;

    if expected_uncompressed > MAX_DECOMPRESSED_BYTES {
        return Err(PngDecodeError::InvalidDimensions {
            width: header.width,
            height: header.height,
        });
    }

    let uncompressed = zlib_decompress_within(idat, OutputLimit::of_bytes(expected_uncompressed))?;
    if uncompressed.len() != expected_uncompressed {
        return Err(PngDecodeError::ScanlineLengthMismatch {
            expected: expected_uncompressed,
            actual: uncompressed.len(),
        });
    }
    Ok(uncompressed)
}

fn unfilter_scanlines(
    header: &ImageHeader,
    uncompressed: &[u8],
) -> Result<Framebuffer, PngDecodeError> {
    let width_usize =
        usize::try_from(header.width).map_err(|_| PngDecodeError::InvalidDimensions {
            width: header.width,
            height: header.height,
        })?;
    let height_usize =
        usize::try_from(header.height).map_err(|_| PngDecodeError::InvalidDimensions {
            width: header.width,
            height: header.height,
        })?;
    let bpp = if header.color_type == COLOR_TYPE_RGB {
        3_usize
    } else {
        4_usize
    };

    let row_bytes = width_usize
        .checked_mul(bpp)
        .ok_or(PngDecodeError::InvalidDimensions {
            width: header.width,
            height: header.height,
        })?;
    let scanline_bytes = row_bytes
        .checked_add(1)
        .ok_or(PngDecodeError::InvalidDimensions {
            width: header.width,
            height: header.height,
        })?;

    let pixel_count =
        width_usize
            .checked_mul(height_usize)
            .ok_or(PngDecodeError::InvalidDimensions {
                width: header.width,
                height: header.height,
            })?;
    let total_pixels_bytes =
        pixel_count
            .checked_mul(4)
            .ok_or(PngDecodeError::InvalidDimensions {
                width: header.width,
                height: header.height,
            })?;

    let mut pixels = Vec::with_capacity(total_pixels_bytes);
    let mut prior = vec![0_u8; row_bytes];
    let mut cursor: &[u8] = uncompressed;

    for row in 0..header.height {
        let filter = *cursor.first().ok_or(PngDecodeError::Truncated)?;
        let scanline = cursor
            .get(1..scanline_bytes)
            .ok_or(PngDecodeError::Truncated)?;
        cursor = cursor
            .get(scanline_bytes..)
            .ok_or(PngDecodeError::Truncated)?;

        let mut recon = vec![0_u8; row_bytes];
        for i in 0..row_bytes {
            let filt = *scanline.get(i).ok_or(PngDecodeError::Truncated)?;
            let a = if i >= bpp {
                *recon.get(i.saturating_sub(bpp)).unwrap_or(&0)
            } else {
                0
            };
            let b = *prior.get(i).unwrap_or(&0);
            let c = if i >= bpp {
                *prior.get(i.saturating_sub(bpp)).unwrap_or(&0)
            } else {
                0
            };

            let val = match filter {
                0 => filt,
                1 => filt.wrapping_add(a),
                2 => filt.wrapping_add(b),
                3 => filt.wrapping_add(average(a, b)),
                4 => filt.wrapping_add(paeth_predictor(a, b, c)),
                _ => return Err(PngDecodeError::InvalidFilter { filter, row }),
            };

            if let Some(slot) = recon.get_mut(i) {
                *slot = val;
            }
        }

        if bpp == 3 {
            for rgb in recon.chunks_exact(3) {
                let r = *rgb.first().unwrap_or(&0);
                let g = *rgb.get(1).unwrap_or(&0);
                let b = *rgb.get(2).unwrap_or(&0);
                pixels.extend_from_slice(&[r, g, b, 255]);
            }
        } else {
            pixels.extend_from_slice(&recon);
        }

        prior = recon;
    }

    let size =
        SurfaceSize::new(header.width, header.height).ok_or(PngDecodeError::InvalidDimensions {
            width: header.width,
            height: header.height,
        })?;
    Framebuffer::from_rgba8(size, pixels).ok_or(PngDecodeError::InvalidDimensions {
        width: header.width,
        height: header.height,
    })
}

fn average(a: u8, b: u8) -> u8 {
    let sum = u16::from(a).saturating_add(u16::from(b));
    let half = sum.checked_div(2).unwrap_or(0);
    u8::try_from(half).unwrap_or(0)
}

fn paeth_predictor(a: u8, b: u8, c: u8) -> u8 {
    let a_i = i32::from(a);
    let b_i = i32::from(b);
    let c_i = i32::from(c);
    let p = a_i.saturating_add(b_i).saturating_sub(c_i);
    let pa = p.saturating_sub(a_i).abs();
    let pb = p.saturating_sub(b_i).abs();
    let pc = p.saturating_sub(c_i).abs();
    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}

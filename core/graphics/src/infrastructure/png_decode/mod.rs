//! A hostile-input-safe PNG decoder over `network::inflate` (v0.5 Phase X).
//!
//! Decodes PNG files (signature, `IHDR`, `IDAT`, `IEND`) with RGB (color type 2)
//! or RGBA (color type 6) at 8 bits per channel, unfiltering all five standard
//! filter types (None, Sub, Up, Average, Paeth).
//!
//! Under `#![forbid(unsafe_code)]`, zero panics, zero unwrap/expect, no `as` casts,
//! and integer-exact arithmetic throughout (`ADR-0016`, `ADR-0018`).

mod chunk;
pub mod error;
mod filter;
mod header;

use crate::domain::framebuffer::Framebuffer;
use network::inflate::{OutputLimit, zlib_decompress_within};

pub use error::PngDecodeError;
use header::{COLOR_TYPE_RGB, ImageHeader, parse_ihdr};

/// The eight bytes every PNG file begins with (RFC 2083 §3.1).
const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];

/// Maximum uncompressed bytes allocated for a single decoded image (64 MiB).
const MAXIMUM_DECOMPRESSED_BYTES: usize = 64 * 1024 * 1024;

/// Decodes an arbitrary hostile byte slice into a straight-alpha RGBA8 [`Framebuffer`].
pub fn decode_png(bytes: &[u8]) -> Result<Framebuffer, PngDecodeError> {
    if bytes.len() < PNG_SIGNATURE.len() || bytes.get(..PNG_SIGNATURE.len()) != Some(&PNG_SIGNATURE)
    {
        return Err(validate_signature(bytes));
    }

    let mut cursor = bytes
        .get(PNG_SIGNATURE.len()..)
        .ok_or(PngDecodeError::Truncated)?;
    let mut header: Option<ImageHeader> = None;
    let mut idat_payloads = Vec::new();
    let mut seen_iend = false;

    while !cursor.is_empty() {
        let chunk = chunk::read_chunk(cursor)?;
        cursor = chunk.rest;

        match &chunk.kind {
            b"IHDR" => {
                if header.is_some() {
                    return Err(PngDecodeError::DuplicateIhdr);
                }
                header = Some(parse_ihdr(chunk.payload)?);
            }
            b"IDAT" => {
                if header.is_none() {
                    return Err(PngDecodeError::MissingIhdr);
                }
                idat_payloads.extend_from_slice(chunk.payload);
            }
            b"IEND" => {
                if header.is_none() {
                    return Err(PngDecodeError::MissingIhdr);
                }
                if !chunk.rest.is_empty() {
                    return Err(PngDecodeError::TrailingData);
                }
                seen_iend = true;
                break;
            }
            _ => {
                if header.is_none() {
                    return Err(PngDecodeError::MissingIhdr);
                }
            }
        }
    }

    if !seen_iend || idat_payloads.is_empty() {
        return Err(PngDecodeError::Truncated);
    }
    let validated_header = header.ok_or(PngDecodeError::MissingIhdr)?;
    let uncompressed = decompress_idat(&validated_header, &idat_payloads)?;
    filter::unfilter_scanlines(&validated_header, &uncompressed)
}

const fn validate_signature(bytes: &[u8]) -> PngDecodeError {
    if bytes.len() < PNG_SIGNATURE.len() {
        return PngDecodeError::Truncated;
    }
    PngDecodeError::InvalidSignature
}

fn decompress_idat(header: &ImageHeader, idat: &[u8]) -> Result<Vec<u8>, PngDecodeError> {
    let width_usize = usize::try_from(header.width).map_err(|_| invalid_dim(header))?;
    let height_usize = usize::try_from(header.height).map_err(|_| invalid_dim(header))?;
    let stride = match header.color_type {
        COLOR_TYPE_RGB => 3_usize,
        _ => 4_usize,
    };

    let row_bytes = width_usize
        .checked_mul(stride)
        .ok_or_else(|| invalid_dim(header))?;
    let scanline_bytes = row_bytes
        .checked_add(1)
        .ok_or_else(|| invalid_dim(header))?;
    let expected_bytes = scanline_bytes
        .checked_mul(height_usize)
        .ok_or_else(|| invalid_dim(header))?;

    if expected_bytes > MAXIMUM_DECOMPRESSED_BYTES {
        return Err(invalid_dim(header));
    }

    let uncompressed = zlib_decompress_within(idat, OutputLimit::of_bytes(expected_bytes))?;
    if uncompressed.len() != expected_bytes {
        return Err(PngDecodeError::ScanlineLengthMismatch {
            expected: expected_bytes,
            actual: uncompressed.len(),
        });
    }
    Ok(uncompressed)
}

const fn invalid_dim(header: &ImageHeader) -> PngDecodeError {
    PngDecodeError::InvalidDimensions {
        width: header.width,
        height: header.height,
    }
}

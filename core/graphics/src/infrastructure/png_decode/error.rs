//! Typed PNG decoding errors (`ADR-0011`, `PRD-005`).

use network::inflate::InflateError;

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

    /// A duplicate `IHDR` chunk was encountered.
    #[error("duplicate IHDR chunk found")]
    DuplicateIhdr,

    /// Unexpected bytes or chunks appeared after `IEND`.
    #[error("found unexpected trailing data after IEND")]
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
    DecompressionFailed(#[from] InflateError),

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

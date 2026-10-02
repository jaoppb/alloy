//! Chunk parsing with zero-copy CRC-32 verification (RFC 2083 §3.2).

use crate::infrastructure::png::crc32;
use crate::infrastructure::png_decode::error::PngDecodeError;

/// Maximum allowable chunk data length (2^31 - 1 bytes, RFC 2083 §3.2).
const MAXIMUM_CHUNK_LENGTH: u32 = 0x7fff_ffff;

/// A raw uncompressed chunk read from the byte stream.
pub(super) struct RawChunk<'a> {
    pub(super) kind: [u8; 4],
    pub(super) payload: &'a [u8],
    pub(super) rest: &'a [u8],
}

/// Reads the next chunk from `bytes`, verifying its length and CRC-32 checksum.
pub(super) fn read_chunk(bytes: &[u8]) -> Result<RawChunk<'_>, PngDecodeError> {
    if bytes.len() < 12 {
        return Err(PngDecodeError::Truncated);
    }

    let length = read_be_u32(bytes.get(0..4).ok_or(PngDecodeError::Truncated)?)?;
    if length > MAXIMUM_CHUNK_LENGTH {
        return Err(PngDecodeError::Truncated);
    }
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

    let data_to_check = bytes.get(4..payload_end).ok_or(PngDecodeError::Truncated)?;
    let calculated_crc = crc32(data_to_check);

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

/// Reads a big-endian 32-bit unsigned integer from a 4-byte slice.
pub(super) fn read_be_u32(bytes: &[u8]) -> Result<u32, PngDecodeError> {
    let array = <[u8; 4]>::try_from(bytes).map_err(|_| PngDecodeError::Truncated)?;
    Ok(u32::from_be_bytes(array))
}

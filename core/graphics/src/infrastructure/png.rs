//! A PNG encoder and a matching decoder, with **zero dependencies**.
//!
//! ## Why hand-written
//!
//! The alternative pulls four crates into a graph `cargo-deny` audits, in order
//! to *write* the one output artefact the whole of v0.3 produces. The file comes
//! out larger, because the deflate blocks are **stored** (`BTYPE=00`, no
//! compression) — and that costs nothing that matters, because the golden gate
//! compares the decoded [`Framebuffer`], not the PNG bytes (v0.3 report §2.5,
//! `ADR-0016`).
//!
//! *Decoding* an arbitrary hostile PNG is a different problem and a separate
//! decision: [`decode`] deliberately reads only the narrow subset [`encode`]
//! writes — 8-bit RGBA, no interlacing, filter 0, stored deflate — and refuses
//! everything else. Real image decoding arrives with `DrawImage` in v0.5 and
//! will almost certainly adopt an audited crate.
//!
//! ## Why the error type is local
//!
//! [`PngProblem`] is not [`crate::GraphicsError`]. `ADR-0011` item 4 asks for one
//! typed error *per port*, and the port is `RenderBackend`; a container format
//! used by the CLI and the golden gate is an infrastructure detail, and folding
//! it into the frozen port surface would be the wrong kind of tidiness.

use core::fmt;

use crate::domain::color::Color;
use crate::domain::framebuffer::{BYTES_PER_PIXEL, Framebuffer};
use crate::domain::geometry::SurfaceSize;

/// The eight bytes every PNG starts with.
const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
/// The largest payload a single stored deflate block can carry.
const MAX_STORED_BLOCK: usize = 0xffff;
/// zlib header: deflate, 32 KiB window, no preset dictionary. `0x7801 % 31 == 0`.
const ZLIB_HEADER: [u8; 2] = [0x78, 0x01];
/// Colour type 6: truecolour with alpha.
const COLOUR_TYPE_RGBA: u8 = 6;
/// The only bit depth this module reads or writes.
const BIT_DEPTH: u8 = 8;

/// Why a byte stream could not be read as one of our PNGs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PngProblem {
    /// The eight-byte signature was absent or wrong.
    NotAPng,
    /// The stream ended inside a header, a chunk, or a deflate block.
    Truncated,
    /// A chunk's CRC-32 did not match its contents.
    ChunkCorrupt,
    /// The zlib stream's Adler-32 did not match the decoded bytes.
    StreamCorrupt,
    /// `IHDR` declared something this decoder does not read: a bit depth other
    /// than 8, a colour type other than RGBA, interlacing, or a zero dimension.
    UnsupportedFormat,
    /// A scanline used a filter other than 0. Our encoder only writes 0.
    UnsupportedFilter,
    /// A deflate block was compressed. Our encoder only writes stored blocks.
    UnsupportedCompression,
}

impl PngProblem {
    #[must_use]
    pub const fn reason(self) -> &'static str {
        match self {
            Self::NotAPng => "the stream does not start with the PNG signature",
            Self::Truncated => "the stream ends mid-structure",
            Self::ChunkCorrupt => "a chunk failed its CRC-32 check",
            Self::StreamCorrupt => "the zlib stream failed its Adler-32 check",
            Self::UnsupportedFormat => "only 8-bit non-interlaced RGBA is read",
            Self::UnsupportedFilter => "only scanline filter 0 is read",
            Self::UnsupportedCompression => "only stored (uncompressed) deflate blocks are read",
        }
    }
}

impl fmt::Display for PngProblem {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.reason())
    }
}

impl std::error::Error for PngProblem {}

/// Encodes a framebuffer as an 8-bit RGBA PNG.
///
/// Deterministic: the same framebuffer always produces the same bytes, which is
/// what lets a `.png` be committed as a reference at all.
#[must_use]
pub fn encode(frame: &Framebuffer) -> Vec<u8> {
    let mut bytes = Vec::from(SIGNATURE);
    append_chunk(&mut bytes, *b"IHDR", &header(frame.size()));
    append_chunk(&mut bytes, *b"IDAT", &zlib_stream(&scanlines(frame)));
    append_chunk(&mut bytes, *b"IEND", &[]);
    bytes
}

/// The 13-byte `IHDR` payload.
fn header(size: SurfaceSize) -> Vec<u8> {
    let mut payload = Vec::with_capacity(13);
    payload.extend_from_slice(&size.width().to_be_bytes());
    payload.extend_from_slice(&size.height().to_be_bytes());
    payload.push(BIT_DEPTH);
    payload.push(COLOUR_TYPE_RGBA);
    payload.push(0); // compression: deflate
    payload.push(0); // filter method: adaptive
    payload.push(0); // interlace: none
    payload
}

/// The raw image bytes: every row prefixed with its filter byte.
fn scanlines(frame: &Framebuffer) -> Vec<u8> {
    let stride = row_bytes(frame.width());
    let mut raw = Vec::new();
    for row in 0..frame.height() {
        raw.push(0); // filter 0: no prediction, so decoding needs no history
        let start = usize::try_from(row).unwrap_or(0).saturating_mul(stride);
        let end = start.saturating_add(stride);
        match frame.as_rgba8().get(start..end) {
            Some(line) => raw.extend_from_slice(line),
            None => raw.extend(core::iter::repeat_n(0, stride)),
        }
    }
    raw
}

/// How many bytes one row of `width` pixels occupies.
fn row_bytes(width: u32) -> usize {
    usize::try_from(width)
        .unwrap_or(0)
        .saturating_mul(BYTES_PER_PIXEL)
}

/// Wraps `raw` in a zlib stream of stored deflate blocks.
fn zlib_stream(raw: &[u8]) -> Vec<u8> {
    let mut stream = Vec::from(ZLIB_HEADER);
    append_stored_blocks(&mut stream, raw);
    stream.extend_from_slice(&adler32(raw).to_be_bytes());
    stream
}

/// Emits `raw` as stored deflate blocks, the last one flagged final.
fn append_stored_blocks(stream: &mut Vec<u8>, raw: &[u8]) {
    let mut remaining = raw;
    loop {
        let take = remaining.len().min(MAX_STORED_BLOCK);
        let (block, rest) = remaining.split_at(take);
        let final_block = rest.is_empty();
        stream.push(u8::from(final_block)); // BFINAL, BTYPE = 00 (stored)
        let length = u16::try_from(block.len()).unwrap_or(u16::MAX);
        stream.extend_from_slice(&length.to_le_bytes());
        stream.extend_from_slice(&(!length).to_le_bytes());
        stream.extend_from_slice(block);
        if final_block {
            return;
        }
        remaining = rest;
    }
}

/// Appends one chunk: length, type, payload, CRC over type and payload.
fn append_chunk(bytes: &mut Vec<u8>, kind: [u8; 4], payload: &[u8]) {
    let length = u32::try_from(payload.len()).unwrap_or(u32::MAX);
    bytes.extend_from_slice(&length.to_be_bytes());
    bytes.extend_from_slice(&kind);
    bytes.extend_from_slice(payload);
    let mut checked = Vec::with_capacity(payload.len().saturating_add(4));
    checked.extend_from_slice(&kind);
    checked.extend_from_slice(payload);
    bytes.extend_from_slice(&crc32(&checked).to_be_bytes());
}

/// The CRC-32 polynomial of the PNG specification, bit-reversed.
const CRC_POLYNOMIAL: u32 = 0xedb8_8320;

/// CRC-32 as PNG defines it, computed bit by bit.
///
/// Deliberately table-free. A 256-entry table would need indexed assignment
/// inside a `const` block, which is the one thing in this module that would have
/// required an `ADR-0017` carve-out — and eight shifts per byte is more than
/// enough for an artefact measured in kilobytes. Paying microseconds to keep the
/// lint gate intact is the right trade here.
fn crc32(bytes: &[u8]) -> u32 {
    let mut register = u32::MAX;
    for byte in bytes {
        register ^= u32::from(*byte);
        for _ in 0..8 {
            register = step_crc(register);
        }
    }
    register ^ u32::MAX
}

/// One bit of the CRC division.
const fn step_crc(register: u32) -> u32 {
    let shifted = register.wrapping_shr(1);
    match register & 1 {
        1 => shifted ^ CRC_POLYNOMIAL,
        _ => shifted,
    }
}

/// Adler-32 as zlib defines it.
fn adler32(bytes: &[u8]) -> u32 {
    let mut low = 1_u32;
    let mut high = 0_u32;
    for byte in bytes {
        low = (low.saturating_add(u32::from(*byte))) % 65_521;
        high = (high.saturating_add(low)) % 65_521;
    }
    high.saturating_mul(65_536).saturating_add(low)
}

// ---- decoding, for the golden gate only --------------------------------------

/// Reads back a PNG that [`encode`] produced.
///
/// The golden gate compares framebuffers rather than files (v0.3 report §2.5),
/// and this is how the committed reference becomes a framebuffer. It reads only
/// what [`encode`] writes and refuses anything else — see the module docs.
pub fn decode(bytes: &[u8]) -> Result<Framebuffer, PngProblem> {
    let body = bytes.get(SIGNATURE.len()..).ok_or(PngProblem::Truncated)?;
    if bytes.get(..SIGNATURE.len()) != Some(&SIGNATURE) {
        return Err(PngProblem::NotAPng);
    }
    let chunks = split_chunks(body)?;
    let size = read_header(&chunks)?;
    let raw = inflate_stored(&concatenated_data(&chunks))?;
    build_frame(size, &raw)
}

/// One chunk: its four-byte type and its payload.
struct Chunk {
    kind: [u8; 4],
    payload: Vec<u8>,
}

/// Splits the stream after the signature into verified chunks.
fn split_chunks(mut body: &[u8]) -> Result<Vec<Chunk>, PngProblem> {
    let mut chunks = Vec::new();
    while !body.is_empty() {
        let length = read_u32(body.get(..4).ok_or(PngProblem::Truncated)?)?;
        let length = usize::try_from(length).map_err(|_| PngProblem::Truncated)?;
        let kind_end = 8_usize;
        let payload_end = kind_end.checked_add(length).ok_or(PngProblem::Truncated)?;
        let crc_end = payload_end.checked_add(4).ok_or(PngProblem::Truncated)?;
        let kind = <[u8; 4]>::try_from(body.get(4..kind_end).ok_or(PngProblem::Truncated)?)
            .map_err(|_| PngProblem::Truncated)?;
        let payload = body
            .get(kind_end..payload_end)
            .ok_or(PngProblem::Truncated)?;
        let declared = read_u32(
            body.get(payload_end..crc_end)
                .ok_or(PngProblem::Truncated)?,
        )?;
        let mut checked = Vec::with_capacity(length.saturating_add(4));
        checked.extend_from_slice(&kind);
        checked.extend_from_slice(payload);
        if crc32(&checked) != declared {
            return Err(PngProblem::ChunkCorrupt);
        }
        chunks.push(Chunk {
            kind,
            payload: payload.to_vec(),
        });
        body = body.get(crc_end..).ok_or(PngProblem::Truncated)?;
    }
    Ok(chunks)
}

/// Reads the surface size out of `IHDR`, refusing any format we do not write.
fn read_header(chunks: &[Chunk]) -> Result<SurfaceSize, PngProblem> {
    let header = chunks
        .iter()
        .find(|chunk| &chunk.kind == b"IHDR")
        .ok_or(PngProblem::UnsupportedFormat)?;
    let width = read_u32(header.payload.get(..4).ok_or(PngProblem::Truncated)?)?;
    let height = read_u32(header.payload.get(4..8).ok_or(PngProblem::Truncated)?)?;
    let tail = header.payload.get(8..13).ok_or(PngProblem::Truncated)?;
    if tail != [BIT_DEPTH, COLOUR_TYPE_RGBA, 0, 0, 0] {
        return Err(PngProblem::UnsupportedFormat);
    }
    SurfaceSize::new(width, height).ok_or(PngProblem::UnsupportedFormat)
}

/// Every `IDAT` payload joined in order, as the specification requires.
fn concatenated_data(chunks: &[Chunk]) -> Vec<u8> {
    chunks
        .iter()
        .filter(|chunk| &chunk.kind == b"IDAT")
        .flat_map(|chunk| chunk.payload.iter().copied())
        .collect()
}

/// Unwraps a zlib stream of stored deflate blocks and checks its Adler-32.
fn inflate_stored(stream: &[u8]) -> Result<Vec<u8>, PngProblem> {
    let mut cursor = stream
        .get(ZLIB_HEADER.len()..)
        .ok_or(PngProblem::Truncated)?;
    let mut raw = Vec::new();
    loop {
        let flags = *cursor.first().ok_or(PngProblem::Truncated)?;
        if flags & 0b110 != 0 {
            return Err(PngProblem::UnsupportedCompression);
        }
        let length = usize::from(read_u16(cursor.get(1..3).ok_or(PngProblem::Truncated)?)?);
        let end = 5_usize.checked_add(length).ok_or(PngProblem::Truncated)?;
        raw.extend_from_slice(cursor.get(5..end).ok_or(PngProblem::Truncated)?);
        cursor = cursor.get(end..).ok_or(PngProblem::Truncated)?;
        if flags & 1 == 1 {
            break;
        }
    }
    let declared = read_u32(cursor.get(..4).ok_or(PngProblem::Truncated)?)?;
    if adler32(&raw) != declared {
        return Err(PngProblem::StreamCorrupt);
    }
    Ok(raw)
}

/// Turns filtered scanlines back into pixels.
fn build_frame(size: SurfaceSize, raw: &[u8]) -> Result<Framebuffer, PngProblem> {
    let stride = row_bytes(size.width());
    let mut frame = Framebuffer::filled(size, Color::TRANSPARENT).ok_or(PngProblem::Truncated)?;
    let mut cursor = raw;
    for row in 0..size.height() {
        let filter = *cursor.first().ok_or(PngProblem::Truncated)?;
        if filter != 0 {
            return Err(PngProblem::UnsupportedFilter);
        }
        let end = stride.checked_add(1).ok_or(PngProblem::Truncated)?;
        let line = cursor.get(1..end).ok_or(PngProblem::Truncated)?;
        write_row(&mut frame, row, line);
        cursor = cursor.get(end..).ok_or(PngProblem::Truncated)?;
    }
    Ok(frame)
}

/// Writes one decoded scanline into the frame.
fn write_row(frame: &mut Framebuffer, row: u32, line: &[u8]) {
    for (column, pixel) in line.chunks_exact(BYTES_PER_PIXEL).enumerate() {
        let Ok(column) = u32::try_from(column) else {
            return;
        };
        let Ok([red, green, blue, alpha]) = <[u8; BYTES_PER_PIXEL]>::try_from(pixel) else {
            return;
        };
        frame.set_pixel(column, row, Color::rgba(red, green, blue, alpha));
    }
}

fn read_u32(bytes: &[u8]) -> Result<u32, PngProblem> {
    <[u8; 4]>::try_from(bytes)
        .map(u32::from_be_bytes)
        .map_err(|_| PngProblem::Truncated)
}

fn read_u16(bytes: &[u8]) -> Result<u16, PngProblem> {
    <[u8; 2]>::try_from(bytes)
        .map(u16::from_le_bytes)
        .map_err(|_| PngProblem::Truncated)
}

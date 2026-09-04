//! A hand-written RFC 1951 DEFLATE decoder, plus the RFC 1950 (zlib) and
//! RFC 1952 (gzip) containers that wrap it.
//!
//! ## Why it is hand-written
//!
//! This code reads **attacker-controlled bytes** — `ADR-0018` row 1, where
//! third-party `unsafe` is forbidden. Every ecosystem decompressor either
//! carries `unsafe` for SIMD or pulls a C library. So it is written here,
//! under `#![forbid(unsafe_code)]`, with the workspace lint set intact: no
//! `unwrap`, no `expect`, no `panic`, no raw indexing, no `as` cast, and no
//! unchecked arithmetic. A malformed stream is a typed [`InflateError`], never
//! a crash.
//!
//! ## Why it lives in `core/network` and is `pub`
//!
//! `Content-Encoding: gzip` and `deflate` need it here. Phase X's PNG decoder
//! needs exactly the same RFC 1951 core for `IDAT` (which is zlib-wrapped), and
//! the v0.5 plan is explicit that it **re-exports `network::inflate` rather
//! than creating a fourth crate**. That is why this module is not gated behind
//! `real-transport`: it opens no socket and links no crypto, so it is available
//! in the `no-transport` build too, and it is the fuzz target
//! `fuzz/fuzz_targets/inflate.rs` of Phase X.
//!
//! ## The ceiling
//!
//! Every entry point takes — or defaults to — an [`OutputLimit`]. A
//! decompression bomb stops at the ceiling with
//! [`InflateError::OutputLimitExceeded`] instead of exhausting memory.

use crate::domain::defect::DecodeDefect;
use crate::domain::error::NetworkError;

/// The longest Huffman code RFC 1951 permits.
const MAX_CODE_LENGTH: usize = 15;
/// Symbols in the code-length alphabet.
const CODE_LENGTH_SYMBOLS: usize = 19;
/// Symbols in the literal/length alphabet.
const LITERAL_SYMBOLS: usize = 288;
/// Symbols in the distance alphabet.
const DISTANCE_SYMBOLS: usize = 32;
/// The end-of-block symbol.
const END_OF_BLOCK: u16 = 256;
/// The first length symbol.
const FIRST_LENGTH_SYMBOL: u16 = 257;

/// The order RFC 1951 §3.2.7 writes the code-length code lengths in.
const CODE_LENGTH_ORDER: [usize; CODE_LENGTH_SYMBOLS] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

/// RFC 1951 §3.2.5 length bases, indexed by `symbol - 257`.
const LENGTH_BASE: [u16; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
/// RFC 1951 §3.2.5 extra length bits, indexed by `symbol - 257`.
const LENGTH_EXTRA: [u8; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
/// RFC 1951 §3.2.5 distance bases.
const DISTANCE_BASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];
/// RFC 1951 §3.2.5 extra distance bits.
const DISTANCE_EXTRA: [u8; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];

/// How large a decompressed stream is allowed to become.
///
/// A newtype so a ceiling can never be confused with a length at a call site
/// (Object Calisthenics rule 3, `ADR-0010:129`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct OutputLimit(usize);

impl OutputLimit {
    /// 64 MiB — comfortably above any real page, far below "out of memory".
    pub const DEFAULT: Self = Self(64 * 1024 * 1024);

    /// A ceiling of `bytes` bytes.
    #[must_use]
    pub const fn of_bytes(bytes: usize) -> Self {
        Self(bytes)
    }

    /// The ceiling in bytes.
    #[must_use]
    pub const fn bytes(self) -> usize {
        self.0
    }
}

impl Default for OutputLimit {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Why a compressed stream could not be decoded.
#[derive(thiserror::Error, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum InflateError {
    /// The stream ended in the middle of a code, a block or a container.
    #[error("the compressed stream ended early")]
    TruncatedInput,
    /// A block announced type `3`, which RFC 1951 reserves.
    #[error("the stream uses the reserved block type 3")]
    ReservedBlockType,
    /// A stored block's `LEN` and `NLEN` are not complements.
    #[error("a stored block's length and its complement disagree")]
    StoredLengthMismatch,
    /// A bit sequence matched no code in the active Huffman table.
    #[error("a bit sequence matches no Huffman code")]
    InvalidHuffmanCode,
    /// The code lengths do not describe a usable Huffman tree.
    #[error("the code lengths do not describe a valid Huffman tree")]
    InvalidCodeLengths,
    /// A back-reference pointed before the start of the output.
    #[error("a back-reference points before the start of the output")]
    DistanceTooFar,
    /// The output grew past its ceiling — a decompression bomb.
    #[error("the decompressed stream is larger than the {limit}-byte ceiling")]
    OutputLimitExceeded {
        /// The ceiling that was hit.
        limit: usize,
    },
    /// A zlib container header is not one this decoder accepts.
    #[error("the zlib header is malformed or uses an unsupported method")]
    MalformedZlibHeader,
    /// A gzip container header is not one this decoder accepts.
    #[error("the gzip header is malformed or uses an unsupported method")]
    MalformedGzipHeader,
    /// The container's trailing checksum disagreed with the output.
    #[error("the container checksum does not match the decompressed data")]
    ChecksumMismatch,
}

impl From<InflateError> for NetworkError {
    fn from(error: InflateError) -> Self {
        match error {
            InflateError::OutputLimitExceeded { .. } => {
                Self::decode(DecodeDefect::CompressionRatioTooHigh)
            }
            InflateError::ChecksumMismatch => Self::decode(DecodeDefect::ChecksumMismatch),
            _ => Self::decode(DecodeDefect::MalformedCompressedStream),
        }
    }
}

/// Decode a raw DEFLATE stream under the default ceiling.
///
/// # Errors
///
/// [`InflateError`] for any malformed, truncated or over-large stream.
pub fn inflate(input: &[u8]) -> Result<Vec<u8>, InflateError> {
    inflate_within(input, OutputLimit::DEFAULT)
}

/// Decode a raw DEFLATE stream under an explicit ceiling.
///
/// # Errors
///
/// [`InflateError`] for any malformed, truncated or over-large stream.
pub fn inflate_within(input: &[u8], limit: OutputLimit) -> Result<Vec<u8>, InflateError> {
    let mut inflater = Inflater::new(input, limit);
    inflater.run()?;
    Ok(inflater.into_output())
}

/// Decode a zlib (RFC 1950) stream — the framing PNG `IDAT` and
/// `Content-Encoding: deflate` both use in practice — under the default
/// ceiling.
///
/// # Errors
///
/// [`InflateError`] for a bad header, a malformed body or an Adler-32
/// mismatch.
pub fn zlib_decompress(input: &[u8]) -> Result<Vec<u8>, InflateError> {
    zlib_decompress_within(input, OutputLimit::DEFAULT)
}

/// Decode a zlib stream under an explicit ceiling.
///
/// # Errors
///
/// As [`zlib_decompress`].
pub fn zlib_decompress_within(input: &[u8], limit: OutputLimit) -> Result<Vec<u8>, InflateError> {
    let header = input.get(..2).ok_or(InflateError::TruncatedInput)?;
    let compression_method = header.first().copied().unwrap_or_default();
    let flags = header.get(1).copied().unwrap_or_default();
    check_zlib_header(compression_method, flags)?;
    let trailer_start = input
        .len()
        .checked_sub(4)
        .ok_or(InflateError::TruncatedInput)?;
    let body = input
        .get(2..trailer_start)
        .ok_or(InflateError::TruncatedInput)?;
    let output = inflate_within(body, limit)?;
    let expected = read_big_endian_u32(input.get(trailer_start..))?;
    if adler32(&output) != expected {
        return Err(InflateError::ChecksumMismatch);
    }
    Ok(output)
}

fn check_zlib_header(compression_method: u8, flags: u8) -> Result<(), InflateError> {
    if compression_method & 0x0F != 8 {
        return Err(InflateError::MalformedZlibHeader);
    }
    if flags & 0x20 != 0 {
        return Err(InflateError::MalformedZlibHeader);
    }
    let check = u16::from(compression_method)
        .wrapping_mul(256)
        .wrapping_add(u16::from(flags));
    if check.checked_rem(31) != Some(0) {
        return Err(InflateError::MalformedZlibHeader);
    }
    Ok(())
}

/// Decode a gzip (RFC 1952) stream under the default ceiling.
///
/// # Errors
///
/// [`InflateError`] for a bad header, a malformed body or a CRC-32 mismatch.
pub fn gzip_decompress(input: &[u8]) -> Result<Vec<u8>, InflateError> {
    gzip_decompress_within(input, OutputLimit::DEFAULT)
}

/// Decode a gzip stream under an explicit ceiling.
///
/// # Errors
///
/// As [`gzip_decompress`].
pub fn gzip_decompress_within(input: &[u8], limit: OutputLimit) -> Result<Vec<u8>, InflateError> {
    let body_start = gzip_header_length(input)?;
    let trailer_start = input
        .len()
        .checked_sub(8)
        .ok_or(InflateError::TruncatedInput)?;
    if body_start > trailer_start {
        return Err(InflateError::TruncatedInput);
    }
    let body = input
        .get(body_start..trailer_start)
        .ok_or(InflateError::TruncatedInput)?;
    let output = inflate_within(body, limit)?;
    let trailer = input.get(trailer_start..).unwrap_or_default();
    let expected_crc = read_little_endian_u32(trailer.get(..4))?;
    let expected_size = read_little_endian_u32(trailer.get(4..8))?;
    if crc32(&output) != expected_crc {
        return Err(InflateError::ChecksumMismatch);
    }
    if u32::try_from(output.len() & 0xFFFF_FFFF).unwrap_or(u32::MAX) != expected_size {
        return Err(InflateError::ChecksumMismatch);
    }
    Ok(output)
}

/// RFC 1952 §2.3: fixed 10-byte header, then the optional FEXTRA / FNAME /
/// FCOMMENT / FHCRC fields the flag byte announces.
fn gzip_header_length(input: &[u8]) -> Result<usize, InflateError> {
    let fixed = input.get(..10).ok_or(InflateError::TruncatedInput)?;
    if fixed.first() != Some(&0x1F) || fixed.get(1) != Some(&0x8B) {
        return Err(InflateError::MalformedGzipHeader);
    }
    if fixed.get(2) != Some(&8) {
        return Err(InflateError::MalformedGzipHeader);
    }
    let flags = fixed.get(3).copied().unwrap_or_default();
    let mut cursor = 10_usize;
    cursor = skip_extra_field(input, cursor, flags)?;
    cursor = skip_terminated_field(input, cursor, flags & 0x08)?;
    cursor = skip_terminated_field(input, cursor, flags & 0x10)?;
    if flags & 0x02 != 0 {
        cursor = cursor.checked_add(2).ok_or(InflateError::TruncatedInput)?;
    }
    Ok(cursor)
}

fn skip_extra_field(input: &[u8], cursor: usize, flags: u8) -> Result<usize, InflateError> {
    if flags & 0x04 == 0 {
        return Ok(cursor);
    }
    let end = cursor.checked_add(2).ok_or(InflateError::TruncatedInput)?;
    let field = input.get(cursor..end).ok_or(InflateError::TruncatedInput)?;
    let low = field.first().copied().unwrap_or_default();
    let high = field.get(1).copied().unwrap_or_default();
    let extra_length = usize::from(u16::from(low) | u16::from(high).wrapping_mul(256));
    end.checked_add(extra_length)
        .ok_or(InflateError::TruncatedInput)
}

fn skip_terminated_field(input: &[u8], cursor: usize, flag: u8) -> Result<usize, InflateError> {
    if flag == 0 {
        return Ok(cursor);
    }
    let tail = input.get(cursor..).ok_or(InflateError::TruncatedInput)?;
    let terminator = tail
        .iter()
        .position(|byte| *byte == 0)
        .ok_or(InflateError::MalformedGzipHeader)?;
    cursor
        .checked_add(terminator)
        .and_then(|index| index.checked_add(1))
        .ok_or(InflateError::TruncatedInput)
}

fn read_little_endian_u32(bytes: Option<&[u8]>) -> Result<u32, InflateError> {
    let slice = bytes.ok_or(InflateError::TruncatedInput)?;
    let array: [u8; 4] = slice.try_into().map_err(|_| InflateError::TruncatedInput)?;
    Ok(u32::from_le_bytes(array))
}

fn read_big_endian_u32(bytes: Option<&[u8]>) -> Result<u32, InflateError> {
    let slice = bytes.ok_or(InflateError::TruncatedInput)?;
    let array: [u8; 4] = slice.try_into().map_err(|_| InflateError::TruncatedInput)?;
    Ok(u32::from_be_bytes(array))
}

/// RFC 1952 CRC-32, computed bit by bit — no 1 KiB table to carry around, and
/// the checksum is not on a hot path.
#[must_use]
fn crc32(bytes: &[u8]) -> u32 {
    let mut register: u32 = 0xFFFF_FFFF;
    for byte in bytes {
        register ^= u32::from(*byte);
        for _ in 0..8_u8 {
            let mask = (register & 1).wrapping_neg();
            register = register.wrapping_shr(1) ^ (0xEDB8_8320 & mask);
        }
    }
    register ^ 0xFFFF_FFFF
}

/// RFC 1950 Adler-32. The conditional subtraction replaces `%`: both
/// accumulators stay below `2 * BASE`, so one subtraction always suffices.
#[must_use]
fn adler32(bytes: &[u8]) -> u32 {
    const BASE: u32 = 65_521;
    let mut low: u32 = 1;
    let mut high: u32 = 0;
    for byte in bytes {
        low = reduce(low.wrapping_add(u32::from(*byte)), BASE);
        high = reduce(high.wrapping_add(low), BASE);
    }
    high.wrapping_shl(16) | low
}

const fn reduce(value: u32, base: u32) -> u32 {
    if value >= base {
        return value.wrapping_sub(base);
    }
    value
}

// ---- the bit reader ---------------------------------------------------------

/// A least-significant-bit-first reader over the compressed input, the bit
/// order RFC 1951 §3.1.1 specifies.
struct BitReader<'input> {
    input: &'input [u8],
    position: usize,
    accumulator: u64,
    available: u32,
}

impl<'input> BitReader<'input> {
    const fn new(input: &'input [u8]) -> Self {
        Self {
            input,
            position: 0,
            accumulator: 0,
            available: 0,
        }
    }

    fn take_bits(&mut self, count: u32) -> Result<u32, InflateError> {
        if count == 0 {
            return Ok(0);
        }
        self.fill(count)?;
        let mask = 1_u64
            .checked_shl(count)
            .ok_or(InflateError::InvalidCodeLengths)?
            .wrapping_sub(1);
        let value = self.accumulator & mask;
        self.accumulator = self.accumulator.checked_shr(count).unwrap_or_default();
        self.available = self
            .available
            .checked_sub(count)
            .ok_or(InflateError::TruncatedInput)?;
        u32::try_from(value).map_err(|_| InflateError::InvalidCodeLengths)
    }

    fn fill(&mut self, count: u32) -> Result<(), InflateError> {
        while self.available < count {
            let byte = *self
                .input
                .get(self.position)
                .ok_or(InflateError::TruncatedInput)?;
            self.position = self
                .position
                .checked_add(1)
                .ok_or(InflateError::TruncatedInput)?;
            let shifted = u64::from(byte)
                .checked_shl(self.available)
                .ok_or(InflateError::TruncatedInput)?;
            self.accumulator |= shifted;
            self.available = self
                .available
                .checked_add(8)
                .ok_or(InflateError::TruncatedInput)?;
        }
        Ok(())
    }

    /// Discard the partial byte a stored block must not read across
    /// (RFC 1951 §3.2.4).
    fn align_to_byte(&mut self) {
        let stray = self.available & 7;
        self.accumulator = self.accumulator.checked_shr(stray).unwrap_or_default();
        self.available = self.available.saturating_sub(stray);
    }
}

// ---- canonical Huffman ------------------------------------------------------

/// A canonical Huffman table in "count and offset" form (RFC 1951 §3.2.2),
/// decoded one bit at a time. No lookup table, so no table-construction
/// arithmetic to get wrong on hostile input.
struct HuffmanTable {
    counts: Vec<u16>,
    symbols: Vec<u16>,
}

impl HuffmanTable {
    fn from_code_lengths(lengths: &[u8]) -> Result<Self, InflateError> {
        let counts = Self::count_lengths(lengths)?;
        let symbols = Self::order_symbols(lengths, &counts)?;
        Ok(Self { counts, symbols })
    }

    fn count_lengths(lengths: &[u8]) -> Result<Vec<u16>, InflateError> {
        let mut counts = vec![0_u16; MAX_CODE_LENGTH.saturating_add(1)];
        for length in lengths {
            let slot = counts
                .get_mut(usize::from(*length))
                .ok_or(InflateError::InvalidCodeLengths)?;
            *slot = slot
                .checked_add(1)
                .ok_or(InflateError::InvalidCodeLengths)?;
        }
        Ok(counts)
    }

    fn order_symbols(lengths: &[u8], counts: &[u16]) -> Result<Vec<u16>, InflateError> {
        let mut offsets = vec![0_u16; MAX_CODE_LENGTH.saturating_add(2)];
        for length in 1..=MAX_CODE_LENGTH {
            let running = offsets
                .get(length)
                .copied()
                .ok_or(InflateError::InvalidCodeLengths)?;
            let count = counts
                .get(length)
                .copied()
                .ok_or(InflateError::InvalidCodeLengths)?;
            let next = running
                .checked_add(count)
                .ok_or(InflateError::InvalidCodeLengths)?;
            let slot = offsets
                .get_mut(length.saturating_add(1))
                .ok_or(InflateError::InvalidCodeLengths)?;
            *slot = next;
        }
        let mut symbols = vec![0_u16; lengths.len()];
        for (symbol, length) in lengths.iter().enumerate() {
            if *length == 0 {
                continue;
            }
            let cursor = offsets
                .get_mut(usize::from(*length))
                .ok_or(InflateError::InvalidCodeLengths)?;
            let position = usize::from(*cursor);
            *cursor = cursor
                .checked_add(1)
                .ok_or(InflateError::InvalidCodeLengths)?;
            let slot = symbols
                .get_mut(position)
                .ok_or(InflateError::InvalidCodeLengths)?;
            *slot = u16::try_from(symbol).map_err(|_| InflateError::InvalidCodeLengths)?;
        }
        Ok(symbols)
    }

    fn decode(&self, reader: &mut BitReader<'_>) -> Result<u16, InflateError> {
        let mut code: u32 = 0;
        let mut first: u32 = 0;
        let mut index: u32 = 0;
        for length in 1..=MAX_CODE_LENGTH {
            code = code
                .checked_add(reader.take_bits(1)?)
                .ok_or(InflateError::InvalidHuffmanCode)?;
            let count = u32::from(
                self.counts
                    .get(length)
                    .copied()
                    .ok_or(InflateError::InvalidHuffmanCode)?,
            );
            if let Some(symbol) = self.symbol_at(code, first, index, count)? {
                return Ok(symbol);
            }
            index = index
                .checked_add(count)
                .ok_or(InflateError::InvalidHuffmanCode)?;
            first = first
                .checked_add(count)
                .and_then(|sum| sum.checked_shl(1))
                .ok_or(InflateError::InvalidHuffmanCode)?;
            code = code
                .checked_shl(1)
                .ok_or(InflateError::InvalidHuffmanCode)?;
        }
        Err(InflateError::InvalidHuffmanCode)
    }

    fn symbol_at(
        &self,
        code: u32,
        first: u32,
        index: u32,
        count: u32,
    ) -> Result<Option<u16>, InflateError> {
        let Some(offset) = code.checked_sub(first) else {
            return Err(InflateError::InvalidHuffmanCode);
        };
        if offset >= count {
            return Ok(None);
        }
        let position = index
            .checked_add(offset)
            .and_then(|sum| usize::try_from(sum).ok())
            .ok_or(InflateError::InvalidHuffmanCode)?;
        self.symbols
            .get(position)
            .copied()
            .map(Some)
            .ok_or(InflateError::InvalidHuffmanCode)
    }
}

/// RFC 1951 §3.2.6: the fixed literal/length code lengths.
fn fixed_literal_lengths() -> Vec<u8> {
    let mut lengths = vec![8_u8; LITERAL_SYMBOLS];
    for (symbol, length) in lengths.iter_mut().enumerate() {
        *length = fixed_literal_length_of(symbol);
    }
    lengths
}

const fn fixed_literal_length_of(symbol: usize) -> u8 {
    if symbol < 144 {
        return 8;
    }
    if symbol < 256 {
        return 9;
    }
    if symbol < 280 {
        return 7;
    }
    8
}

/// RFC 1951 §3.2.6: every fixed distance code is five bits.
fn fixed_distance_lengths() -> Vec<u8> {
    vec![5_u8; DISTANCE_SYMBOLS]
}

// ---- the decoder ------------------------------------------------------------

/// Drives the block loop and owns the output window.
struct Inflater<'input> {
    reader: BitReader<'input>,
    output: Vec<u8>,
    ceiling: usize,
}

impl<'input> Inflater<'input> {
    const fn new(input: &'input [u8], limit: OutputLimit) -> Self {
        Self {
            reader: BitReader::new(input),
            output: Vec::new(),
            ceiling: limit.bytes(),
        }
    }

    fn into_output(self) -> Vec<u8> {
        self.output
    }

    fn run(&mut self) -> Result<(), InflateError> {
        loop {
            let is_final = self.reader.take_bits(1)?;
            let kind = self.reader.take_bits(2)?;
            self.decode_block(kind)?;
            if is_final == 1 {
                return Ok(());
            }
        }
    }

    fn decode_block(&mut self, kind: u32) -> Result<(), InflateError> {
        match kind {
            0 => self.decode_stored_block(),
            1 => self.decode_compressed_block(
                &HuffmanTable::from_code_lengths(&fixed_literal_lengths())?,
                &HuffmanTable::from_code_lengths(&fixed_distance_lengths())?,
            ),
            2 => self.decode_dynamic_block(),
            _ => Err(InflateError::ReservedBlockType),
        }
    }

    fn decode_stored_block(&mut self) -> Result<(), InflateError> {
        self.reader.align_to_byte();
        let length = self.reader.take_bits(16)?;
        let complement = self.reader.take_bits(16)?;
        if length ^ 0xFFFF != complement {
            return Err(InflateError::StoredLengthMismatch);
        }
        for _ in 0..length {
            let byte = u8::try_from(self.reader.take_bits(8)?)
                .map_err(|_| InflateError::TruncatedInput)?;
            self.push(byte)?;
        }
        Ok(())
    }

    fn decode_dynamic_block(&mut self) -> Result<(), InflateError> {
        let literal_count = usize::try_from(self.reader.take_bits(5)?)
            .ok()
            .and_then(|count| count.checked_add(257))
            .ok_or(InflateError::InvalidCodeLengths)?;
        let distance_count = usize::try_from(self.reader.take_bits(5)?)
            .ok()
            .and_then(|count| count.checked_add(1))
            .ok_or(InflateError::InvalidCodeLengths)?;
        let code_length_count = usize::try_from(self.reader.take_bits(4)?)
            .ok()
            .and_then(|count| count.checked_add(4))
            .ok_or(InflateError::InvalidCodeLengths)?;
        let code_length_table = self.read_code_length_table(code_length_count)?;
        let total = literal_count
            .checked_add(distance_count)
            .ok_or(InflateError::InvalidCodeLengths)?;
        let lengths = self.read_code_lengths(&code_length_table, total)?;
        let literal_lengths = lengths
            .get(..literal_count)
            .ok_or(InflateError::InvalidCodeLengths)?;
        let distance_lengths = lengths
            .get(literal_count..)
            .ok_or(InflateError::InvalidCodeLengths)?;
        self.decode_compressed_block(
            &HuffmanTable::from_code_lengths(literal_lengths)?,
            &HuffmanTable::from_code_lengths(distance_lengths)?,
        )
    }

    fn read_code_length_table(&mut self, count: usize) -> Result<HuffmanTable, InflateError> {
        let mut lengths = vec![0_u8; CODE_LENGTH_SYMBOLS];
        for position in CODE_LENGTH_ORDER.iter().take(count) {
            let value = u8::try_from(self.reader.take_bits(3)?)
                .map_err(|_| InflateError::TruncatedInput)?;
            let slot = lengths
                .get_mut(*position)
                .ok_or(InflateError::InvalidCodeLengths)?;
            *slot = value;
        }
        HuffmanTable::from_code_lengths(&lengths)
    }

    fn read_code_lengths(
        &mut self,
        table: &HuffmanTable,
        total: usize,
    ) -> Result<Vec<u8>, InflateError> {
        let mut lengths: Vec<u8> = Vec::with_capacity(total);
        while lengths.len() < total {
            let symbol = table.decode(&mut self.reader)?;
            self.extend_code_lengths(&mut lengths, symbol)?;
        }
        if lengths.len() > total {
            return Err(InflateError::InvalidCodeLengths);
        }
        Ok(lengths)
    }

    /// RFC 1951 §3.2.7: symbols `0..=15` are literal lengths, `16` repeats the
    /// previous length, `17` and `18` run zeroes.
    fn extend_code_lengths(
        &mut self,
        lengths: &mut Vec<u8>,
        symbol: u16,
    ) -> Result<(), InflateError> {
        if symbol <= 15 {
            lengths.push(u8::try_from(symbol).map_err(|_| InflateError::InvalidCodeLengths)?);
            return Ok(());
        }
        let (value, repeat) = self.repeat_instruction(lengths, symbol)?;
        for _ in 0..repeat {
            lengths.push(value);
        }
        Ok(())
    }

    fn repeat_instruction(
        &mut self,
        lengths: &[u8],
        symbol: u16,
    ) -> Result<(u8, u32), InflateError> {
        if symbol == 16 {
            let previous = lengths
                .last()
                .copied()
                .ok_or(InflateError::InvalidCodeLengths)?;
            let repeat = self
                .reader
                .take_bits(2)?
                .checked_add(3)
                .ok_or(InflateError::InvalidCodeLengths)?;
            return Ok((previous, repeat));
        }
        if symbol == 17 {
            let repeat = self
                .reader
                .take_bits(3)?
                .checked_add(3)
                .ok_or(InflateError::InvalidCodeLengths)?;
            return Ok((0, repeat));
        }
        if symbol == 18 {
            let repeat = self
                .reader
                .take_bits(7)?
                .checked_add(11)
                .ok_or(InflateError::InvalidCodeLengths)?;
            return Ok((0, repeat));
        }
        Err(InflateError::InvalidCodeLengths)
    }

    fn decode_compressed_block(
        &mut self,
        literals: &HuffmanTable,
        distances: &HuffmanTable,
    ) -> Result<(), InflateError> {
        loop {
            let symbol = literals.decode(&mut self.reader)?;
            if symbol == END_OF_BLOCK {
                return Ok(());
            }
            if symbol < END_OF_BLOCK {
                let byte = u8::try_from(symbol).map_err(|_| InflateError::InvalidHuffmanCode)?;
                self.push(byte)?;
                continue;
            }
            let length = self.read_copy_length(symbol)?;
            let distance = self.read_copy_distance(distances)?;
            self.copy_back_reference(length, distance)?;
        }
    }

    fn read_copy_length(&mut self, symbol: u16) -> Result<usize, InflateError> {
        let index = usize::from(
            symbol
                .checked_sub(FIRST_LENGTH_SYMBOL)
                .ok_or(InflateError::InvalidHuffmanCode)?,
        );
        let base = LENGTH_BASE
            .get(index)
            .copied()
            .ok_or(InflateError::InvalidHuffmanCode)?;
        let extra_bits = LENGTH_EXTRA
            .get(index)
            .copied()
            .ok_or(InflateError::InvalidHuffmanCode)?;
        let extra = self.reader.take_bits(u32::from(extra_bits))?;
        usize::try_from(
            u32::from(base)
                .checked_add(extra)
                .ok_or(InflateError::InvalidHuffmanCode)?,
        )
        .map_err(|_| InflateError::InvalidHuffmanCode)
    }

    fn read_copy_distance(&mut self, distances: &HuffmanTable) -> Result<usize, InflateError> {
        let symbol = distances.decode(&mut self.reader)?;
        let index = usize::from(symbol);
        let base = DISTANCE_BASE
            .get(index)
            .copied()
            .ok_or(InflateError::InvalidHuffmanCode)?;
        let extra_bits = DISTANCE_EXTRA
            .get(index)
            .copied()
            .ok_or(InflateError::InvalidHuffmanCode)?;
        let extra = self.reader.take_bits(u32::from(extra_bits))?;
        usize::try_from(
            u32::from(base)
                .checked_add(extra)
                .ok_or(InflateError::DistanceTooFar)?,
        )
        .map_err(|_| InflateError::DistanceTooFar)
    }

    /// LZ77 back-reference. Copied byte by byte on purpose: a run may overlap
    /// itself (`distance < length`), which is how DEFLATE encodes repeats.
    fn copy_back_reference(&mut self, length: usize, distance: usize) -> Result<(), InflateError> {
        let start = self
            .output
            .len()
            .checked_sub(distance)
            .ok_or(InflateError::DistanceTooFar)?;
        for offset in 0..length {
            let index = start
                .checked_add(offset)
                .ok_or(InflateError::DistanceTooFar)?;
            let byte = self
                .output
                .get(index)
                .copied()
                .ok_or(InflateError::DistanceTooFar)?;
            self.push(byte)?;
        }
        Ok(())
    }

    fn push(&mut self, byte: u8) -> Result<(), InflateError> {
        if self.output.len() >= self.ceiling {
            return Err(InflateError::OutputLimitExceeded {
                limit: self.ceiling,
            });
        }
        self.output.push(byte);
        Ok(())
    }
}

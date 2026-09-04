//! Turning a response body into text.
//!
//! The decision order is fixed by the v0.5 plan ("Fase C1") and matches what
//! the HTML standard prescribes:
//!
//! 1. a **byte-order mark**, which overrides everything;
//! 2. the **`charset` parameter of `Content-Type`**;
//! 3. a **`<meta charset>`** in the first [`SNIFF_WINDOW`] bytes;
//! 4. **windows-1252**, the legacy fallback.
//!
//! Two encodings are implemented, and only two. A label outside them — or a
//! UTF-16 byte-order mark — is a typed [`NetworkError`] in the
//! [`ProtocolPhase::Decode`](crate::domain::phase::ProtocolPhase::Decode)
//! phase, never a silently mis-decoded page. Mojibake that renders is worse
//! than an error that says what happened.
//!
//! Pure `&[u8] -> Result<String, NetworkError>`: no socket, no crypto, so this
//! module is not gated behind `real-transport`.

use crate::domain::defect::DecodeDefect;
use crate::domain::error::NetworkError;
use crate::domain::media_type::Charset;

/// How far into a body a `<meta charset>` is looked for. The HTML standard's
/// pre-scan window.
pub const SNIFF_WINDOW: usize = 1024;

/// The UTF-8 byte-order mark.
const UTF8_BOM: [u8; 3] = [0xEF, 0xBB, 0xBF];
/// The UTF-16 big-endian byte-order mark.
const UTF16_BE_BOM: [u8; 2] = [0xFE, 0xFF];
/// The UTF-16 little-endian byte-order mark.
const UTF16_LE_BOM: [u8; 2] = [0xFF, 0xFE];

/// The 32 code points where windows-1252 differs from ISO-8859-1 (bytes
/// `0x80..=0x9F`). Every other byte maps to the code point of the same value,
/// so `windows_1252_char` is a total function over all 256 bytes.
///
/// The five positions WHATWG leaves unmapped (`0x81`, `0x8D`, `0x8F`, `0x90`,
/// `0x9D`) decode to their C1 control, which is what the Encoding Standard
/// specifies — not to `U+FFFD`.
const WINDOWS_1252_HIGH: [char; 32] = [
    '\u{20AC}', '\u{0081}', '\u{201A}', '\u{0192}', '\u{201E}', '\u{2026}', '\u{2020}', '\u{2021}',
    '\u{02C6}', '\u{2030}', '\u{0160}', '\u{2039}', '\u{0152}', '\u{008D}', '\u{017D}', '\u{008F}',
    '\u{0090}', '\u{2018}', '\u{2019}', '\u{201C}', '\u{201D}', '\u{2022}', '\u{2013}', '\u{2014}',
    '\u{02DC}', '\u{2122}', '\u{0161}', '\u{203A}', '\u{0153}', '\u{009D}', '\u{017E}', '\u{0178}',
];

/// Decode a body to text, choosing the encoding by the four-step order above.
///
/// A UTF-8 byte-order mark is consumed rather than left in the text.
///
/// # Errors
///
/// [`NetworkError::Decode`] when a byte-order mark or a label names an
/// encoding this engine does not implement.
pub fn decode_document(bytes: &[u8], declared: Option<Charset>) -> Result<String, NetworkError> {
    let charset = detect(bytes, declared)?;
    let payload = strip_byte_order_mark(bytes, charset);
    Ok(decode_with(payload, charset))
}

/// Decide which encoding a body is in, without decoding it.
///
/// # Errors
///
/// [`NetworkError::Decode`] with [`DecodeDefect::UnsupportedByteOrderMark`] for
/// a UTF-16 mark, or [`DecodeDefect::UnsupportedCharset`] when a `<meta>` names
/// an unimplemented label.
pub fn detect(bytes: &[u8], declared: Option<Charset>) -> Result<Charset, NetworkError> {
    if let Some(from_mark) = byte_order_mark(bytes)? {
        return Ok(from_mark);
    }
    if let Some(from_header) = declared {
        return Ok(from_header);
    }
    if let Some(from_meta) = meta_charset(bytes)? {
        return Ok(from_meta);
    }
    Ok(Charset::Windows1252)
}

/// Decode bytes with an encoding already chosen.
#[must_use]
pub fn decode_with(bytes: &[u8], charset: Charset) -> String {
    match charset {
        Charset::Utf8 => String::from_utf8_lossy(bytes).into_owned(),
        Charset::Windows1252 => bytes.iter().copied().map(windows_1252_char).collect(),
    }
}

/// The windows-1252 code point of a byte. Total over all 256 inputs.
#[must_use]
pub fn windows_1252_char(byte: u8) -> char {
    let Some(offset) = byte.checked_sub(0x80) else {
        return char::from(byte);
    };
    if byte > 0x9F {
        return char::from(byte);
    }
    WINDOWS_1252_HIGH
        .get(usize::from(offset))
        .copied()
        .unwrap_or(char::REPLACEMENT_CHARACTER)
}

fn byte_order_mark(bytes: &[u8]) -> Result<Option<Charset>, NetworkError> {
    if bytes.starts_with(&UTF8_BOM) {
        return Ok(Some(Charset::Utf8));
    }
    if bytes.starts_with(&UTF16_BE_BOM) || bytes.starts_with(&UTF16_LE_BOM) {
        return Err(NetworkError::decode(DecodeDefect::UnsupportedByteOrderMark));
    }
    Ok(None)
}

fn strip_byte_order_mark(bytes: &[u8], charset: Charset) -> &[u8] {
    if charset == Charset::Utf8 && bytes.starts_with(&UTF8_BOM) {
        return bytes.get(UTF8_BOM.len()..).unwrap_or_default();
    }
    bytes
}

/// Look for `charset` in the pre-scan window and read the label that follows
/// it. Deliberately simple-minded: it finds `<meta charset=…>` and the
/// `charset=` inside a `<meta http-equiv>` content attribute alike, which is
/// all step 3 is for. Full `<meta>` parsing is `core/html`'s job (B5).
fn meta_charset(bytes: &[u8]) -> Result<Option<Charset>, NetworkError> {
    let window = bytes.get(..SNIFF_WINDOW).unwrap_or(bytes);
    let lowered: Vec<u8> = window.iter().map(u8::to_ascii_lowercase).collect();
    let Some(start) = find_keyword(&lowered) else {
        return Ok(None);
    };
    let Some(tail) = lowered.get(start..) else {
        return Ok(None);
    };
    let Some(label) = read_label(tail) else {
        return Ok(None);
    };
    Charset::from_label(&label).map(Some)
}

fn find_keyword(lowered: &[u8]) -> Option<usize> {
    let position = lowered
        .windows(b"charset".len())
        .position(|candidate| candidate == b"charset")?;
    position.checked_add(b"charset".len())
}

/// After `charset`: optional whitespace, `=`, optional whitespace, an optional
/// quote, then the label itself.
fn read_label(tail: &[u8]) -> Option<String> {
    let mut cursor = tail.iter().copied().skip_while(u8::is_ascii_whitespace);
    if cursor.next() != Some(b'=') {
        return None;
    }
    let mut value = cursor.skip_while(u8::is_ascii_whitespace).peekable();
    let quote = match value.peek() {
        Some(b'"') => Some(b'"'),
        Some(b'\'') => Some(b'\''),
        _ => None,
    };
    if quote.is_some() {
        let _ = value.next();
    }
    let label: Vec<u8> = value
        .take_while(|byte| !ends_label(*byte, quote))
        .take(64)
        .collect();
    if label.is_empty() {
        return None;
    }
    String::from_utf8(label).ok()
}

const fn ends_label(byte: u8, quote: Option<u8>) -> bool {
    if let Some(delimiter) = quote {
        return byte == delimiter;
    }
    byte.is_ascii_whitespace() || matches!(byte, b';' | b'>' | b'/' | b'"' | b'\'')
}

//! The pure byte codecs of `core/network` — the RFC 1951 `inflate` (reused by
//! Phase X for PNG) and the charset decoder. Both read attacker-controlled
//! bytes (`ADR-0018` row 1) and must fail typed, never panic.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use network::inflate::{InflateError, inflate};
use network::infrastructure::charset::{decode_document, decode_with};
use network::{Charset, NetworkError};

/// A single RFC 1951 *stored* (uncompressed) block holding `abc`:
/// `BFINAL=1, BTYPE=00`, `LEN=3`, `NLEN=~3`, then the literal bytes.
const STORED_ABC: [u8; 8] = [0x01, 0x03, 0x00, 0xFC, 0xFF, b'a', b'b', b'c'];

#[test]
fn inflate_decodes_a_stored_block() {
    assert_eq!(inflate(&STORED_ABC).unwrap(), b"abc");
}

#[test]
fn inflate_rejects_a_truncated_stream_typed() {
    let outcome = inflate(&STORED_ABC[..4]);
    assert!(
        matches!(
            outcome,
            Err(InflateError::TruncatedInput | InflateError::StoredLengthMismatch)
        ),
        "a truncated deflate stream must be a typed InflateError, got {outcome:?}"
    );
}

#[test]
fn inflate_rejects_an_empty_input_typed() {
    assert!(inflate(&[]).is_err(), "an empty stream has no final block");
}

#[test]
fn utf8_is_decoded_and_invalid_bytes_become_replacement() {
    assert_eq!(decode_with(b"caf\xC3\xA9", Charset::Utf8), "café");
    assert_eq!(decode_with(b"a\xFFb", Charset::Utf8), "a\u{FFFD}b");
}

#[test]
fn windows_1252_high_bytes_map_through_the_table() {
    // 0x80 is the Euro sign in windows-1252, unlike Latin-1.
    assert_eq!(decode_with(&[0x80], Charset::Windows1252), "\u{20AC}");
}

#[test]
fn a_bom_wins_over_a_declared_charset() {
    let with_bom = [0xEF, 0xBB, 0xBF, b'h', b'i'];
    assert_eq!(
        decode_document(&with_bom, Some(Charset::Windows1252)).unwrap(),
        "hi"
    );
}

#[test]
fn an_unsupported_charset_label_is_a_typed_decode_error() {
    assert!(matches!(
        Charset::from_label("shift_jis"),
        Err(NetworkError::Decode { .. })
    ));
    assert_eq!(Charset::from_label("UTF8").unwrap(), Charset::Utf8);
}

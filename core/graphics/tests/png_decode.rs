//! Unit tests for `png_decode::decode_png` (v0.5 Phase X).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects
)]

use graphics::infrastructure::png_decode::{PngDecodeError, decode_png};
use graphics::{Color, Framebuffer, SurfaceSize};

/// The standard 8-byte PNG signature.
const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];

/// Adler-32 checksum calculation for zlib container test fixtures.
fn adler32(bytes: &[u8]) -> u32 {
    let mut low = 1_u32;
    let mut high = 0_u32;
    for byte in bytes {
        low = (low.saturating_add(u32::from(*byte))) % 65_521;
        high = (high.saturating_add(low)) % 65_521;
    }
    high.saturating_mul(65_536).saturating_add(low)
}

/// Constructs a simple zlib-wrapped stored DEFLATE stream.
fn wrap_zlib_stored(payload: &[u8]) -> Vec<u8> {
    let mut stream = Vec::new();
    stream.extend_from_slice(&[0x78, 0x01]); // zlib header

    let mut remaining = payload;
    while !remaining.is_empty() {
        let block_len = remaining.len().min(0xffff);
        let block = remaining.get(..block_len).unwrap();
        remaining = remaining.get(block_len..).unwrap_or(&[]);
        let final_block = remaining.is_empty();

        stream.push(u8::from(final_block));
        let len_u16 = u16::try_from(block_len).unwrap();
        stream.extend_from_slice(&len_u16.to_le_bytes());
        stream.extend_from_slice(&(!len_u16).to_le_bytes());
        stream.extend_from_slice(block);
    }

    let checksum = adler32(payload);
    stream.extend_from_slice(&checksum.to_be_bytes());
    stream
}

/// Helper to build a valid PNG byte vector.
fn build_png(
    width: u32,
    height: u32,
    color_type: u8,
    bit_depth: u8,
    raw_scanlines: &[u8],
) -> Vec<u8> {
    let mut png = Vec::new();
    png.extend_from_slice(&SIGNATURE);

    // IHDR
    let mut ihdr_payload = Vec::new();
    ihdr_payload.extend_from_slice(&width.to_be_bytes());
    ihdr_payload.extend_from_slice(&height.to_be_bytes());
    ihdr_payload.push(bit_depth);
    ihdr_payload.push(color_type);
    ihdr_payload.push(0); // compression = deflate
    ihdr_payload.push(0); // filter = adaptive
    ihdr_payload.push(0); // interlace = none
    append_chunk(&mut png, *b"IHDR", &ihdr_payload);

    // IDAT
    let zlib_data = wrap_zlib_stored(raw_scanlines);
    append_chunk(&mut png, *b"IDAT", &zlib_data);

    // IEND
    append_chunk(&mut png, *b"IEND", &[]);

    png
}

fn append_chunk(dest: &mut Vec<u8>, kind: [u8; 4], payload: &[u8]) {
    let len = u32::try_from(payload.len()).unwrap();
    dest.extend_from_slice(&len.to_be_bytes());
    dest.extend_from_slice(&kind);
    dest.extend_from_slice(payload);

    let mut to_check = Vec::with_capacity(payload.len().saturating_add(4));
    to_check.extend_from_slice(&kind);
    to_check.extend_from_slice(payload);
    let crc = graphics::infrastructure::png::crc32(&to_check);
    dest.extend_from_slice(&crc.to_be_bytes());
}

#[test]
fn decodes_an_encoded_framebuffer_roundtrip() {
    let size = SurfaceSize::new(2, 2).unwrap();
    let mut original = Framebuffer::filled(size, Color::TRANSPARENT).unwrap();
    original.set_pixel(0, 0, Color::rgba(255, 0, 0, 255));
    original.set_pixel(1, 0, Color::rgba(0, 255, 0, 255));
    original.set_pixel(0, 1, Color::rgba(0, 0, 255, 255));
    original.set_pixel(1, 1, Color::rgba(255, 255, 0, 128));

    let encoded = graphics::png::encode(&original);
    let decoded = decode_png(&encoded).expect("roundtrip should decode successfully");

    assert_eq!(decoded, original);
}

#[test]
fn decodes_rgb_image_and_adds_opaque_alpha() {
    // 2x1 RGB image: pixel 1 = (10, 20, 30), pixel 2 = (40, 50, 60)
    // Scanline: [filter_byte: 0, 10, 20, 30, 40, 50, 60]
    let raw = [0, 10, 20, 30, 40, 50, 60];
    let png = build_png(2, 1, 2, 8, &raw);

    let decoded = decode_png(&png).expect("valid RGB png");
    assert_eq!(decoded.width(), 2);
    assert_eq!(decoded.height(), 1);
    assert_eq!(decoded.pixel(0, 0), Some(Color::rgba(10, 20, 30, 255)));
    assert_eq!(decoded.pixel(1, 0), Some(Color::rgba(40, 50, 60, 255)));
}

#[test]
fn unfilters_all_five_filter_types() {
    // 3x5 RGBA image: each row uses a different filter type (0..=4)
    // Row 0 (None): filter 0, 3 pixels
    // Row 1 (Sub): filter 1
    // Row 2 (Up): filter 2
    // Row 3 (Average): filter 3
    // Row 4 (Paeth): filter 4

    let mut scanlines = Vec::new();

    // Row 0: filter 0. P0=(10,0,0,255), P1=(20,0,0,255), P2=(30,0,0,255)
    scanlines.push(0);
    scanlines.extend_from_slice(&[10, 0, 0, 255, 20, 0, 0, 255, 30, 0, 0, 255]);

    // Row 1: filter 1 (Sub). Target: P0=(15,0,0,255), P1=(25,0,0,255), P2=(35,0,0,255)
    // Diff P0: (15 - 0) = 15
    // Diff P1: (25 - 15) = 10
    // Diff P2: (35 - 25) = 10
    // Alpha diffs: P0=(255 - 0)=255, P1=(255-255)=0, P2=(255-255)=0
    scanlines.push(1);
    scanlines.extend_from_slice(&[15, 0, 0, 255, 10, 0, 0, 0, 10, 0, 0, 0]);

    // Row 2: filter 2 (Up). Target: P0=(20,0,0,255), P1=(30,0,0,255), P2=(40,0,0,255)
    // Prior row was: P0=15, P1=25, P2=35.
    // Diff: 5 for each red, 0 for alpha
    scanlines.push(2);
    scanlines.extend_from_slice(&[5, 0, 0, 0, 5, 0, 0, 0, 5, 0, 0, 0]);

    // Row 3: filter 3 (Average).
    // Target: P0=(30,0,0,255), P1=(40,0,0,255), P2=(50,0,0,255)
    // P0: prior=20, a=0. avg = 10. filt = 30 - 10 = 20. alpha prior=255, a=0 -> avg=127. filt=255-127=128.
    // P1: prior=30, a=30. avg = 30. filt = 40 - 30 = 10. alpha: avg=255. filt=0.
    // P2: prior=40, a=40. avg = 40. filt = 50 - 40 = 10. alpha: avg=255. filt=0.
    scanlines.push(3);
    scanlines.extend_from_slice(&[20, 0, 0, 128, 10, 0, 0, 0, 10, 0, 0, 0]);

    // Row 4: filter 4 (Paeth).
    // Prior was (30, 40, 50) with alpha 255.
    // Target: (35, 45, 55)
    // For P0: a=0, b=30, c=0. p = 30. paeth(0, 30, 0) = 30. filt = 35 - 30 = 5.
    // For P1: a=35, b=40, c=30. p = 35 + 40 - 30 = 45. pa=10, pb=5, pc=15. paeth=40. filt = 45 - 40 = 5.
    // For P2: a=45, b=50, c=40. p = 45 + 50 - 40 = 55. paeth=50. filt = 55 - 50 = 5.
    scanlines.push(4);
    scanlines.extend_from_slice(&[5, 0, 0, 0, 5, 0, 0, 0, 5, 0, 0, 0]);

    let png = build_png(3, 5, 6, 8, &scanlines);
    let decoded = decode_png(&png).expect("all filter types should decode");

    assert_eq!(decoded.pixel(0, 0), Some(Color::rgba(10, 0, 0, 255)));
    assert_eq!(decoded.pixel(1, 0), Some(Color::rgba(20, 0, 0, 255)));
    assert_eq!(decoded.pixel(2, 0), Some(Color::rgba(30, 0, 0, 255)));

    assert_eq!(decoded.pixel(0, 1), Some(Color::rgba(15, 0, 0, 255)));
    assert_eq!(decoded.pixel(1, 1), Some(Color::rgba(25, 0, 0, 255)));
    assert_eq!(decoded.pixel(2, 1), Some(Color::rgba(35, 0, 0, 255)));

    assert_eq!(decoded.pixel(0, 2), Some(Color::rgba(20, 0, 0, 255)));
    assert_eq!(decoded.pixel(1, 2), Some(Color::rgba(30, 0, 0, 255)));
    assert_eq!(decoded.pixel(2, 2), Some(Color::rgba(40, 0, 0, 255)));

    assert_eq!(decoded.pixel(0, 3), Some(Color::rgba(30, 0, 0, 255)));
    assert_eq!(decoded.pixel(1, 3), Some(Color::rgba(40, 0, 0, 255)));
    assert_eq!(decoded.pixel(2, 3), Some(Color::rgba(50, 0, 0, 255)));

    assert_eq!(decoded.pixel(0, 4), Some(Color::rgba(35, 0, 0, 255)));
    assert_eq!(decoded.pixel(1, 4), Some(Color::rgba(45, 0, 0, 255)));
    assert_eq!(decoded.pixel(2, 4), Some(Color::rgba(55, 0, 0, 255)));
}

#[test]
fn refuses_invalid_or_truncated_signatures() {
    assert_eq!(decode_png(&[]), Err(PngDecodeError::Truncated));
    assert_eq!(decode_png(b"GIF89a"), Err(PngDecodeError::Truncated));
    assert_eq!(
        decode_png(b"NOTAPNG!12345678"),
        Err(PngDecodeError::InvalidSignature)
    );
}

#[test]
fn detects_corrupt_chunk_crc() {
    let mut png = build_png(1, 1, 6, 8, &[0, 255, 0, 0, 255]);
    // Corrupt one byte in the IHDR chunk payload
    let ihdr_offset = 8 + 8; // Signature (8) + len (4) + IHDR (4)
    png[ihdr_offset] ^= 0xff;

    let err = decode_png(&png).expect_err("corrupt CRC must be detected");
    match err {
        PngDecodeError::ChunkCorrupt { kind, .. } => assert_eq!(kind, "IHDR"),
        other => panic!("expected ChunkCorrupt, got {other:?}"),
    }
}

#[test]
fn refuses_unsupported_color_types_and_bit_depths() {
    // Color type 0 (Grayscale), depth 8
    let png_gray = build_png(1, 1, 0, 8, &[0, 128]);
    assert_eq!(
        decode_png(&png_gray),
        Err(PngDecodeError::UnsupportedColorType {
            color_type: 0,
            bit_depth: 8,
        })
    );

    // Color type 6 (RGBA), depth 16
    let png_16bit = build_png(1, 1, 6, 16, &[0, 0]);
    assert_eq!(
        decode_png(&png_16bit),
        Err(PngDecodeError::UnsupportedColorType {
            color_type: 6,
            bit_depth: 16,
        })
    );
}

#[test]
fn refuses_invalid_filters() {
    // Filter type 99 on row 0
    let raw = [99, 255, 0, 0, 255];
    let png = build_png(1, 1, 6, 8, &raw);

    assert_eq!(
        decode_png(&png),
        Err(PngDecodeError::InvalidFilter { filter: 99, row: 0 })
    );
}

#[test]
fn multiple_idat_chunks_are_concatenated() {
    let mut png = Vec::new();
    png.extend_from_slice(&SIGNATURE);

    // IHDR 1x1 RGBA
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&1_u32.to_be_bytes());
    ihdr.extend_from_slice(&1_u32.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    append_chunk(&mut png, *b"IHDR", &ihdr);

    let raw = [0, 12, 34, 56, 78];
    let zlib_data = wrap_zlib_stored(&raw);

    // Split zlib_data into two IDAT chunks
    let mid = zlib_data.len() / 2;
    append_chunk(&mut png, *b"IDAT", &zlib_data[..mid]);
    append_chunk(&mut png, *b"IDAT", &zlib_data[mid..]);
    append_chunk(&mut png, *b"IEND", &[]);

    let decoded = decode_png(&png).expect("multiple IDATs should decode seamlessly");
    assert_eq!(decoded.pixel(0, 0), Some(Color::rgba(12, 34, 56, 78)));
}

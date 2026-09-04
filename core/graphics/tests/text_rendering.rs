//! `DrawText` (v0.5 B3): the golden image, its determinism gate, and unit
//! coverage for the `FontProvider` adapters. Every golden/determinism test
//! here uses [`SyntheticFontProvider`] exclusively — deterministic, no
//! filesystem — so the PNG is byte-identical on Linux, macOS and Windows
//! (`ADR-0016`), the same discipline `golden_boxes.rs` established for rects.

#![cfg(feature = "software-backend")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;
use std::sync::Arc;

use graphics::{
    Au, Color, DisplayList, DisplayListBuilder, FaceMetrics, FontId, FontProvider, Framebuffer,
    GlyphBitmap, GlyphId, GlyphInstance, GlyphRun, Point, RenderBackend, SoftwareCpuBackend,
    SurfaceSize, SyntheticFontProvider, golden,
};

fn reference(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("golden")
        .join(name)
}

const FONT_SIZE: Au = Au::from_raw(16 * graphics::AU_PER_PX);
const TEXT_FONT: FontId = FontId::new(1);

fn synthetic_provider() -> Arc<dyn FontProvider> {
    Arc::new(SyntheticFontProvider::new().with_size(TEXT_FONT, FONT_SIZE))
}

fn glyph_run(count: u16, baseline_y: Au) -> GlyphRun {
    let advance = Au::from_whole_px(12).expect("12px is a valid extent");
    let mut run = GlyphRun::new();
    for index in 0..count {
        let x =
            advance.saturating_add(Au::from_raw(i32::from(index).saturating_mul(advance.raw())));
        run.push(GlyphInstance::new(
            GlyphId::new(index.saturating_add(1)),
            Point::new(x, baseline_y),
        ));
    }
    run
}

fn scene() -> DisplayList {
    let mut builder = DisplayListBuilder::new();
    let line_one = glyph_run(6, Au::from_whole_px(20).expect("valid"));
    let line_two = glyph_run(4, Au::from_whole_px(40).expect("valid"));
    builder
        .draw_text(line_one, Color::BLACK, TEXT_FONT)
        .expect("a well-formed glyph run is always accepted");
    builder
        .draw_text(line_two, Color::rgb(200, 0, 0), TEXT_FONT)
        .expect("a well-formed glyph run is always accepted");
    builder.build().expect("a balanced list always builds")
}

fn render(list: &DisplayList) -> Framebuffer {
    let mut backend = SoftwareCpuBackend::with_font_provider(synthetic_provider());
    backend
        .begin_frame(SurfaceSize::new(96, 64).expect("a non-zero surface"))
        .unwrap();
    backend.submit(list).unwrap();
    backend.end_frame().unwrap();
    backend.read_back().unwrap()
}

#[test]
fn the_text_scene_matches_its_golden_image() {
    let frame = render(&scene());
    golden::assert_matches_golden(&frame, &reference("text.png"));
}

#[test]
fn a_hundred_renders_of_the_text_scene_are_byte_identical() {
    let list = scene();
    let reference_frame = render(&list);
    for attempt in 0..100 {
        let frame = render(&list);
        assert_eq!(
            frame.as_rgba8(),
            reference_frame.as_rgba8(),
            "render {attempt} diverged: text rasterization is not deterministic"
        );
    }
}

#[test]
fn an_unregistered_font_is_a_typed_error_not_a_panic() {
    let provider = SyntheticFontProvider::new();
    let outcome = provider.rasterize(FontId::new(99), GlyphId::new(1));
    assert!(
        outcome.is_err(),
        "an unregistered font must be refused typed"
    );
}

#[test]
fn notdef_rasterizes_to_an_empty_bitmap() {
    let provider = SyntheticFontProvider::new().with_size(TEXT_FONT, FONT_SIZE);
    let bitmap = provider
        .rasterize(TEXT_FONT, GlyphId::NOTDEF)
        .expect("a registered font always answers");
    assert!(bitmap.is_empty(), "NOTDEF stands in for whitespace");
}

#[test]
fn a_registered_glyph_produces_a_fully_covered_block() {
    let provider = SyntheticFontProvider::new().with_size(TEXT_FONT, FONT_SIZE);
    let bitmap = provider
        .rasterize(TEXT_FONT, GlyphId::new(1))
        .expect("a registered font always answers");
    assert!(!bitmap.is_empty());
    assert_eq!(bitmap.coverage_at(0, 0), u8::MAX);
}

#[test]
fn synthetic_metrics_are_a_fixed_fraction_of_the_registered_size() {
    let provider = SyntheticFontProvider::new().with_size(TEXT_FONT, FONT_SIZE);
    let metrics: FaceMetrics = provider.metrics(TEXT_FONT).expect("registered font");
    assert!(metrics.ascent().raw() > 0);
    assert!(metrics.descent().raw() > 0);
    assert_eq!(
        metrics.line_height(),
        metrics.ascent().saturating_add(metrics.descent())
    );
}

#[test]
fn glyph_bitmap_refuses_a_mismatched_coverage_length() {
    assert!(GlyphBitmap::new(2, 2, vec![0; 3], Point::ORIGIN).is_none());
    assert!(GlyphBitmap::new(2, 2, vec![0; 4], Point::ORIGIN).is_some());
}

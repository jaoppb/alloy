//! `FontBackedMeasurer` (v0.5 B3): a real `graphics::FontProvider`-backed
//! `TextMeasurer`, proven here against the deterministic
//! `SyntheticFontProvider` — no filesystem, no installed fonts required for
//! this test to be meaningful on any machine.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;

use css::{ComputedText, FontBackedMeasurer, TextMeasurer, TextRun};
use graphics::{Au, FontId, SyntheticFontProvider};

const FONT: FontId = FontId::new(7);

fn measurer(size: Au) -> FontBackedMeasurer {
    let provider = Arc::new(SyntheticFontProvider::new().with_size(FONT, size));
    FontBackedMeasurer::new(provider, FONT)
}

#[test]
fn a_longer_run_measures_wider() {
    let size = Au::from_whole_px(16).expect("valid font size");
    let measurer = measurer(size);
    let style = ComputedText::new(size);

    let short = measurer
        .measure(&TextRun::new("hi"), &style)
        .expect("a registered font always measures");
    let long = measurer
        .measure(&TextRun::new("hello world"), &style)
        .expect("a registered font always measures");

    assert!(long.width().raw() > short.width().raw());
}

#[test]
fn an_empty_run_has_zero_width() {
    let size = Au::from_whole_px(16).expect("valid font size");
    let measurer = measurer(size);
    let style = ComputedText::new(size);

    let metrics = measurer
        .measure(&TextRun::new(""), &style)
        .expect("an empty run still measures");
    assert_eq!(metrics.width(), Au::ZERO);
    assert!(
        metrics.height().raw() > 0,
        "line height comes from face metrics alone"
    );
}

#[test]
fn measuring_against_an_unregistered_font_is_a_typed_error() {
    let provider = Arc::new(SyntheticFontProvider::new());
    let measurer = FontBackedMeasurer::new(provider, FontId::new(99));
    let size = Au::from_whole_px(16).expect("valid font size");

    let outcome = measurer.measure(&TextRun::new("x"), &ComputedText::new(size));
    assert!(outcome.is_err());
}

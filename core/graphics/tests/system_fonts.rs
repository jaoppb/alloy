//! [`SystemFontProvider::resolve_named`] — resolving a *named* `font-family`
//! against the OS font index (v0.5 fonts increment).
//!
//! Tolerant of a font-less CI image: a miss is [`GraphicsError::FontUnavailable`],
//! never a panic, and no golden or conformance test touches this path (they all
//! use `SyntheticFontProvider`).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use graphics::{Au, FontId, FontProvider, GlyphId, GraphicsError, SystemFontProvider};

const ID: FontId = FontId::new(1);
const SIZE: Au = Au::from_raw(16 * graphics::AU_PER_PX);

#[test]
fn an_unknown_family_is_font_unavailable_not_a_panic() {
    let outcome = SystemFontProvider::resolve_named("no-machine-has-this-family-zzzqqq", ID, SIZE);
    assert!(
        matches!(outcome, Err(GraphicsError::FontUnavailable { .. })),
        "an unresolvable name must return a typed error, got {outcome:?}"
    );
}

#[test]
fn resolution_is_case_insensitive_when_the_index_has_a_match() {
    // Best-effort: on a font-less image both lookups miss and the test still
    // passes (it only asserts the two spellings agree).
    let lower = SystemFontProvider::resolve_named("dejavu sans", ID, SIZE).is_ok();
    let mixed = SystemFontProvider::resolve_named("DejaVu Sans", ID, SIZE).is_ok();
    assert_eq!(
        lower, mixed,
        "`dejavu sans` and `DejaVu Sans` must resolve the same way"
    );
}

#[test]
fn a_resolved_named_face_maps_a_common_glyph() {
    let Ok(provider) = SystemFontProvider::resolve_named("DejaVu Sans", ID, SIZE) else {
        return; // no DejaVu on this machine — nothing to assert
    };
    let glyph = provider.glyph_for_char(ID, 'A').unwrap();
    assert_ne!(
        glyph,
        GlyphId::NOTDEF,
        "'A' maps to a real glyph in DejaVu Sans"
    );
}

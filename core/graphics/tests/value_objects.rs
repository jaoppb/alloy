//! Unit coverage for the `domain/` value objects — no backend involved.
//!
//! The rules these guard are the two sanitization rules of the v0.3 report §2.3
//! and the fixed-point determinism policy of `ADR-0016`. They are asserted here,
//! on the value objects themselves, because that is where they are enforced —
//! the builder of F4a step 2 only reports what these types already decided.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

use graphics::{
    AU_PER_PX, Au, BackendTier, Color, CommandIndex, CommandKind, CommandRejection, FontId,
    FrameOperation, FrameState, GlyphId, GlyphInstance, GraphicsError, ImageId, Opacity, Point, Px,
    Rect, Size, SurfaceSize,
};

// ---- Au and the one author-input crossing ----

#[test]
fn from_px_refuses_every_non_finite_input_because_none_of_them_has_a_correct_reading() {
    for hostile in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert_eq!(
            Au::from_px(Px::new(hostile)),
            None,
            "a non-finite length must be refused, not substituted: {hostile}"
        );
    }
}

#[test]
fn from_px_clamps_a_finite_length_instead_of_refusing_it() {
    let absurd = Au::from_px(Px::new(f32::MAX)).expect("a finite length is never refused");
    let negative = Au::from_px(Px::new(f32::MIN)).expect("a finite length is never refused");

    assert_eq!(
        absurd,
        Au::MAX_EXTENT,
        "a finite length past the envelope clamps to it"
    );
    assert_eq!(negative, Au::MIN_EXTENT, "and the negative twin clamps too");
}

#[test]
fn a_ten_thousand_pixel_page_is_nowhere_near_the_clamp() {
    let tall = Au::from_px(Px::new(10_000.0)).unwrap();

    assert!(
        tall < Au::MAX_EXTENT,
        "a legitimately tall page must not be clipped by MAX_EXTENT"
    );
    assert_eq!(tall.raw(), 10_000 * AU_PER_PX, "and it converts exactly");
}

#[test]
fn a_sixty_fourth_of_a_pixel_survives_the_round_trip() {
    let sixty_fourth = Au::from_px(Px::new(1.0 / 64.0)).unwrap();

    assert_eq!(sixty_fourth.raw(), 1, "1/64 px is exactly one Au");
    assert_eq!(sixty_fourth.to_px().get(), 1.0 / 64.0);
}

#[test]
fn subnormals_and_negative_zero_are_finite_and_therefore_accepted() {
    for tiny in [f32::MIN_POSITIVE / 2.0, -0.0, 0.0] {
        let converted = Au::from_px(Px::new(tiny));

        assert_eq!(
            converted,
            Some(Au::ZERO),
            "a subnormal or signed zero rounds to zero, it does not fail: {tiny}"
        );
    }
}

#[test]
fn au_arithmetic_is_total_by_construction() {
    let big = Au::from_raw(i32::MAX);

    assert_eq!(
        big.checked_add(Au::from_raw(1)),
        None,
        "checked_add reports the overflow instead of wrapping"
    );
    assert_eq!(
        big.saturating_add(Au::from_raw(1)),
        big,
        "saturating_add stops at the extreme"
    );
    assert_eq!(
        Au::from_whole_px(i32::MAX),
        None,
        "and so does from_whole_px"
    );
}

#[test]
fn smaller_and_larger_order_lengths_without_an_else() {
    let short = Au::from_raw(10);
    let tall = Au::from_raw(20);

    assert_eq!(short.smaller(tall), short);
    assert_eq!(short.larger(tall), tall);
    assert_eq!(tall.smaller(short), short);
}

// ---- Geometry: the refuse rule ----

#[test]
fn a_negative_extent_is_refused_because_it_means_the_caller_computed_wrong() {
    assert_eq!(
        Size::new(Au::from_raw(-1), Au::ZERO),
        None,
        "a negative width is a defect, not something to clamp away"
    );
    assert_eq!(Size::new(Au::ZERO, Au::from_raw(-1)), None);
    assert!(Size::new(Au::ZERO, Au::ZERO).is_some(), "but zero is legal");
}

#[test]
fn a_surface_with_a_zero_dimension_is_refused() {
    assert_eq!(SurfaceSize::new(0, 600), None);
    assert_eq!(SurfaceSize::new(800, 0), None);
    assert_eq!(
        SurfaceSize::new(800, 600).unwrap().pixel_count(),
        Some(480_000)
    );
}

#[test]
fn a_rect_reports_edges_that_saturate_rather_than_wrap() {
    let origin = Point::new(Au::from_raw(i32::MAX), Au::ZERO);
    let size = Size::new(Au::from_raw(100), Au::from_raw(100)).unwrap();
    let rect = Rect::new(origin, size);

    assert_eq!(
        rect.max_x().raw(),
        i32::MAX,
        "an edge past the extreme saturates instead of wrapping into negatives"
    );
}

#[test]
fn intersection_narrows_a_clip_and_reports_disjoint_regions_as_none() {
    let square = |x: i32, y: i32, side: i32| {
        Rect::new(
            Point::new(Au::from_raw(x), Au::from_raw(y)),
            Size::new(Au::from_raw(side), Au::from_raw(side)).unwrap(),
        )
    };
    let overlap = square(0, 0, 10).intersection(square(5, 5, 10)).unwrap();

    assert_eq!(overlap.min_x(), Au::from_raw(5));
    assert_eq!(overlap.size().width(), Au::from_raw(5));
    assert_eq!(
        square(0, 0, 10).intersection(square(50, 50, 10)),
        None,
        "disjoint rectangles have no intersection, not an empty one"
    );
    assert_eq!(
        square(0, 0, 10).intersection(square(10, 0, 10)),
        None,
        "rectangles that merely touch do not overlap"
    );
}

// ---- Colour and opacity ----

#[test]
fn colour_channels_survive_packing_in_memory_order() {
    let colour = Color::rgba(0x12, 0x34, 0x56, 0x78);

    assert_eq!(
        (colour.red(), colour.green(), colour.blue(), colour.alpha()),
        (0x12, 0x34, 0x56, 0x78)
    );
    assert_eq!(colour.packed(), 0x1234_5678);
    assert_eq!(colour.to_rgba8(), [0x12, 0x34, 0x56, 0x78]);
    assert!(Color::rgb(1, 2, 3).is_opaque());
    assert!(Color::TRANSPARENT.is_transparent());
}

#[test]
fn premultiplication_keeps_the_extremes_exact() {
    let opaque = Color::rgba(200, 100, 50, 255).premultiplied();
    let clear = Color::rgba(200, 100, 50, 0).premultiplied();

    assert_eq!(
        (opaque.red(), opaque.green(), opaque.blue()),
        (200, 100, 50),
        "premultiplying by full alpha must be the identity, not a rounding loss"
    );
    assert_eq!(
        (clear.red(), clear.green(), clear.blue()),
        (0, 0, 0),
        "and premultiplying by zero alpha must reach exactly zero"
    );
}

#[test]
fn opacity_refuses_nan_but_clamps_a_finite_value() {
    assert_eq!(
        Opacity::from_unit_interval(f32::NAN),
        None,
        "NaN opacity has no correct reading"
    );
    assert_eq!(Opacity::from_unit_interval(1.5), Some(Opacity::OPAQUE));
    assert_eq!(
        Opacity::from_unit_interval(-3.0),
        Some(Opacity::TRANSPARENT)
    );
    assert_eq!(Opacity::from_unit_interval(1.0), Some(Opacity::OPAQUE));
}

#[test]
fn fading_by_a_transparent_opacity_erases_the_alpha_and_nothing_else() {
    let faded = Color::rgba(10, 20, 30, 255).faded(Opacity::TRANSPARENT);

    assert_eq!(faded.alpha(), 0);
    assert_eq!((faded.red(), faded.green(), faded.blue()), (10, 20, 30));
}

// ---- Handles, tiers and diagnostics ----

#[test]
fn the_cascade_lists_every_tier_in_preference_order() {
    assert_eq!(
        BackendTier::CASCADE,
        [
            BackendTier::Vulkan,
            BackendTier::OpenGl,
            BackendTier::Software
        ]
    );
    assert!(BackendTier::Software.is_always_available());
    assert!(!BackendTier::Vulkan.is_always_available());
}

#[test]
fn the_force_tier_override_parses_exactly_the_three_rung_names() {
    assert_eq!(BackendTier::parse(" Vulkan "), Some(BackendTier::Vulkan));
    assert_eq!(BackendTier::parse("OPENGL"), Some(BackendTier::OpenGl));
    assert_eq!(BackendTier::parse("software"), Some(BackendTier::Software));
    assert_eq!(BackendTier::parse("metal"), None);
}

#[test]
fn a_command_index_saturates_rather_than_losing_the_diagnostic() {
    assert_eq!(CommandIndex::from_position(7).get(), 7);
    assert_eq!(CommandIndex::from_position(usize::MAX).get(), u32::MAX);
    assert_eq!(CommandIndex::FIRST.get(), 0);
}

#[test]
fn glyph_and_resource_handles_keep_their_value_and_print_readably() {
    let instance = GlyphInstance::new(GlyphId::new(42), Point::ORIGIN);

    assert_eq!(instance.glyph().get(), 42);
    assert_eq!(instance.position(), Point::ORIGIN);
    assert_eq!(GlyphId::NOTDEF.get(), 0);
    assert_eq!(FontId::new(3).to_string(), "font #3");
    assert_eq!(ImageId::new(9).to_string(), "image #9");
}

#[test]
fn error_display_names_the_rule_that_fired_and_where() {
    let refused = GraphicsError::InvalidCommand {
        index: CommandIndex::new(4),
        reason: CommandRejection::NonFiniteCoordinate,
    };
    let unsupported = GraphicsError::Unsupported {
        tier: BackendTier::Software,
        command: CommandKind::DrawPath,
    };
    let out_of_order = GraphicsError::FrameOutOfOrder {
        attempted: FrameOperation::Submit,
        state: FrameState::Idle,
    };

    assert_eq!(
        refused.to_string(),
        "display command #4 was refused: a coordinate was NaN or infinite"
    );
    assert_eq!(
        unsupported.to_string(),
        "the software backend does not implement DrawPath"
    );
    assert_eq!(
        out_of_order.to_string(),
        "submit is not valid while the backend is idle"
    );
    assert_eq!(
        GraphicsError::BackendUnavailable {
            tier: BackendTier::Vulkan
        }
        .to_string(),
        "the vulkan backend is unavailable on this system"
    );
}

#[test]
fn graphics_error_implements_std_error() {
    fn assert_is_error<E: std::error::Error>(_: &E) {}

    assert_is_error(&GraphicsError::SurfaceLost);
}

#[test]
fn the_port_schema_version_is_recorded_in_exactly_one_place() {
    assert_eq!(graphics::PORT_SCHEMA_VERSION, 1);
}

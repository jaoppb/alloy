//! Coverage for the parts of the frozen port surface that no v0.3 backend
//! exercises.
//!
//! `RenderBackend` freezes at `F4` (`ADR-0011:121`), and the vocabulary froze
//! whole even though the software rasterizer refuses `DrawImage` and `DrawPath`
//! (v0.3 report §2.3). Untested frozen surface is the worst kind: it is the part
//! a later version will build on, and the part nothing would notice breaking.
//! So the accessors, the names and the `Display` strings are asserted here even
//! where nothing paints with them yet.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

use graphics::{
    AU_PER_PX, Au, BackendTier, Color, CommandIndex, CommandKind, DisplayCommand, FontId,
    FrameState, GlyphId, GlyphInstance, GlyphRun, GraphicsError, ImageId, Opacity, Path,
    PathSegment, Point, Px, Rect, Size, Stroke,
};

const fn rect(x: i32, y: i32, width: i32, height: i32) -> Rect {
    Rect::new(
        Point::new(Au::from_raw(x), Au::from_raw(y)),
        Size::new(Au::from_raw(width), Au::from_raw(height)).expect("non-negative"),
    )
}

// ---- Path and Stroke: declared for the contract, refused by the backend ----

#[test]
fn a_path_is_a_first_class_collection_of_segments() {
    let mut path = Path::new();

    assert!(path.is_empty());
    assert_eq!(path.len(), 0);

    path.push(PathSegment::MoveTo { to: Point::ORIGIN });
    path.push(PathSegment::LineTo {
        to: Point::new(Au::from_raw(64), Au::ZERO),
    });
    path.push(PathSegment::QuadraticTo {
        control: Point::ORIGIN,
        to: Point::ORIGIN,
    });
    path.push(PathSegment::CubicTo {
        first_control: Point::ORIGIN,
        second_control: Point::ORIGIN,
        to: Point::ORIGIN,
    });
    path.push(PathSegment::Close);

    assert_eq!(path.len(), 5);
    assert!(!path.is_empty());
    assert_eq!(path.iter().count(), 5);
    assert_eq!(
        path.iter().next(),
        Some(&PathSegment::MoveTo { to: Point::ORIGIN })
    );
}

#[test]
fn from_segments_and_default_agree_with_new() {
    assert_eq!(Path::from_segments([]), Path::new());
    assert_eq!(Path::default(), Path::new());
}

#[test]
fn a_stroke_keeps_its_width_and_colour() {
    let stroke = Stroke::new(Au::from_raw(128), Color::BLACK);

    assert_eq!(stroke.width(), Au::from_raw(128));
    assert_eq!(stroke.color(), Color::BLACK);
}

// ---- Glyph runs: declared for the contract, filled in F4b ----

#[test]
fn a_glyph_run_is_a_first_class_collection() {
    let mut run = GlyphRun::new();

    assert!(run.is_empty());
    assert_eq!(run.len(), 0);

    run.push(GlyphInstance::new(GlyphId::new(1), Point::ORIGIN));
    run.push(GlyphInstance::new(
        GlyphId::new(2),
        Point::new(Au::from_raw(64), Au::ZERO),
    ));

    assert_eq!(run.len(), 2);
    assert_eq!(run.iter().count(), 2);
    assert_eq!(
        (&run).into_iter().count(),
        2,
        "borrowed iteration works too"
    );
    assert_eq!(GlyphRun::default(), GlyphRun::new());
    assert_eq!(
        GlyphRun::from_glyphs(run.iter().copied()),
        run,
        "collecting a run's own glyphs reproduces it"
    );
}

#[test]
fn a_glyph_instance_prints_its_glyph_and_pen_position() {
    let instance = GlyphInstance::new(GlyphId::new(7), Point::new(Au::from_raw(64), Au::ZERO));

    assert_eq!(instance.to_string(), "glyph #7 at (64au, 0au)");
    assert_eq!(GlyphId::new(3).to_string(), "glyph #3");
}

// ---- Handles ----

#[test]
fn resource_handles_round_trip_their_value() {
    assert_eq!(FontId::new(u16::MAX).get(), u16::MAX);
    assert_eq!(ImageId::new(u32::MAX).get(), u32::MAX);
    assert_eq!(GlyphId::new(0), GlyphId::NOTDEF);
}

// ---- Command names and scope classification ----

#[test]
fn every_command_kind_names_itself() {
    let expected = [
        (CommandKind::DrawRect, "DrawRect"),
        (CommandKind::DrawText, "DrawText"),
        (CommandKind::DrawImage, "DrawImage"),
        (CommandKind::DrawPath, "DrawPath"),
        (CommandKind::PushClip, "PushClip"),
        (CommandKind::PopClip, "PopClip"),
        (CommandKind::PushOpacity, "PushOpacity"),
        (CommandKind::PopOpacity, "PopOpacity"),
    ];

    for (kind, name) in expected {
        assert_eq!(kind.name(), name);
        assert_eq!(kind.to_string(), name);
    }
}

#[test]
fn only_the_push_commands_open_a_scope() {
    let opens = DisplayCommand::PushClip {
        region: rect(0, 0, 64, 64),
    };
    let closes = DisplayCommand::PopClip;
    let paints = DisplayCommand::DrawRect {
        rect: rect(0, 0, 64, 64),
        color: Color::BLACK,
        corner_radius: Au::ZERO,
    };

    assert!(opens.opens_scope());
    assert!(!closes.opens_scope());
    assert!(!paints.opens_scope());
    assert!(
        DisplayCommand::PushOpacity {
            opacity: Opacity::OPAQUE
        }
        .opens_scope()
    );
    assert!(!DisplayCommand::PopOpacity.opens_scope());
}

#[test]
fn a_display_command_reports_its_own_kind() {
    let image = DisplayCommand::DrawImage {
        image: ImageId::new(0),
        source: rect(0, 0, 64, 64),
        destination: rect(0, 0, 64, 64),
    };
    let path = DisplayCommand::DrawPath {
        path: Path::new(),
        fill: None,
        stroke: None,
    };
    let text = DisplayCommand::DrawText {
        glyphs: GlyphRun::new(),
        color: Color::BLACK,
        font: FontId::new(0),
    };

    assert_eq!(image.kind(), CommandKind::DrawImage);
    assert_eq!(path.kind(), CommandKind::DrawPath);
    assert_eq!(text.kind(), CommandKind::DrawText);
}

// ---- Geometry accessors and edge behaviour ----

#[test]
fn a_point_translates_and_prints() {
    let moved = Point::ORIGIN.translated(Au::from_raw(10), Au::from_raw(-5));

    assert_eq!(moved.horizontal(), Au::from_raw(10));
    assert_eq!(moved.vertical(), Au::from_raw(-5));
    assert_eq!(moved.to_string(), "(10au, -5au)");
    assert_eq!(Point::default(), Point::ORIGIN);
}

#[test]
fn translation_saturates_at_the_extremes_instead_of_wrapping() {
    let far = Point::new(Au::from_raw(i32::MAX), Au::from_raw(i32::MIN));

    let moved = far.translated(Au::from_raw(1000), Au::from_raw(-1000));

    assert_eq!(moved.horizontal().raw(), i32::MAX);
    assert_eq!(moved.vertical().raw(), i32::MIN);
}

#[test]
fn a_size_reports_emptiness_on_either_axis_and_prints() {
    let flat = Size::new(Au::from_raw(64), Au::ZERO).unwrap();
    let thin = Size::new(Au::ZERO, Au::from_raw(64)).unwrap();
    let real = Size::new(Au::from_raw(64), Au::from_raw(32)).unwrap();

    assert!(flat.is_empty());
    assert!(thin.is_empty());
    assert!(!real.is_empty());
    assert_eq!(Size::EMPTY, Size::default());
    assert_eq!(real.to_string(), "64au × 32au");
    assert_eq!(
        (real.width(), real.height()),
        (Au::from_raw(64), Au::from_raw(32))
    );
}

#[test]
fn a_rect_exposes_its_origin_extent_and_edges_and_prints() {
    let area = rect(10, 20, 30, 40);

    assert_eq!(
        area.origin(),
        Point::new(Au::from_raw(10), Au::from_raw(20))
    );
    assert_eq!(area.size().width(), Au::from_raw(30));
    assert_eq!(area.min_x(), Au::from_raw(10));
    assert_eq!(area.min_y(), Au::from_raw(20));
    assert_eq!(area.max_x(), Au::from_raw(40));
    assert_eq!(area.max_y(), Au::from_raw(60));
    assert!(!area.is_empty());
    assert!(rect(0, 0, 0, 10).is_empty());
    assert_eq!(area.to_string(), "30au × 40au at (10au, 20au)");
    assert_eq!(Rect::default(), rect(0, 0, 0, 0));
}

#[test]
fn a_rect_fully_inside_another_intersects_to_itself() {
    let outer = rect(0, 0, 640, 640);
    let inner = rect(64, 64, 64, 64);

    assert_eq!(inner.intersection(outer), Some(inner));
    assert_eq!(outer.intersection(inner), Some(inner));
}

// ---- Units ----

#[test]
fn au_reports_its_sign_and_zero_and_prints() {
    assert!(Au::from_raw(-1).is_negative());
    assert!(!Au::ZERO.is_negative());
    assert!(Au::ZERO.is_zero());
    assert!(!Au::from_raw(1).is_zero());
    assert_eq!(Au::from_raw(-5).to_string(), "-5au");
    assert_eq!(Au::default(), Au::ZERO);
    assert_eq!(AU_PER_PX, 64);
}

#[test]
fn checked_subtraction_reports_underflow() {
    assert_eq!(Au::from_raw(i32::MIN).checked_sub(Au::from_raw(1)), None);
    assert_eq!(
        Au::from_raw(i32::MIN).saturating_sub(Au::from_raw(1)),
        Au::from_raw(i32::MIN)
    );
    assert_eq!(
        Au::from_raw(10).checked_sub(Au::from_raw(4)),
        Some(Au::from_raw(6))
    );
}

#[test]
fn a_px_keeps_its_value_verbatim_including_the_hostile_ones() {
    assert_eq!(Px::new(1.5).get(), 1.5);
    assert!(Px::new(f32::NAN).get().is_nan());
    assert_eq!(Px::default().get(), 0.0);
    assert_eq!(Px::new(2.5).to_string(), "2.5px");
}

// ---- Colour and opacity accessors ----

#[test]
fn colour_prints_as_a_packed_hex_word_and_exposes_its_constants() {
    assert_eq!(Color::BLACK.to_string(), "#000000ff");
    assert_eq!(Color::WHITE.to_string(), "#ffffffff");
    assert_eq!(Color::TRANSPARENT.to_string(), "#00000000");
    assert_eq!(Color::default(), Color::TRANSPARENT);
    assert_eq!(Color::BLACK.with_alpha(0), Color::TRANSPARENT);
}

#[test]
fn opacity_exposes_its_level_and_extremes() {
    assert_eq!(Opacity::OPAQUE.level(), 255);
    assert_eq!(Opacity::TRANSPARENT.level(), 0);
    assert!(Opacity::OPAQUE.is_opaque());
    assert!(Opacity::TRANSPARENT.is_transparent());
    assert!(!Opacity::from_level(128).is_opaque());
    assert!(!Opacity::from_level(128).is_transparent());
    assert_eq!(Opacity::default(), Opacity::OPAQUE);
    assert_eq!(Opacity::from_level(200).to_string(), "200/255");
    assert!(Opacity::OPAQUE > Opacity::TRANSPARENT, "opacities order");
}

// ---- Error surface ----

#[test]
fn every_error_variant_prints_something_a_reader_can_act_on() {
    let cases = [
        (
            GraphicsError::SurfaceLost,
            "the render surface was lost during the frame",
        ),
        (
            GraphicsError::ReadbackFailed {
                tier: BackendTier::OpenGl,
            },
            "could not read the frame back from the opengl backend",
        ),
        (
            GraphicsError::BackendUnavailable {
                tier: BackendTier::Software,
            },
            "the software backend is unavailable on this system",
        ),
    ];

    for (error, message) in cases {
        assert_eq!(error.to_string(), message);
    }
}

#[test]
fn frame_state_names_each_stage_of_the_lifecycle() {
    assert_eq!(FrameState::Idle.name(), "idle");
    assert_eq!(FrameState::Recording.name(), "recording");
    assert_eq!(FrameState::Presented.name(), "presented");
    assert_eq!(FrameState::Recording.to_string(), "recording");
    assert_eq!(FrameState::default(), FrameState::Idle);
}

#[test]
fn a_command_index_prints_the_way_a_diagnostic_reads() {
    assert_eq!(CommandIndex::new(12).to_string(), "#12");
    assert_eq!(CommandIndex::default(), CommandIndex::FIRST);
}

#[test]
fn the_tier_names_match_what_the_force_variable_accepts() {
    for tier in BackendTier::CASCADE {
        assert_eq!(
            BackendTier::parse(tier.name()),
            Some(tier),
            "a tier's own name must round-trip through parse"
        );
        assert_eq!(tier.to_string(), tier.name());
    }
    assert!(BackendTier::Vulkan.rank() < BackendTier::OpenGl.rank());
    assert!(BackendTier::OpenGl.rank() < BackendTier::Software.rank());
}

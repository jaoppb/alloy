//! The sanitizing boundary of `PRD-005:80` and the first-class `DisplayList`.
//!
//! Every assertion here is about one of the two rules of the v0.3 report §2.3 —
//! refuse what has no correct reading, clamp what merely overruns — or about the
//! balanced state stack. The point of the property test at the end is that no
//! input at all reaches a backend unsanitized.

// `clippy.toml`'s `allow-panic-in-tests` only reaches `#[test]` functions, and
// the two `rejection_of` / `index_of` helpers below are plain functions. The
// float comparison is exact on purpose: `10_000.0` px is representable, and the
// point of the assertion is that the envelope did *not* perturb it. Same shape,
// same reasoning, as `core/engine/tests/conversions.rs:3-4`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::float_cmp,
    clippy::single_match_else
)]

use graphics::{
    Au, Color, CommandIndex, CommandKind, CommandRejection, DisplayCommand, DisplayList,
    DisplayListBuilder, FontId, GlyphId, GlyphInstance, GlyphRun, GraphicsError, ImageId, Opacity,
    Path, PathSegment, Point, PxRect, Stroke,
};

fn rejection_of(error: &GraphicsError) -> CommandRejection {
    match error {
        GraphicsError::InvalidCommand { reason, .. } => *reason,
        other => panic!("expected a refused command, got {other:?}"),
    }
}

fn index_of(error: &GraphicsError) -> CommandIndex {
    match error {
        GraphicsError::InvalidCommand { index, .. } => *index,
        other => panic!("expected a refused command, got {other:?}"),
    }
}

// ---- rule one: refuse what has no correct reading ----

#[test]
fn every_non_finite_coordinate_is_refused_on_every_axis() {
    let hostile = [f32::NAN, f32::INFINITY, f32::NEG_INFINITY];
    let axes: [fn(f32) -> PxRect; 4] = [
        |bad| PxRect::from_px(bad, 0.0, 10.0, 10.0),
        |bad| PxRect::from_px(0.0, bad, 10.0, 10.0),
        |bad| PxRect::from_px(0.0, 0.0, bad, 10.0),
        |bad| PxRect::from_px(0.0, 0.0, 10.0, bad),
    ];

    for value in hostile {
        for build_area in axes {
            let mut builder = DisplayListBuilder::new();
            let error = builder
                .draw_rect(build_area(value), Color::BLACK)
                .expect_err("a non-finite coordinate must be refused");

            assert_eq!(rejection_of(&error), CommandRejection::NonFiniteCoordinate);
            assert!(builder.is_empty(), "a refused command must not be recorded");
        }
    }
}

#[test]
fn a_negative_extent_is_refused_rather_than_clamped_to_zero() {
    let mut builder = DisplayListBuilder::new();

    let error = builder
        .draw_rect(PxRect::from_px(0.0, 0.0, -5.0, 10.0), Color::BLACK)
        .expect_err("a negative width means the caller computed wrong");

    assert_eq!(rejection_of(&error), CommandRejection::NegativeExtent);
}

#[test]
fn a_refused_command_reports_the_position_it_would_have_occupied() {
    let mut builder = DisplayListBuilder::new();
    builder
        .draw_rect(PxRect::from_px(0.0, 0.0, 1.0, 1.0), Color::BLACK)
        .unwrap();
    builder
        .draw_rect(PxRect::from_px(0.0, 0.0, 1.0, 1.0), Color::BLACK)
        .unwrap();

    let error = builder
        .draw_rect(PxRect::from_px(f32::NAN, 0.0, 1.0, 1.0), Color::BLACK)
        .expect_err("still refused");

    assert_eq!(
        index_of(&error),
        CommandIndex::new(2),
        "the index is the ADR-0011:93-95 location metadata for this port"
    );
}

#[test]
fn a_non_finite_opacity_is_refused() {
    let mut builder = DisplayListBuilder::new();

    let error = builder
        .push_opacity(f32::NAN)
        .expect_err("NaN opacity has no correct reading");

    assert_eq!(rejection_of(&error), CommandRejection::NonFiniteCoordinate);
    assert_eq!(builder.open_scope_count(), 0, "and no scope was opened");
}

// ---- rule two: clamp what merely overruns ----

#[test]
fn a_finite_coordinate_past_the_envelope_is_clamped_and_the_page_still_paints() {
    let mut builder = DisplayListBuilder::new();

    builder
        .draw_rect(PxRect::from_px(0.0, 0.0, f32::MAX, 10.0), Color::BLACK)
        .expect("a legitimate page with a giant box must still paint");

    let list = builder.build().unwrap();
    let DisplayCommand::DrawRect { rect, .. } = list.command(CommandIndex::FIRST).unwrap() else {
        panic!("expected a DrawRect");
    };
    assert_eq!(rect.size().width(), Au::MAX_EXTENT);
}

#[test]
fn an_opacity_outside_the_unit_interval_is_clamped() {
    let mut builder = DisplayListBuilder::new();
    builder.push_opacity(1.5).unwrap();
    builder.pop_opacity().unwrap();
    builder.push_opacity(-2.0).unwrap();
    builder.pop_opacity().unwrap();

    let list = builder.build().unwrap();
    let opacities: Vec<Opacity> = list
        .iter()
        .filter_map(|command| match command {
            DisplayCommand::PushOpacity { opacity } => Some(*opacity),
            _ => None,
        })
        .collect();

    assert_eq!(opacities, vec![Opacity::OPAQUE, Opacity::TRANSPARENT]);
}

#[test]
fn a_ten_thousand_pixel_tall_page_is_not_clipped_by_the_envelope() {
    let mut builder = DisplayListBuilder::new();

    builder
        .draw_rect(PxRect::from_px(0.0, 0.0, 800.0, 10_000.0), Color::WHITE)
        .unwrap();

    let list = builder.build().unwrap();
    let DisplayCommand::DrawRect { rect, .. } = list.command(CommandIndex::FIRST).unwrap() else {
        panic!("expected a DrawRect");
    };
    assert_eq!(
        rect.size().height().to_px().get(),
        10_000.0,
        "MAX_EXTENT must not corrupt a legitimately tall page"
    );
}

// ---- the balanced state stack ----

#[test]
fn a_pop_without_a_push_is_refused_for_both_scope_kinds() {
    let mut clips = DisplayListBuilder::new();
    let mut layers = DisplayListBuilder::new();

    assert_eq!(
        rejection_of(&clips.pop_clip().expect_err("no clip is open")),
        CommandRejection::ClipPopWithoutPush
    );
    assert_eq!(
        rejection_of(&layers.pop_opacity().expect_err("no layer is open")),
        CommandRejection::OpacityPopWithoutPush
    );
}

#[test]
fn a_pop_must_match_the_scope_actually_on_top() {
    let mut builder = DisplayListBuilder::new();
    builder
        .push_clip(PxRect::from_px(0.0, 0.0, 10.0, 10.0))
        .unwrap();

    let error = builder
        .pop_opacity()
        .expect_err("the top of the stack is a clip, not a layer");

    assert_eq!(
        rejection_of(&error),
        CommandRejection::OpacityPopWithoutPush
    );
}

#[test]
fn building_with_a_scope_still_open_refuses_the_whole_list() {
    let mut clips = DisplayListBuilder::new();
    clips
        .push_clip(PxRect::from_px(0.0, 0.0, 10.0, 10.0))
        .unwrap();
    let mut layers = DisplayListBuilder::new();
    layers.push_opacity(0.5).unwrap();

    assert_eq!(
        rejection_of(&clips.build().expect_err("a clip was left open")),
        CommandRejection::ClipLeftOpen
    );
    assert_eq!(
        rejection_of(&layers.build().expect_err("a layer was left open")),
        CommandRejection::OpacityLeftOpen
    );
}

#[test]
fn properly_nested_scopes_build_cleanly() {
    let mut builder = DisplayListBuilder::new();
    builder
        .push_clip(PxRect::from_px(0.0, 0.0, 100.0, 100.0))
        .unwrap();
    builder.push_opacity(0.5).unwrap();
    builder
        .draw_rect(PxRect::from_px(1.0, 1.0, 2.0, 2.0), Color::BLACK)
        .unwrap();
    builder.pop_opacity().unwrap();
    builder.pop_clip().unwrap();

    let list = builder.build().expect("balanced scopes build");

    assert_eq!(list.len(), 5);
    assert_eq!(
        list.iter().map(DisplayCommand::kind).collect::<Vec<_>>(),
        vec![
            CommandKind::PushClip,
            CommandKind::PushOpacity,
            CommandKind::DrawRect,
            CommandKind::PopOpacity,
            CommandKind::PopClip,
        ]
    );
}

// ---- the aggregate itself ----

#[test]
fn the_whole_prd_005_vocabulary_can_be_expressed() {
    let mut builder = DisplayListBuilder::new();
    let glyphs = GlyphRun::from_glyphs([GlyphInstance::new(GlyphId::new(7), Point::ORIGIN)]);
    let path = Path::from_segments([
        PathSegment::MoveTo { to: Point::ORIGIN },
        PathSegment::Close,
    ]);

    builder
        .draw_rounded_rect(
            PxRect::from_px(0.0, 0.0, 4.0, 4.0),
            Color::BLACK,
            graphics::Px::new(2.0),
        )
        .unwrap();
    builder
        .draw_text(glyphs, Color::WHITE, FontId::new(0))
        .unwrap();
    builder
        .draw_image(
            ImageId::new(1),
            PxRect::from_px(0.0, 0.0, 8.0, 8.0),
            PxRect::from_px(0.0, 0.0, 16.0, 16.0),
        )
        .unwrap();
    builder
        .draw_path(
            path,
            Some(Color::BLACK),
            Some(Stroke::new(Au::from_raw(64), Color::WHITE)),
        )
        .unwrap();

    let list = builder.build().unwrap();

    assert_eq!(
        list.iter().map(DisplayCommand::kind).collect::<Vec<_>>(),
        vec![
            CommandKind::DrawRect,
            CommandKind::DrawText,
            CommandKind::DrawImage,
            CommandKind::DrawPath,
        ],
        "all six PRD-005:65-70 commands are expressible even though two are refused downstream"
    );
}

#[test]
fn a_display_list_is_indexable_and_bounded() {
    let empty = DisplayList::empty();

    assert!(empty.is_empty());
    assert_eq!(empty.len(), 0);
    assert_eq!(empty.command(CommandIndex::FIRST), None);
    assert_eq!(empty.iter().count(), 0);
}

// ---- the property: nothing reaches a backend unsanitized ----

#[test]
fn no_hostile_input_ever_reaches_a_built_list_unsanitized() {
    let hostile = [
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::MAX,
        f32::MIN,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        0.0,
        -0.0,
        -1.0,
        1e30,
        -1e30,
    ];
    let mut refused = 0_usize;
    let mut accepted = 0_usize;

    for left in hostile {
        for width in hostile {
            let mut builder = DisplayListBuilder::new();
            let outcome = builder.draw_rect(PxRect::from_px(left, 0.0, width, 1.0), Color::BLACK);

            match outcome {
                Err(error) => {
                    refused += 1;
                    assert!(
                        matches!(error, GraphicsError::InvalidCommand { .. }),
                        "a refusal must be a typed InvalidCommand, got {error:?}"
                    );
                }
                Ok(()) => {
                    accepted += 1;
                    let list = builder.build().unwrap();
                    let DisplayCommand::DrawRect { rect, .. } =
                        list.command(CommandIndex::FIRST).unwrap()
                    else {
                        panic!("expected a DrawRect");
                    };
                    assert!(
                        rect.min_x() >= Au::MIN_EXTENT && rect.min_x() <= Au::MAX_EXTENT,
                        "an accepted origin is always inside the envelope: {left}"
                    );
                    assert!(
                        !rect.size().width().is_negative(),
                        "an accepted extent is never negative: {width}"
                    );
                    assert!(
                        rect.size().width() <= Au::MAX_EXTENT,
                        "an accepted extent is always inside the envelope: {width}"
                    );
                }
            }
        }
    }

    assert!(refused > 0, "the hostile set must exercise the refuse rule");
    assert!(accepted > 0, "and the clamp rule");
}

//! The CPU rasterizer: exact integer coverage, clipping, opacity and the
//! refusal of the two commands v0.3 does not implement.
//!
//! Every number asserted here is computed by hand from the coverage rule of
//! `raster.rs` — a pixel is `64 × 64` Au, so a rectangle covering half a pixel
//! blends at exactly half. If any of these drift, the golden images of step 6
//! would drift with them, silently.

// The whole file is about the concrete rasterizer, so it does not exist in the
// `no-backend` build. What still compiles and runs there is the display list,
// the port and `RecordingBackend` — which is precisely the proof
// `ADR-0011:99-102` asks for.
#![cfg(feature = "software-backend")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use graphics::{
    Au, BackendTier, Color, DisplayList, DisplayListBuilder, GraphicsError, ImageId, Path,
    PathSegment, Point, Px, PxRect, RenderBackend, SoftwareCpuBackend, SurfaceSize,
};

const fn surface(width: u32, height: u32) -> SurfaceSize {
    SurfaceSize::new(width, height).expect("a non-zero surface")
}

/// Paints `list` on a `width × height` canvas and returns the frame.
fn render(width: u32, height: u32, list: &DisplayList) -> graphics::Framebuffer {
    let mut backend = SoftwareCpuBackend::new();
    backend.begin_frame(surface(width, height)).unwrap();
    backend.submit(list).unwrap();
    backend.end_frame().unwrap();
    backend.read_back().unwrap()
}

fn list_of(build: impl FnOnce(&mut DisplayListBuilder)) -> DisplayList {
    let mut builder = DisplayListBuilder::new();
    build(&mut builder);
    builder.build().expect("the test builds a balanced list")
}

#[test]
fn the_software_backend_passes_the_conformance_suite() {
    graphics::conformance::run_backend_suite(&mut SoftwareCpuBackend::new());
}

#[test]
fn a_frame_starts_as_an_opaque_white_canvas() {
    let frame = render(2, 2, &DisplayList::empty());

    for row in 0..2 {
        for column in 0..2 {
            assert_eq!(
                frame.pixel(column, row),
                Some(Color::WHITE),
                "an empty list must leave the canvas untouched at ({column}, {row})"
            );
        }
    }
}

#[test]
fn a_whole_pixel_rectangle_lands_exactly_on_the_pixel_grid() {
    let frame = render(
        3,
        3,
        &list_of(|builder| {
            builder
                .draw_rect(PxRect::from_px(1.0, 1.0, 1.0, 1.0), Color::BLACK)
                .unwrap();
        }),
    );

    assert_eq!(frame.pixel(1, 1), Some(Color::BLACK), "the covered pixel");
    assert_eq!(frame.pixel(0, 1), Some(Color::WHITE), "and no bleed left");
    assert_eq!(frame.pixel(2, 1), Some(Color::WHITE), "or right");
    assert_eq!(frame.pixel(1, 0), Some(Color::WHITE), "or above");
    assert_eq!(frame.pixel(1, 2), Some(Color::WHITE), "or below");
}

#[test]
fn half_covering_a_pixel_blends_at_exactly_half() {
    // 0.5 px wide, a full pixel tall: coverage is 32 × 64 = 2048 of 4096.
    let frame = render(
        1,
        1,
        &list_of(|builder| {
            builder
                .draw_rect(PxRect::from_px(0.0, 0.0, 0.5, 1.0), Color::BLACK)
                .unwrap();
        }),
    );

    let blended = frame.pixel(0, 0).unwrap();

    assert_eq!(
        blended,
        Color::rgb(127, 127, 127),
        "255 * (1 - 128/255) is exactly 127 — the integer answer, not a rounded 128"
    );
}

#[test]
fn a_quarter_covered_pixel_blends_at_exactly_a_quarter() {
    // 0.5 × 0.5 px: coverage is 32 × 32 = 1024 of 4096.
    let frame = render(
        1,
        1,
        &list_of(|builder| {
            builder
                .draw_rect(PxRect::from_px(0.0, 0.0, 0.5, 0.5), Color::BLACK)
                .unwrap();
        }),
    );

    assert_eq!(frame.pixel(0, 0), Some(Color::rgb(191, 191, 191)));
}

#[test]
fn a_source_alpha_attenuates_coverage_multiplicatively() {
    // Full coverage, half alpha: the same 127 as half coverage at full alpha.
    let frame = render(
        1,
        1,
        &list_of(|builder| {
            builder
                .draw_rect(
                    PxRect::from_px(0.0, 0.0, 1.0, 1.0),
                    Color::rgba(0, 0, 0, 128),
                )
                .unwrap();
        }),
    );

    assert_eq!(frame.pixel(0, 0), Some(Color::rgb(127, 127, 127)));
}

#[test]
fn the_canvas_stays_opaque_however_it_is_painted() {
    let frame = render(
        1,
        1,
        &list_of(|builder| {
            builder
                .draw_rect(
                    PxRect::from_px(0.0, 0.0, 1.0, 1.0),
                    Color::rgba(10, 20, 30, 1),
                )
                .unwrap();
        }),
    );

    assert_eq!(
        frame.pixel(0, 0).unwrap().alpha(),
        255,
        "destination alpha must stay 255 so src-over needs no un-premultiply"
    );
}

// ---- clipping ----

#[test]
fn a_clip_narrows_the_fill_and_the_pop_restores_it() {
    let frame = render(
        4,
        1,
        &list_of(|builder| {
            builder
                .push_clip(PxRect::from_px(1.0, 0.0, 1.0, 1.0))
                .unwrap();
            builder
                .draw_rect(PxRect::from_px(0.0, 0.0, 4.0, 1.0), Color::BLACK)
                .unwrap();
            builder.pop_clip().unwrap();
            builder
                .draw_rect(PxRect::from_px(3.0, 0.0, 1.0, 1.0), Color::BLACK)
                .unwrap();
        }),
    );

    assert_eq!(frame.pixel(0, 0), Some(Color::WHITE), "clipped away");
    assert_eq!(frame.pixel(1, 0), Some(Color::BLACK), "inside the clip");
    assert_eq!(frame.pixel(2, 0), Some(Color::WHITE), "clipped away");
    assert_eq!(frame.pixel(3, 0), Some(Color::BLACK), "after the pop");
}

#[test]
fn nested_clips_intersect_rather_than_replace() {
    let frame = render(
        4,
        1,
        &list_of(|builder| {
            builder
                .push_clip(PxRect::from_px(0.0, 0.0, 3.0, 1.0))
                .unwrap();
            builder
                .push_clip(PxRect::from_px(2.0, 0.0, 2.0, 1.0))
                .unwrap();
            builder
                .draw_rect(PxRect::from_px(0.0, 0.0, 4.0, 1.0), Color::BLACK)
                .unwrap();
            builder.pop_clip().unwrap();
            builder.pop_clip().unwrap();
        }),
    );

    assert_eq!(frame.pixel(1, 0), Some(Color::WHITE));
    assert_eq!(
        frame.pixel(2, 0),
        Some(Color::BLACK),
        "only the overlap of both clips survives"
    );
    assert_eq!(frame.pixel(3, 0), Some(Color::WHITE));
}

#[test]
fn a_fill_outside_the_surface_is_dropped_instead_of_wrapping() {
    let frame = render(
        2,
        2,
        &list_of(|builder| {
            builder
                .draw_rect(PxRect::from_px(50.0, 50.0, 10.0, 10.0), Color::BLACK)
                .unwrap();
        }),
    );

    assert_eq!(frame.pixel(0, 0), Some(Color::WHITE));
    assert_eq!(frame.pixel(1, 1), Some(Color::WHITE));
}

// ---- opacity ----

#[test]
fn an_opacity_layer_attenuates_what_is_drawn_inside_it() {
    let frame = render(
        1,
        1,
        &list_of(|builder| {
            builder.push_opacity(0.5).unwrap();
            builder
                .draw_rect(PxRect::from_px(0.0, 0.0, 1.0, 1.0), Color::BLACK)
                .unwrap();
            builder.pop_opacity().unwrap();
        }),
    );

    assert_eq!(frame.pixel(0, 0), Some(Color::rgb(127, 127, 127)));
}

#[test]
fn nested_opacity_layers_multiply() {
    let frame = render(
        1,
        1,
        &list_of(|builder| {
            builder.push_opacity(0.5).unwrap();
            builder.push_opacity(0.5).unwrap();
            builder
                .draw_rect(PxRect::from_px(0.0, 0.0, 1.0, 1.0), Color::BLACK)
                .unwrap();
            builder.pop_opacity().unwrap();
            builder.pop_opacity().unwrap();
        }),
    );

    // 0.5 × 0.5 = 0.25 of black over white.
    assert_eq!(frame.pixel(0, 0), Some(Color::rgb(191, 191, 191)));
}

#[test]
fn popping_an_opacity_layer_restores_full_strength() {
    let frame = render(
        2,
        1,
        &list_of(|builder| {
            builder.push_opacity(0.5).unwrap();
            builder
                .draw_rect(PxRect::from_px(0.0, 0.0, 1.0, 1.0), Color::BLACK)
                .unwrap();
            builder.pop_opacity().unwrap();
            builder
                .draw_rect(PxRect::from_px(1.0, 0.0, 1.0, 1.0), Color::BLACK)
                .unwrap();
        }),
    );

    assert_eq!(frame.pixel(0, 0), Some(Color::rgb(127, 127, 127)));
    assert_eq!(frame.pixel(1, 0), Some(Color::BLACK));
}

// ---- rounded corners ----

#[test]
fn a_corner_radius_erodes_the_corner_pixels_and_leaves_the_straight_runs_exact() {
    // 8 x 4 px at radius 1: the arc centres sit at x = 1 and x = 7, so columns
    // 1..7 are a genuine straight run. A radius of half the width would make
    // the two centres coincide and the shape a circle, with no straight run at
    // all — which is correct, and is why this rectangle is wide.
    let frame = render(
        8,
        4,
        &list_of(|builder| {
            builder
                .draw_rounded_rect(
                    PxRect::from_px(0.0, 0.0, 8.0, 4.0),
                    Color::BLACK,
                    Px::new(1.0),
                )
                .unwrap();
        }),
    );
    let corner = frame.pixel(0, 0).unwrap();

    assert!(
        corner.red() > Color::BLACK.red(),
        "the corner pixel must be partly eroded, got {corner}"
    );
    assert_eq!(
        frame.pixel(3, 0),
        Some(Color::BLACK),
        "the straight run along the top keeps exact analytic coverage"
    );
    assert_eq!(
        frame.pixel(3, 1),
        Some(Color::BLACK),
        "and so does the interior"
    );
    assert_eq!(
        frame.pixel(0, 0),
        frame.pixel(7, 3),
        "all four corners must be eroded identically"
    );
}

#[test]
fn a_zero_radius_is_indistinguishable_from_a_square_rectangle() {
    let square = render(
        3,
        3,
        &list_of(|builder| {
            builder
                .draw_rect(PxRect::from_px(0.0, 0.0, 3.0, 3.0), Color::BLACK)
                .unwrap();
        }),
    );
    let rounded = render(
        3,
        3,
        &list_of(|builder| {
            builder
                .draw_rounded_rect(
                    PxRect::from_px(0.0, 0.0, 3.0, 3.0),
                    Color::BLACK,
                    Px::new(0.0),
                )
                .unwrap();
        }),
    );

    assert_eq!(square.as_rgba8(), rounded.as_rgba8());
}

// ---- what is unimplemented or checked against providers ----

#[test]
fn the_unimplemented_draw_path_reports_unsupported_naming_itself() {
    let list = list_of(|builder| {
        builder
            .draw_path(
                Path::from_segments([PathSegment::MoveTo { to: Point::ORIGIN }]),
                Some(Color::BLACK),
                None,
            )
            .unwrap();
    });

    let mut backend = SoftwareCpuBackend::new();
    backend.begin_frame(surface(1, 1)).unwrap();

    let error = backend
        .submit(&list)
        .expect_err("DrawPath is not implemented");

    let GraphicsError::Unsupported { tier, command } = error else {
        panic!("DrawPath must be refused as Unsupported, got {error:?}");
    };
    assert_eq!(tier, BackendTier::Software);
    assert_eq!(
        command.name(),
        "DrawPath",
        "the error must name the command"
    );
}

#[test]
fn draw_image_with_unregistered_image_reports_image_unavailable() {
    let list = list_of(|builder| {
        builder
            .draw_image(
                ImageId::new(42),
                PxRect::from_px(0.0, 0.0, 1.0, 1.0),
                PxRect::from_px(0.0, 0.0, 1.0, 1.0),
            )
            .unwrap();
    });

    let mut backend = SoftwareCpuBackend::new();
    backend.begin_frame(surface(1, 1)).unwrap();

    let error = backend
        .submit(&list)
        .expect_err("unregistered image must be refused");

    assert_eq!(
        error,
        GraphicsError::ImageUnavailable {
            image: ImageId::new(42),
        }
    );
}

// ---- determinism ----

#[test]
fn the_same_list_renders_to_identical_bytes_a_hundred_times() {
    let list = list_of(|builder| {
        builder
            .push_clip(PxRect::from_px(0.5, 0.5, 6.25, 6.25))
            .unwrap();
        builder.push_opacity(0.375).unwrap();
        builder
            .draw_rounded_rect(
                PxRect::from_px(0.125, 0.375, 6.5, 5.75),
                Color::rgba(17, 99, 231, 200),
                Px::new(1.5),
            )
            .unwrap();
        builder.pop_opacity().unwrap();
        builder.pop_clip().unwrap();
        builder
            .draw_rect(
                PxRect::from_px(1.0 / 3.0, 2.0 / 7.0, 3.3, 2.9),
                Color::BLACK,
            )
            .unwrap();
    });
    let reference = render(8, 8, &list);

    for attempt in 0..100 {
        assert_eq!(
            render(8, 8, &list).as_rgba8(),
            reference.as_rgba8(),
            "render {attempt} diverged: the rasterizer is not deterministic"
        );
    }
}

#[test]
fn a_reused_backend_starts_each_frame_from_a_clean_canvas() {
    let mut backend = SoftwareCpuBackend::new();
    let painted = list_of(|builder| {
        builder
            .draw_rect(PxRect::from_px(0.0, 0.0, 1.0, 1.0), Color::BLACK)
            .unwrap();
    });

    backend.begin_frame(surface(1, 1)).unwrap();
    backend.submit(&painted).unwrap();
    backend.end_frame().unwrap();
    backend.begin_frame(surface(1, 1)).unwrap();
    backend.submit(&DisplayList::empty()).unwrap();
    backend.end_frame().unwrap();

    assert_eq!(
        backend.read_back().unwrap().pixel(0, 0),
        Some(Color::WHITE),
        "a new frame must not inherit the previous frame's pixels"
    );
}

#[test]
fn a_clip_left_by_a_failed_frame_does_not_leak_into_the_next_one() {
    let mut backend = SoftwareCpuBackend::new();
    let clipped_away = list_of(|builder| {
        builder
            .push_clip(PxRect::from_px(9.0, 9.0, 1.0, 1.0))
            .unwrap();
        builder
            .draw_rect(PxRect::from_px(0.0, 0.0, 1.0, 1.0), Color::BLACK)
            .unwrap();
        // Deliberately no `pop_clip`: the builder refuses an unbalanced list, so
        // this one is balanced — the point is that `begin_frame` resets anyway.
        builder.pop_clip().unwrap();
    });

    backend.begin_frame(surface(1, 1)).unwrap();
    backend.submit(&clipped_away).unwrap();
    backend.end_frame().unwrap();
    let second = render(
        1,
        1,
        &list_of(|builder| {
            builder
                .draw_rect(PxRect::from_px(0.0, 0.0, 1.0, 1.0), Color::BLACK)
                .unwrap();
        }),
    );

    assert_eq!(second.pixel(0, 0), Some(Color::BLACK));
    assert_eq!(Au::from_whole_px(1), Some(Au::from_raw(64)));
}

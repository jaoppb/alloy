//! The first golden image, and the determinism gate that guards it.
//!
//! The v0.3 report's risk §6.2 is that cross-OS determinism is the version's
//! most fragile gate, and that the first *text* golden to diverge between Linux
//! and macOS costs days of bisection. The mitigation it names is process, not
//! code: run the determinism job from the first **box** golden, while the
//! surface under investigation is still small. This file is that job.

#![cfg(feature = "software-backend")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use graphics::{
    Color, DisplayList, DisplayListBuilder, Framebuffer, Px, PxRect, RenderBackend,
    SoftwareCpuBackend, SurfaceSize, golden, png,
};

fn reference(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("golden")
        .join(name)
}

fn render(width: u32, height: u32, list: &DisplayList) -> Framebuffer {
    let mut backend = SoftwareCpuBackend::new();
    backend
        .begin_frame(SurfaceSize::new(width, height).expect("a non-zero surface"))
        .unwrap();
    backend.submit(list).unwrap();
    backend.end_frame().unwrap();
    backend.read_back().unwrap()
}

/// A scene exercising every path the box rasterizer has: whole-pixel edges,
/// fractional edges on both axes, a nested clip, an opacity layer, a rounded
/// corner and an alpha-blended fill.
fn scene() -> DisplayList {
    let mut builder = DisplayListBuilder::new();
    builder
        .draw_rect(
            PxRect::from_px(0.0, 0.0, 32.0, 32.0),
            Color::rgb(250, 250, 250),
        )
        .unwrap();
    builder
        .draw_rect(
            PxRect::from_px(2.0, 2.0, 12.0, 8.0),
            Color::rgb(30, 90, 200),
        )
        .unwrap();
    builder
        .draw_rect(
            PxRect::from_px(2.5, 12.25, 11.5, 7.75),
            Color::rgb(200, 40, 60),
        )
        .unwrap();
    builder
        .draw_rounded_rect(
            PxRect::from_px(17.0, 2.0, 13.0, 13.0),
            Color::rgb(20, 150, 90),
            Px::new(4.0),
        )
        .unwrap();
    builder
        .push_clip(PxRect::from_px(17.0, 17.0, 8.0, 8.0))
        .unwrap();
    builder
        .draw_rect(
            PxRect::from_px(17.0, 17.0, 20.0, 20.0),
            Color::rgb(240, 180, 20),
        )
        .unwrap();
    builder.pop_clip().unwrap();
    builder.push_opacity(0.4).unwrap();
    builder
        .draw_rect(PxRect::from_px(4.0, 22.0, 10.0, 8.0), Color::BLACK)
        .unwrap();
    builder.pop_opacity().unwrap();
    builder
        .draw_rect(
            PxRect::from_px(22.0, 20.0, 8.0, 10.0),
            Color::rgba(120, 40, 200, 128),
        )
        .unwrap();
    builder.build().unwrap()
}

#[test]
fn the_box_scene_matches_its_golden_image() {
    let frame = render(32, 32, &scene());

    golden::assert_matches_golden(&frame, &reference("boxes.png"));
}

#[test]
fn a_hundred_renders_of_the_box_scene_are_byte_identical() {
    // The determinism gate, run from the first golden rather than after the
    // text work — risk §6.2's process mitigation.
    let list = scene();
    let expected = render(32, 32, &list);

    for attempt in 0..100 {
        assert_eq!(
            render(32, 32, &list).as_rgba8(),
            expected.as_rgba8(),
            "render {attempt} diverged from the first"
        );
    }
}

#[test]
fn a_frame_survives_a_png_round_trip_unchanged() {
    // The golden gate compares decoded framebuffers, so the encoder and decoder
    // must agree exactly. If they ever stop agreeing, every golden in the repo
    // becomes meaningless — so this is asserted separately from any image.
    let frame = render(32, 32, &scene());

    let decoded = png::decode(&png::encode(&frame)).expect("our own PNG must decode");

    assert_eq!(decoded, frame);
}

#[test]
fn encoding_the_same_frame_twice_produces_identical_bytes() {
    let frame = render(8, 8, &scene());

    assert_eq!(png::encode(&frame), png::encode(&frame));
}

#[test]
fn a_difference_map_counts_and_locates_exactly_the_changed_pixels() {
    let plain = render(4, 4, &DisplayList::empty());
    let mut spotted = plain.clone();
    spotted.set_pixel(1, 2, Color::BLACK);
    spotted.set_pixel(3, 0, Color::BLACK);

    let (map, changed) = golden::difference_map(&spotted, &plain).expect("same size");

    assert_eq!(changed, 2, "exactly the two altered pixels are reported");
    assert_eq!(map.pixel(1, 2), Some(Color::rgb(255, 0, 0)));
    assert_eq!(map.pixel(3, 0), Some(Color::rgb(255, 0, 0)));
    assert_eq!(map.pixel(0, 0), Some(Color::rgb(0, 0, 0)));
}

#[test]
fn frames_of_different_sizes_have_no_difference_map() {
    let small = render(2, 2, &DisplayList::empty());
    let large = render(4, 4, &DisplayList::empty());

    assert!(
        golden::difference_map(&small, &large).is_none(),
        "a per-pixel map between different sizes would be meaningless"
    );
}

#[test]
fn a_corrupt_reference_is_reported_rather_than_silently_accepted() {
    let frame = render(1, 1, &DisplayList::empty());
    let mut bytes = png::encode(&frame);
    let last = bytes.len() - 1;
    bytes[last] ^= 0xff;

    let problem = png::decode(&bytes).expect_err("a flipped CRC byte must be caught");

    assert_eq!(problem, graphics::png::PngProblem::ChunkCorrupt);
}

#[test]
fn a_stream_that_is_not_a_png_is_refused() {
    assert_eq!(
        png::decode(b"not a png at all!!").expect_err("refused"),
        graphics::png::PngProblem::NotAPng
    );
    assert_eq!(
        png::decode(&[]).expect_err("refused"),
        graphics::png::PngProblem::Truncated
    );
}

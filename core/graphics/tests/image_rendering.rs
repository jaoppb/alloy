//! `DrawImage` (v0.5 Phase X): the golden image, determinism gate, and unit tests
//! for integer box-sampled image rendering.

#![cfg(feature = "software-backend")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;
use std::sync::Arc;

use graphics::infrastructure::png_decode::decode_png;
use graphics::{
    Color, DisplayList, DisplayListBuilder, Framebuffer, ImageId, InMemoryImageProvider, PxRect,
    RenderBackend, SoftwareCpuBackend, SurfaceSize, golden,
};

fn reference(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("golden")
        .join(name)
}

const TEST_IMAGE_ID: ImageId = ImageId::new(1);

/// Creates an 8x8 RGBA test image with colored quadrants and varied alpha,
/// encodes it to PNG via `graphics::png::encode`, and decodes it via `decode_png`.
fn sample_image() -> Framebuffer {
    let size = SurfaceSize::new(8, 8).unwrap();
    let mut frame = Framebuffer::filled(size, Color::TRANSPARENT).unwrap();

    for y in 0..8 {
        for x in 0..8 {
            let color = match (x < 4, y < 4) {
                (true, true) => Color::rgba(220, 20, 60, 255), // Crimson
                (false, true) => Color::rgba(50, 205, 50, 200), // Lime green with alpha
                (true, false) => Color::rgba(30, 144, 255, 180), // Dodger blue with alpha
                (false, false) => Color::rgba(255, 215, 0, 255), // Gold
            };
            frame.set_pixel(x, y, color);
        }
    }

    let encoded_png = graphics::png::encode(&frame);
    decode_png(&encoded_png).expect("sample image must decode via decode_png")
}

fn image_provider() -> Arc<InMemoryImageProvider> {
    Arc::new(InMemoryImageProvider::new().with_image(TEST_IMAGE_ID, sample_image()))
}

fn scene() -> DisplayList {
    let mut builder = DisplayListBuilder::new();

    // 1. 1:1 draw at (8, 8)
    builder
        .draw_image(
            TEST_IMAGE_ID,
            PxRect::from_px(0.0, 0.0, 8.0, 8.0),
            PxRect::from_px(8.0, 8.0, 8.0, 8.0),
        )
        .expect("1:1 draw image is accepted");

    // 2. Scaled up (8x8 -> 20x20) at (24, 8)
    builder
        .draw_image(
            TEST_IMAGE_ID,
            PxRect::from_px(0.0, 0.0, 8.0, 8.0),
            PxRect::from_px(24.0, 8.0, 20.0, 20.0),
        )
        .expect("scaled up draw image is accepted");

    // 3. Scaled down (8x8 -> 4x4) at (52, 8)
    builder
        .draw_image(
            TEST_IMAGE_ID,
            PxRect::from_px(0.0, 0.0, 8.0, 8.0),
            PxRect::from_px(52.0, 8.0, 4.0, 4.0),
        )
        .expect("scaled down draw image is accepted");

    // 4. Cropped sub-rectangle (top-right quadrant 4x4) scaled to (16x16) at (64, 8)
    builder
        .draw_image(
            TEST_IMAGE_ID,
            PxRect::from_px(4.0, 0.0, 4.0, 4.0),
            PxRect::from_px(64.0, 8.0, 16.0, 16.0),
        )
        .expect("cropped sub-rect is accepted");

    // 5. With Opacity layer at (8, 36)
    builder.push_opacity(0.5).expect("push opacity");
    builder
        .draw_image(
            TEST_IMAGE_ID,
            PxRect::from_px(0.0, 0.0, 8.0, 8.0),
            PxRect::from_px(8.0, 36.0, 16.0, 16.0),
        )
        .expect("draw image in opacity layer");
    builder.pop_opacity().expect("pop opacity");

    // 6. With Clip region at (36, 36)
    builder
        .push_clip(PxRect::from_px(40.0, 40.0, 12.0, 12.0))
        .expect("push clip");
    builder
        .draw_image(
            TEST_IMAGE_ID,
            PxRect::from_px(0.0, 0.0, 8.0, 8.0),
            PxRect::from_px(36.0, 36.0, 20.0, 20.0),
        )
        .expect("draw image clipped");
    builder.pop_clip().expect("pop clip");

    builder.build().expect("balanced list builds")
}

fn render(list: &DisplayList) -> Framebuffer {
    let mut backend = SoftwareCpuBackend::with_image_provider(image_provider());
    backend
        .begin_frame(SurfaceSize::new(96, 64).expect("a non-zero surface"))
        .unwrap();
    backend.submit(list).unwrap();
    backend.end_frame().unwrap();
    backend.read_back().unwrap()
}

#[test]
fn the_image_scene_matches_its_golden_image() {
    let frame = render(&scene());
    golden::assert_matches_golden(&frame, &reference("image.png"));
}

#[test]
fn a_hundred_renders_of_the_image_scene_are_byte_identical() {
    let list = scene();
    let reference_frame = render(&list);
    for attempt in 0..100 {
        let candidate = render(&list);
        assert_eq!(
            candidate.as_rgba8(),
            reference_frame.as_rgba8(),
            "render #{attempt} diverged from the reference frame",
        );
    }
}

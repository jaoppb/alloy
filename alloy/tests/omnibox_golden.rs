//! Declarative omnibox rendering and determinism golden gate (ADR-0016, Epic #43 / US-04).
//!
//! Asserts that:
//! 1. The chrome emits declarative drawing commands to [`DisplayListBuilder`]
//!    (background rectangle, input box with `corner_radius`, buffer text).
//! 2. When focused, the omnibox renders an active focus border and a vertical cursor bar
//!    (caret) at the current insertion point.
//! 3. Caret horizontal offset advances deterministically with font advance metrics,
//!    supporting UTF-8 multibyte scalar values without raw indexing.
//! 4. Empty omnibox state emits minimal commands without text runs.
//! 5. Rasterization with [`SoftwareCpuBackend`] is byte-for-byte deterministic over 100 runs.
//! 6. Blesses and asserts against golden reference PNG images.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::as_conversions,
    clippy::cast_precision_loss
)]

use std::path::PathBuf;
use std::sync::Arc;

use alloy::application::omnibox::{
    CARET_COLOR, CARET_WIDTH, CHROME_BACKGROUND_COLOR, ChromeHeight, DEFAULT_CHROME_HEIGHT,
    FOCUS_BORDER_COLOR, FOCUS_BORDER_WIDTH, FOCUS_INNER_RADIUS, INPUT_BACKGROUND_COLOR,
    INPUT_BOX_HEIGHT, INPUT_BOX_MARGIN_X, INPUT_BOX_RADIUS, OmniboxPainter, OmniboxState,
    OmniboxTheme, TEXT_COLOR, Theme, compute_cursor_offset, paint_omnibox,
    paint_omnibox_with_theme,
};
use alloy::application::paint::DEFAULT_FONT;
use alloy::application::pipeline::DEFAULT_FONT_SIZE;
use graphics::golden::assert_matches_golden;
use graphics::{
    Au, DisplayCommand, DisplayListBuilder, FontProvider, Framebuffer, GraphicsError, Point, Px,
    PxRect, RenderBackend, SoftwareCpuBackend, SurfaceSize, SyntheticFontProvider,
};
use network::Url;

fn golden_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("golden")
        .join(name)
}

fn deterministic_font_provider() -> SyntheticFontProvider {
    SyntheticFontProvider::new().with_size(DEFAULT_FONT, DEFAULT_FONT_SIZE)
}

#[test]
fn unfocused_omnibox_emits_expected_display_command_sequence() {
    let url = Url::parse("https://example.com/").expect("valid url");
    let state = OmniboxState::with_url(url);
    let provider = deterministic_font_provider();
    let mut builder = DisplayListBuilder::new();

    paint_omnibox(&state, Px::new(800.0), &provider, &mut builder)
        .expect("painting unfocused omnibox succeeds");
    let display_list = builder.build().expect("display list builds cleanly");

    let commands: Vec<_> = display_list.iter().collect();
    assert_eq!(
        commands.len(),
        3,
        "unfocused non-empty omnibox must emit exactly 3 commands"
    );

    // Command 0: Chrome background rectangle
    match commands[0] {
        DisplayCommand::DrawRect {
            rect,
            color,
            corner_radius,
        } => {
            assert_eq!(rect.origin(), Point::ORIGIN);
            assert_eq!(rect.size().width().to_px(), Px::new(800.0));
            assert_eq!(rect.size().height().to_px(), Px::new(42.0));
            assert_eq!(*color, CHROME_BACKGROUND_COLOR);
            assert_eq!(*corner_radius, Au::ZERO);
        }
        other => panic!("expected DrawRect for chrome background, got {other:?}"),
    }

    // Command 1: Input box with 6px corner radius
    match commands[1] {
        DisplayCommand::DrawRect {
            rect,
            color,
            corner_radius,
        } => {
            assert_eq!(rect.origin().horizontal(), INPUT_BOX_MARGIN_X);
            assert_eq!(
                rect.size().width(),
                Au::from_px(Px::new(800.0))
                    .expect("au")
                    .saturating_sub(INPUT_BOX_MARGIN_X.saturating_add(INPUT_BOX_MARGIN_X))
            );
            assert_eq!(rect.size().height(), INPUT_BOX_HEIGHT);
            assert_eq!(*color, INPUT_BACKGROUND_COLOR);
            assert_eq!(*corner_radius, Au::from_px(INPUT_BOX_RADIUS).expect("au"));
        }
        other => panic!("expected DrawRect for input box, got {other:?}"),
    }

    // Command 2: Text run with buffer text
    match commands[2] {
        DisplayCommand::DrawText {
            glyphs,
            color,
            font,
        } => {
            assert_eq!(
                glyphs.len(),
                20,
                "glyphs count must match length of 'https://example.com/'"
            );
            assert_eq!(*color, TEXT_COLOR);
            assert_eq!(*font, DEFAULT_FONT);
        }
        other => panic!("expected DrawText for buffer text, got {other:?}"),
    }
}

#[test]
fn focused_omnibox_emits_active_border_and_caret() {
    let url = Url::parse("https://example.com/").expect("valid url");
    let mut state = OmniboxState::with_url(url);
    state.focus();
    let provider = deterministic_font_provider();
    let mut builder = DisplayListBuilder::new();

    paint_omnibox(&state, Px::new(800.0), &provider, &mut builder)
        .expect("painting focused omnibox succeeds");
    let display_list = builder.build().expect("display list builds cleanly");

    let commands: Vec<_> = display_list.iter().collect();
    assert_eq!(
        commands.len(),
        5,
        "focused non-empty omnibox must emit exactly 5 commands"
    );

    // Command 0: Chrome background
    match commands[0] {
        DisplayCommand::DrawRect { color, .. } => {
            assert_eq!(*color, CHROME_BACKGROUND_COLOR);
        }
        other => panic!("expected DrawRect for chrome background, got {other:?}"),
    }

    // Command 1: Active focus border ring (6px corner radius, focus color)
    match commands[1] {
        DisplayCommand::DrawRect {
            color,
            corner_radius,
            ..
        } => {
            assert_eq!(*color, FOCUS_BORDER_COLOR);
            assert_eq!(*corner_radius, Au::from_px(INPUT_BOX_RADIUS).expect("au"));
        }
        other => panic!("expected DrawRect for focus ring, got {other:?}"),
    }

    // Command 2: Inner input box (inset by 2px, 4px corner radius, white background)
    match commands[2] {
        DisplayCommand::DrawRect {
            rect,
            color,
            corner_radius,
        } => {
            assert_eq!(
                rect.origin().horizontal(),
                INPUT_BOX_MARGIN_X.saturating_add(FOCUS_BORDER_WIDTH)
            );
            assert_eq!(*color, INPUT_BACKGROUND_COLOR);
            assert_eq!(*corner_radius, Au::from_px(FOCUS_INNER_RADIUS).expect("au"));
        }
        other => panic!("expected DrawRect for inner input box, got {other:?}"),
    }

    // Command 3: Text run
    match commands[3] {
        DisplayCommand::DrawText {
            glyphs,
            color,
            font,
        } => {
            assert_eq!(glyphs.len(), 20);
            assert_eq!(*color, TEXT_COLOR);
            assert_eq!(*font, DEFAULT_FONT);
        }
        other => panic!("expected DrawText, got {other:?}"),
    }

    // Command 4: Vertical cursor bar (caret)
    match commands[4] {
        DisplayCommand::DrawRect {
            rect,
            color,
            corner_radius,
        } => {
            assert_eq!(*color, CARET_COLOR);
            assert_eq!(*corner_radius, Au::ZERO);
            assert_eq!(rect.size().width(), CARET_WIDTH);
            let metrics = provider.metrics(DEFAULT_FONT).expect("metrics");
            assert_eq!(
                rect.size().height(),
                metrics.ascent().saturating_add(metrics.descent())
            );
        }
        other => panic!("expected DrawRect for caret, got {other:?}"),
    }
}

#[test]
fn cursor_caret_horizontal_offset_advances_deterministically() {
    let mut state = OmniboxState::new();
    state.focus();
    let provider = deterministic_font_provider();

    // At position 0, offset must be zero
    let initial_offset =
        compute_cursor_offset(&state, DEFAULT_FONT, &provider).expect("compute offset");
    assert_eq!(initial_offset, Au::ZERO);

    // Insert 3 ASCII characters
    state.insert_text("abc");
    let after_abc_offset =
        compute_cursor_offset(&state, DEFAULT_FONT, &provider).expect("compute offset");
    let glyph = provider.glyph_for_char(DEFAULT_FONT, 'a').expect("glyph");
    let char_advance = provider.advance(DEFAULT_FONT, glyph).expect("advance");
    assert_eq!(
        after_abc_offset,
        char_advance
            .saturating_add(char_advance)
            .saturating_add(char_advance)
    );

    // Move cursor left 1 char
    state.move_left();
    let left_offset =
        compute_cursor_offset(&state, DEFAULT_FONT, &provider).expect("compute offset");
    assert_eq!(left_offset, char_advance.saturating_add(char_advance));

    // Move to start
    state.move_start();
    let start_offset =
        compute_cursor_offset(&state, DEFAULT_FONT, &provider).expect("compute offset");
    assert_eq!(start_offset, Au::ZERO);
}

#[test]
fn compute_cursor_offset_handles_multibyte_utf8() {
    let mut state = OmniboxState::new();
    state.focus();
    let provider = deterministic_font_provider();

    // Multibyte string with accents and multi-byte characters
    state.insert_text("héllo 世界 🦀");
    let full_offset =
        compute_cursor_offset(&state, DEFAULT_FONT, &provider).expect("compute offset");
    assert!(
        full_offset > Au::ZERO,
        "full offset for multibyte string must be strictly positive"
    );

    // Move cursor left 2 Unicode characters
    state.move_left();
    state.move_left();
    let moved_offset =
        compute_cursor_offset(&state, DEFAULT_FONT, &provider).expect("compute offset");
    assert!(
        moved_offset < full_offset,
        "offset must decrease as cursor moves left across multibyte scalars"
    );
}

#[test]
fn empty_omnibox_command_sequences() {
    let state = OmniboxState::new();
    let provider = deterministic_font_provider();

    // Unfocused empty: chrome background + input box
    let mut unfocused_builder = DisplayListBuilder::new();
    paint_omnibox(&state, Px::new(600.0), &provider, &mut unfocused_builder)
        .expect("paint unfocused empty");
    let unfocused_list = unfocused_builder.build().expect("build list");
    assert_eq!(
        unfocused_list.len(),
        2,
        "unfocused empty omnibox must emit exactly background and input box"
    );

    // Focused empty: background + focus border + input box + caret
    let mut focused_state = OmniboxState::new();
    focused_state.focus();
    let mut focused_builder = DisplayListBuilder::new();
    paint_omnibox(
        &focused_state,
        Px::new(600.0),
        &provider,
        &mut focused_builder,
    )
    .expect("paint focused empty");
    let focused_list = focused_builder.build().expect("build list");
    assert_eq!(
        focused_list.len(),
        4,
        "focused empty omnibox must emit background, focus border, input box and caret"
    );
}

#[test]
fn custom_chrome_height_scales_input_box_and_background() {
    let custom_height = ChromeHeight::from_px(Px::new(50.0));
    let provider = deterministic_font_provider();
    let painter = OmniboxPainter::new(&provider).with_chrome_height(custom_height);
    let mut builder = DisplayListBuilder::new();
    let state = OmniboxState::new();

    painter
        .paint(&state, Px::new(800.0), &mut builder)
        .expect("paint custom chrome height");
    let list = builder.build().expect("build list");
    let commands: Vec<_> = list.iter().collect();

    match commands[0] {
        DisplayCommand::DrawRect { rect, .. } => {
            assert_eq!(rect.size().height().to_px(), Px::new(50.0));
        }
        other => panic!("expected DrawRect for background, got {other:?}"),
    }
}

#[test]
fn painter_input_box_bounds_is_consistent_with_geometry() {
    let provider = deterministic_font_provider();
    let painter = OmniboxPainter::new(&provider);
    let bounds = painter
        .input_box_bounds(Px::new(800.0))
        .expect("bounds calculated");

    let vertical_space = DEFAULT_CHROME_HEIGHT.saturating_sub(INPUT_BOX_HEIGHT);
    let vertical_offset = Au::from_raw(vertical_space.raw() / 2);
    let expected = PxRect::new(
        INPUT_BOX_MARGIN_X.to_px(),
        vertical_offset.to_px(),
        Px::new(800.0 - 24.0),
        INPUT_BOX_HEIGHT.to_px(),
    );
    assert_eq!(bounds, expected);
}

fn rasterize_omnibox(
    state: &OmniboxState,
    width: u32,
    provider: Arc<SyntheticFontProvider>,
) -> Result<Framebuffer, GraphicsError> {
    let mut builder = DisplayListBuilder::new();
    let width_px = Px::new(width as f32);
    paint_omnibox(state, width_px, &*provider, &mut builder)?;
    let display_list = builder.build()?;

    let surface_size = SurfaceSize::new(width, 42).expect("valid surface size");
    let mut backend = SoftwareCpuBackend::with_font_provider(provider);
    backend.begin_frame(surface_size)?;
    backend.submit(&display_list)?;
    backend.end_frame()?;
    backend.read_back()
}

#[test]
fn omnibox_rasterizes_deterministically_over_100_runs() {
    let url = Url::parse("https://example.com/test").expect("valid url");
    let mut state = OmniboxState::with_url(url);
    state.focus();

    let provider = Arc::new(deterministic_font_provider());
    let reference_frame =
        rasterize_omnibox(&state, 640, Arc::clone(&provider)).expect("initial render");

    for iteration in 1..=100 {
        let frame =
            rasterize_omnibox(&state, 640, Arc::clone(&provider)).expect("subsequent render");
        assert_eq!(
            reference_frame, frame,
            "omnibox rasterization diverged at iteration {iteration}"
        );
    }
}

#[test]
fn omnibox_renders_to_golden_reference() {
    let url = Url::parse("https://example.com/").expect("valid url");
    let provider = Arc::new(deterministic_font_provider());

    // 1. Unfocused reference
    let unfocused_state = OmniboxState::with_url(url.clone());
    let unfocused_frame =
        rasterize_omnibox(&unfocused_state, 640, Arc::clone(&provider)).expect("render unfocused");
    assert_matches_golden(&unfocused_frame, &golden_path("omnibox_unfocused.png"));

    // 2. Focused reference
    let mut focused_state = OmniboxState::with_url(url);
    focused_state.focus();
    let focused_frame =
        rasterize_omnibox(&focused_state, 640, Arc::clone(&provider)).expect("render focused");
    assert_matches_golden(&focused_frame, &golden_path("omnibox_focused.png"));
}

#[test]
fn custom_theme_styling_and_dynamic_colors_rendered_into_display_list() {
    let custom_bg = graphics::Color::rgb(30, 30, 30);
    let custom_input_bg = graphics::Color::rgb(50, 50, 50);
    let custom_focus_border = graphics::Color::rgb(255, 100, 0);
    let custom_text_color = graphics::Color::rgb(240, 240, 240);
    let custom_caret_color = graphics::Color::rgb(255, 200, 0);

    let theme = OmniboxTheme::new()
        .with_chrome_background_color(custom_bg)
        .with_input_background_color(custom_input_bg)
        .with_focus_border_color(custom_focus_border)
        .with_text_color(custom_text_color)
        .with_caret_color(custom_caret_color);

    let url = Url::parse("https://example.com/").expect("valid url");
    let mut state = OmniboxState::with_url(url);
    state.focus();

    let provider = deterministic_font_provider();
    let mut builder = DisplayListBuilder::new();

    paint_omnibox_with_theme(&state, Px::new(800.0), &provider, theme, &mut builder)
        .expect("painting with theme succeeds");
    let display_list = builder.build().expect("build succeeds");

    let commands: Vec<_> = display_list.iter().collect();
    assert_eq!(commands.len(), 5);

    // Command 0: custom chrome background
    match commands[0] {
        DisplayCommand::DrawRect { color, .. } => assert_eq!(*color, custom_bg),
        other => panic!("expected DrawRect for background, got {other:?}"),
    }

    // Command 1: custom focus border
    match commands[1] {
        DisplayCommand::DrawRect { color, .. } => assert_eq!(*color, custom_focus_border),
        other => panic!("expected DrawRect for focus border, got {other:?}"),
    }

    // Command 2: custom inner input background
    match commands[2] {
        DisplayCommand::DrawRect { color, .. } => assert_eq!(*color, custom_input_bg),
        other => panic!("expected DrawRect for inner input, got {other:?}"),
    }

    // Command 3: custom text color
    match commands[3] {
        DisplayCommand::DrawText { color, .. } => assert_eq!(*color, custom_text_color),
        other => panic!("expected DrawText for text, got {other:?}"),
    }

    // Command 4: custom caret color
    match commands[4] {
        DisplayCommand::DrawRect { color, .. } => assert_eq!(*color, custom_caret_color),
        other => panic!("expected DrawRect for caret, got {other:?}"),
    }

    // Verify painter struct debug format and copy/clone
    let painter = OmniboxPainter::new(&provider).with_theme(theme);
    let painter_copy = painter;
    let debug_repr = format!("{painter_copy:?}");
    assert!(debug_repr.contains("OmniboxPainter"));

    let root_theme = Theme::new().with_omnibox(theme);
    assert_eq!(*root_theme.omnibox(), theme);
}

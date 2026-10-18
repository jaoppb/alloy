//! Declarative rendering of the browser search and address bar (omnibox).
//!
//! Emits declarative [`DisplayCommand`] instructions into a [`DisplayListBuilder`],
//! adhering to the Skeleton and Muscle architecture (ADR-0003) and Object Calisthenics
//! (ADR-0010) without third-party GUI dependencies.

use graphics::{
    Au, Color, DisplayListBuilder, FaceMetrics, FontId, FontProvider, GlyphId, GlyphInstance,
    GlyphRun, GraphicsError, Point, Px, PxRect, Rect, Size,
};

use super::state::OmniboxState;
use crate::application::paint::DEFAULT_FONT;

/// Default vertical height of the chrome navigation bar in app units (42 px).
pub const DEFAULT_CHROME_HEIGHT: Au = Au::from_raw(42 * graphics::AU_PER_PX);

/// Default height of the omnibox input box in app units (28 px).
pub const INPUT_BOX_HEIGHT: Au = Au::from_raw(28 * graphics::AU_PER_PX);

/// Horizontal margin on left and right sides of the input box in app units (12 px).
pub const INPUT_BOX_MARGIN_X: Au = Au::from_raw(12 * graphics::AU_PER_PX);

/// Corner radius of the input box (6 px), per specification.
pub const INPUT_BOX_RADIUS: Px = Px::new(6.0);

/// Thickness of the active focus border in app units (2 px).
pub const FOCUS_BORDER_WIDTH: Au = Au::from_raw(2 * graphics::AU_PER_PX);

/// Inner corner radius when focused (4 px = 6 px - 2 px).
pub const FOCUS_INNER_RADIUS: Px = Px::new(4.0);

/// Horizontal padding inside the input box before text begins in app units (10 px).
pub const TEXT_PADDING_X: Au = Au::from_raw(10 * graphics::AU_PER_PX);

/// Width of the vertical cursor caret in app units (2 px).
pub const CARET_WIDTH: Au = Au::from_raw(2 * graphics::AU_PER_PX);

/// Background color of the chrome top bar (#F3F4F6).
pub const CHROME_BACKGROUND_COLOR: Color = Color::rgb(243, 244, 246);

/// Background color of the omnibox input field (white).
pub const INPUT_BACKGROUND_COLOR: Color = Color::WHITE;

/// Active focus border ring color (#3B82F6).
pub const FOCUS_BORDER_COLOR: Color = Color::rgb(59, 130, 246);

/// Primary text color for typed input or active URL (#111827).
pub const TEXT_COLOR: Color = Color::rgb(17, 24, 39);

/// Color of the vertical insertion caret.
pub const CARET_COLOR: Color = Color::rgb(17, 24, 39);

/// Strongly-typed height of the browser chrome bar at the top of the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChromeHeight(Au);

impl ChromeHeight {
    /// Default chrome height (42 px).
    pub const DEFAULT: Self = Self(DEFAULT_CHROME_HEIGHT);

    /// Create a new chrome height from app units.
    #[must_use]
    pub const fn from_au(height: Au) -> Self {
        Self(height)
    }

    /// Create a new chrome height from pixels.
    #[must_use]
    pub fn from_px(height: Px) -> Self {
        Self(Au::from_px(height).unwrap_or(DEFAULT_CHROME_HEIGHT))
    }

    /// The chrome height in app units.
    #[must_use]
    pub const fn to_au(self) -> Au {
        self.0
    }

    /// The chrome height in pixels.
    #[must_use]
    pub fn to_px(self) -> Px {
        self.0.to_px()
    }
}

impl Default for ChromeHeight {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Declarative painter for the browser omnibox navigation chrome.
#[derive(Clone, Copy, Debug)]
pub struct OmniboxPainter<'a, F> {
    font_provider: &'a F,
    font: FontId,
    chrome_height: ChromeHeight,
}

impl<'a, F: FontProvider> OmniboxPainter<'a, F> {
    /// Create a new omnibox painter using the specified font provider.
    #[must_use]
    pub const fn new(font_provider: &'a F) -> Self {
        Self {
            font_provider,
            font: DEFAULT_FONT,
            chrome_height: ChromeHeight::DEFAULT,
        }
    }

    /// Set a custom chrome height.
    #[must_use]
    pub const fn with_chrome_height(mut self, height: ChromeHeight) -> Self {
        self.chrome_height = height;
        self
    }

    /// Set a custom font identifier for text rendering.
    #[must_use]
    pub const fn with_font(mut self, font: FontId) -> Self {
        self.font = font;
        self
    }

    /// Access the registered chrome height.
    #[must_use]
    pub const fn chrome_height(&self) -> ChromeHeight {
        self.chrome_height
    }

    /// Access the active font identifier.
    #[must_use]
    pub const fn font(&self) -> FontId {
        self.font
    }

    /// Returns the bounding rectangle of the omnibox input box in pixels.
    #[must_use]
    pub fn input_box_bounds(&self, width: Px) -> Option<PxRect> {
        let width_au = Au::from_px(width)?;
        let bounds = self.calculate_input_bounds(width_au)?;
        Some(rect_to_px_rect(bounds))
    }

    /// Paints the omnibox chrome into the display list builder.
    pub fn paint(
        &self,
        state: &OmniboxState,
        width: Px,
        builder: &mut DisplayListBuilder,
    ) -> Result<(), GraphicsError> {
        let Some(width_au) = Au::from_px(width) else {
            return Ok(());
        };
        if width_au.is_zero() || width_au.is_negative() {
            return Ok(());
        }
        self.paint_valid(state, width_au, builder)
    }

    fn paint_valid(
        &self,
        state: &OmniboxState,
        width: Au,
        builder: &mut DisplayListBuilder,
    ) -> Result<(), GraphicsError> {
        self.paint_background(width, builder)?;
        let Some(bounds) = self.calculate_input_bounds(width) else {
            return Ok(());
        };
        paint_input_chrome(state, bounds, builder)?;
        self.paint_content(state, bounds, builder)
    }

    fn calculate_input_bounds(&self, width: Au) -> Option<Rect> {
        let double_margin = INPUT_BOX_MARGIN_X.saturating_add(INPUT_BOX_MARGIN_X);
        let input_width = width.saturating_sub(double_margin);
        if input_width.is_zero() {
            return None;
        }
        let input_size = Size::new(input_width, INPUT_BOX_HEIGHT)?;
        let chrome_height_au = self.chrome_height.to_au();
        let vertical_space = chrome_height_au.saturating_sub(INPUT_BOX_HEIGHT);
        let half_vertical_space = vertical_space.raw().checked_div(2).unwrap_or(0);
        let vertical_offset = Au::from_raw(half_vertical_space);
        let origin = Point::new(INPUT_BOX_MARGIN_X, vertical_offset);
        Some(Rect::new(origin, input_size))
    }

    fn paint_background(
        &self,
        width: Au,
        builder: &mut DisplayListBuilder,
    ) -> Result<(), GraphicsError> {
        let chrome_height_au = self.chrome_height.to_au();
        let Some(size) = Size::new(width, chrome_height_au) else {
            return Ok(());
        };
        let chrome_rect = Rect::new(Point::ORIGIN, size);
        builder.draw_rect(rect_to_px_rect(chrome_rect), CHROME_BACKGROUND_COLOR)
    }

    fn paint_content(
        &self,
        state: &OmniboxState,
        bounds: Rect,
        builder: &mut DisplayListBuilder,
    ) -> Result<(), GraphicsError> {
        let metrics = self.font_provider.metrics(self.font)?;
        let baseline_y = calculate_baseline_y(bounds, metrics);
        let start_x = bounds.min_x().saturating_add(TEXT_PADDING_X);
        self.paint_text(state, start_x, baseline_y, builder)?;
        self.paint_caret_if_focused(state, start_x, baseline_y, metrics, builder)
    }

    fn paint_text(
        &self,
        state: &OmniboxState,
        start_x: Au,
        baseline_y: Au,
        builder: &mut DisplayListBuilder,
    ) -> Result<(), GraphicsError> {
        let text = state.text();
        if text.is_empty() {
            return Ok(());
        }
        let run = self.build_glyph_run(&text, start_x, baseline_y)?;
        if run.is_empty() {
            return Ok(());
        }
        builder.draw_text(run, TEXT_COLOR, self.font)
    }

    fn build_glyph_run(
        &self,
        text: &str,
        start_x: Au,
        baseline_y: Au,
    ) -> Result<GlyphRun, GraphicsError> {
        let mut run = GlyphRun::new();
        let mut pen_x = start_x;
        for character in text.chars() {
            pen_x = self.append_glyph(&mut run, character, pen_x, baseline_y)?;
        }
        Ok(run)
    }

    fn append_glyph(
        &self,
        run: &mut GlyphRun,
        character: char,
        pen_x: Au,
        baseline_y: Au,
    ) -> Result<Au, GraphicsError> {
        let glyph = self.font_provider.glyph_for_char(self.font, character)?;
        if glyph != GlyphId::NOTDEF {
            run.push(GlyphInstance::new(glyph, Point::new(pen_x, baseline_y)));
        }
        let advance = self.font_provider.advance(self.font, glyph)?;
        Ok(pen_x.saturating_add(advance))
    }

    fn paint_caret_if_focused(
        &self,
        state: &OmniboxState,
        start_x: Au,
        baseline_y: Au,
        metrics: FaceMetrics,
        builder: &mut DisplayListBuilder,
    ) -> Result<(), GraphicsError> {
        if !state.is_focused() {
            return Ok(());
        }
        let cursor_offset = compute_cursor_offset(state, self.font, self.font_provider)?;
        let caret_x = start_x.saturating_add(cursor_offset);
        let caret_y = baseline_y.saturating_sub(metrics.ascent());
        let caret_height = metrics.ascent().saturating_add(metrics.descent());
        let Some(caret_size) = Size::new(CARET_WIDTH, caret_height) else {
            return Ok(());
        };
        let caret_rect = Rect::new(Point::new(caret_x, caret_y), caret_size);
        builder.draw_rect(rect_to_px_rect(caret_rect), CARET_COLOR)
    }
}

fn paint_input_chrome(
    state: &OmniboxState,
    bounds: Rect,
    builder: &mut DisplayListBuilder,
) -> Result<(), GraphicsError> {
    if state.is_focused() {
        return paint_focused_borders(bounds, builder);
    }
    paint_unfocused_border(bounds, builder)
}

fn paint_focused_borders(
    bounds: Rect,
    builder: &mut DisplayListBuilder,
) -> Result<(), GraphicsError> {
    builder.draw_rounded_rect(
        rect_to_px_rect(bounds),
        FOCUS_BORDER_COLOR,
        INPUT_BOX_RADIUS,
    )?;
    let Some(inner_bounds) = inset_rect(bounds, FOCUS_BORDER_WIDTH) else {
        return Ok(());
    };
    builder.draw_rounded_rect(
        rect_to_px_rect(inner_bounds),
        INPUT_BACKGROUND_COLOR,
        FOCUS_INNER_RADIUS,
    )
}

fn paint_unfocused_border(
    bounds: Rect,
    builder: &mut DisplayListBuilder,
) -> Result<(), GraphicsError> {
    builder.draw_rounded_rect(
        rect_to_px_rect(bounds),
        INPUT_BACKGROUND_COLOR,
        INPUT_BOX_RADIUS,
    )
}

/// Paints the browser omnibox chrome bar into a [`DisplayListBuilder`].
pub fn paint_omnibox<F: FontProvider>(
    state: &OmniboxState,
    width: Px,
    font_provider: &F,
    builder: &mut DisplayListBuilder,
) -> Result<(), GraphicsError> {
    OmniboxPainter::new(font_provider).paint(state, width, builder)
}

/// Computes the horizontal advance in [`Au`] from the start of the text to the cursor.
pub fn compute_cursor_offset<F: FontProvider>(
    state: &OmniboxState,
    font: FontId,
    font_provider: &F,
) -> Result<Au, GraphicsError> {
    let text_before = state.buffer().text_before_cursor();
    accumulate_text_advance(text_before, font, font_provider)
}

fn accumulate_text_advance<F: FontProvider>(
    text: &str,
    font: FontId,
    font_provider: &F,
) -> Result<Au, GraphicsError> {
    let mut total_advance = Au::ZERO;
    for character in text.chars() {
        let advance = advance_for_char(font_provider, font, character)?;
        total_advance = total_advance.saturating_add(advance);
    }
    Ok(total_advance)
}

fn advance_for_char<F: FontProvider>(
    font_provider: &F,
    font: FontId,
    character: char,
) -> Result<Au, GraphicsError> {
    let glyph = font_provider.glyph_for_char(font, character)?;
    font_provider.advance(font, glyph)
}

const fn inset_rect(rect: Rect, inset: Au) -> Option<Rect> {
    let double_inset = inset.saturating_add(inset);
    let new_width = rect.size().width().saturating_sub(double_inset);
    let new_height = rect.size().height().saturating_sub(double_inset);
    let Some(size) = Size::new(new_width, new_height) else {
        return None;
    };
    let origin = Point::new(
        rect.min_x().saturating_add(inset),
        rect.min_y().saturating_add(inset),
    );
    Some(Rect::new(origin, size))
}

fn calculate_baseline_y(bounds: Rect, metrics: FaceMetrics) -> Au {
    let text_height = metrics.ascent().saturating_add(metrics.descent());
    let vertical_space = bounds.size().height().saturating_sub(text_height);
    let half_vertical_space = vertical_space.raw().checked_div(2).unwrap_or(0);
    let vertical_margin = Au::from_raw(half_vertical_space);
    bounds
        .min_y()
        .saturating_add(vertical_margin)
        .saturating_add(metrics.ascent())
}

fn rect_to_px_rect(rect: Rect) -> PxRect {
    PxRect::new(
        rect.min_x().to_px(),
        rect.min_y().to_px(),
        rect.size().width().to_px(),
        rect.size().height().to_px(),
    )
}

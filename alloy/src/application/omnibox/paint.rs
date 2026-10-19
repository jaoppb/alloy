//! Declarative rendering of the browser search and address bar (omnibox).
//!
//! Emits declarative [`DisplayCommand`] instructions into a [`DisplayListBuilder`],
//! adhering to the Skeleton and Muscle architecture (ADR-0003) and Object Calisthenics
//! (ADR-0010) without third-party GUI dependencies.

use graphics::{
    Au, DisplayListBuilder, FaceMetrics, FontId, FontProvider, GlyphId, GlyphInstance, GlyphRun,
    GraphicsError, Point, Px, PxRect, Rect, Size,
};

use super::state::OmniboxState;
use crate::application::paint::DEFAULT_FONT;
pub use crate::application::theme::{
    CARET_COLOR, CARET_WIDTH, CHROME_BACKGROUND_COLOR, ChromeHeight, DEFAULT_CHROME_HEIGHT,
    FOCUS_BORDER_COLOR, FOCUS_BORDER_WIDTH, FOCUS_INNER_RADIUS, INPUT_BACKGROUND_COLOR,
    INPUT_BOX_HEIGHT, INPUT_BOX_MARGIN_X, INPUT_BOX_RADIUS, OmniboxTheme, TEXT_COLOR,
    TEXT_PADDING_X, Theme,
};

/// Declarative painter for the browser omnibox navigation chrome.
pub struct OmniboxPainter<'a, F: FontProvider> {
    font_provider: &'a F,
    font: FontId,
    theme: OmniboxTheme,
}

impl<F: FontProvider> Clone for OmniboxPainter<'_, F> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<F: FontProvider> Copy for OmniboxPainter<'_, F> {}

impl<F: FontProvider> std::fmt::Debug for OmniboxPainter<'_, F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OmniboxPainter")
            .field("font", &self.font)
            .field("theme", &self.theme)
            .finish_non_exhaustive()
    }
}

impl<'a, F: FontProvider> OmniboxPainter<'a, F> {
    /// Create a new omnibox painter using the specified font provider.
    #[must_use]
    pub const fn new(font_provider: &'a F) -> Self {
        Self {
            font_provider,
            font: DEFAULT_FONT,
            theme: OmniboxTheme::DEFAULT,
        }
    }

    /// Set a custom omnibox styling theme.
    #[must_use]
    pub const fn with_theme(mut self, theme: OmniboxTheme) -> Self {
        self.theme = theme;
        self
    }

    /// Set a custom chrome height.
    #[must_use]
    pub const fn with_chrome_height(mut self, height: ChromeHeight) -> Self {
        self.theme = self.theme.with_chrome_height(height);
        self
    }

    /// Set a custom font identifier for text rendering.
    #[must_use]
    pub const fn with_font(mut self, font: FontId) -> Self {
        self.font = font;
        self
    }

    /// Access the registered styling theme.
    #[must_use]
    pub const fn theme(&self) -> &OmniboxTheme {
        &self.theme
    }

    /// Access the registered chrome height.
    #[must_use]
    pub const fn chrome_height(&self) -> ChromeHeight {
        self.theme.chrome_height()
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
        paint_input_chrome(state, bounds, &self.theme, builder)?;
        self.paint_content(state, bounds, builder)
    }

    fn calculate_input_bounds(&self, width: Au) -> Option<Rect> {
        let margin_x = self.theme.input_box_margin_x();
        let double_margin = margin_x.saturating_add(margin_x);
        let input_width = width.saturating_sub(double_margin);
        if input_width.is_zero() {
            return None;
        }
        let input_size = Size::new(input_width, self.theme.input_box_height())?;
        let chrome_height_au = self.chrome_height().to_au();
        let vertical_space = chrome_height_au.saturating_sub(self.theme.input_box_height());
        let half_vertical_space = vertical_space.raw().checked_div(2).unwrap_or(0);
        let vertical_offset = Au::from_raw(half_vertical_space);
        let origin = Point::new(margin_x, vertical_offset);
        Some(Rect::new(origin, input_size))
    }

    fn paint_background(
        &self,
        width: Au,
        builder: &mut DisplayListBuilder,
    ) -> Result<(), GraphicsError> {
        let chrome_height_au = self.chrome_height().to_au();
        let Some(size) = Size::new(width, chrome_height_au) else {
            return Ok(());
        };
        let chrome_rect = Rect::new(Point::ORIGIN, size);
        builder.draw_rect(
            rect_to_px_rect(chrome_rect),
            self.theme.chrome_background_color(),
        )
    }

    fn paint_content(
        &self,
        state: &OmniboxState,
        bounds: Rect,
        builder: &mut DisplayListBuilder,
    ) -> Result<(), GraphicsError> {
        let metrics = self.font_provider.metrics(self.font)?;
        let baseline_y = calculate_baseline_y(bounds, metrics);
        let start_x = bounds.min_x().saturating_add(self.theme.text_padding_x());
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
        builder.draw_text(run, self.theme.text_color(), self.font)
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
        let Some(caret_size) = Size::new(self.theme.caret_width(), caret_height) else {
            return Ok(());
        };
        let caret_rect = Rect::new(Point::new(caret_x, caret_y), caret_size);
        builder.draw_rect(rect_to_px_rect(caret_rect), self.theme.caret_color())
    }
}

fn paint_input_chrome(
    state: &OmniboxState,
    bounds: Rect,
    theme: &OmniboxTheme,
    builder: &mut DisplayListBuilder,
) -> Result<(), GraphicsError> {
    if state.is_focused() {
        return paint_focused_borders(bounds, theme, builder);
    }
    paint_unfocused_border(bounds, theme, builder)
}

fn paint_focused_borders(
    bounds: Rect,
    theme: &OmniboxTheme,
    builder: &mut DisplayListBuilder,
) -> Result<(), GraphicsError> {
    builder.draw_rounded_rect(
        rect_to_px_rect(bounds),
        theme.focus_border_color(),
        theme.input_box_radius(),
    )?;
    let Some(inner_bounds) = inset_rect(bounds, theme.focus_border_width()) else {
        return Ok(());
    };
    builder.draw_rounded_rect(
        rect_to_px_rect(inner_bounds),
        theme.input_background_color(),
        theme.focus_inner_radius(),
    )
}

fn paint_unfocused_border(
    bounds: Rect,
    theme: &OmniboxTheme,
    builder: &mut DisplayListBuilder,
) -> Result<(), GraphicsError> {
    builder.draw_rounded_rect(
        rect_to_px_rect(bounds),
        theme.input_background_color(),
        theme.input_box_radius(),
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

/// Paints the browser omnibox chrome bar using a custom [`OmniboxTheme`].
pub fn paint_omnibox_with_theme<F: FontProvider>(
    state: &OmniboxState,
    width: Px,
    font_provider: &F,
    theme: OmniboxTheme,
    builder: &mut DisplayListBuilder,
) -> Result<(), GraphicsError> {
    OmniboxPainter::new(font_provider)
        .with_theme(theme)
        .paint(state, width, builder)
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

//! [`SoftwareCpuBackend`] — the CPU rasterizer, and the reference every other
//! tier is measured against (`PRD-005:57`, roadmap point `I6`).
//!
//! Always available: no GPU, no driver, no window. That is what makes the tier
//! cascade's last rung real and **C-17** (`PRD-005:90`) meaningful, and what
//! lets the golden-image gate run on a CI runner with no display.
//!
//! ## What it implements, and what it refuses
//!
//! `DrawRect` (with rounded corners), `PushClip`/`PopClip`,
//! `PushOpacity`/`PopOpacity`, and (v0.5 B3) `DrawText`. `DrawImage` and
//! `DrawPath` report [`GraphicsError::Unsupported`] naming the command — the
//! contract is whole from `F4`, the implementation is incremental (v0.3 report
//! §2.3).
//!
//! ## `DrawText` (v0.5 B3)
//!
//! The backend never touches a font file or a curve — it holds an
//! [`Arc<dyn FontProvider>`](crate::application::FontProvider) and blits the
//! [`GlyphBitmap`](crate::domain::font::GlyphBitmap) each glyph resolves to,
//! the same clip/opacity/blend pipeline `DrawRect` already uses. `new()`
//! defaults to [`SyntheticFontProvider`] — deterministic, no filesystem — so
//! every existing caller keeps working without naming a font provider;
//! [`Self::with_font_provider`] swaps in a real one.
//!
//! [`SyntheticFontProvider`]: crate::infrastructure::font::SyntheticFontProvider
//!
//! ## Documented simplification: opacity is attenuation, not a layer
//!
//! A correct `PushOpacity` renders its contents into a separate buffer and
//! composites that buffer once. This backend instead multiplies each source
//! alpha by the accumulated opacity. The two agree whenever the contents of a
//! layer do not overlap each other, which is every case v0.3 produces — block
//! layout emits disjoint boxes. Where they disagree is overlapping content
//! inside one layer, and that is `F9`'s problem to raise, not something to
//! pretend is solved.

mod raster;

use std::sync::Arc;

use crate::application::FontProvider;
use crate::application::ports::RenderBackend;
use crate::domain::color::{Color, Opacity};
use crate::domain::command::DisplayCommand;
use crate::domain::display_list::DisplayList;
use crate::domain::error::{FrameOperation, FrameState, GraphicsError};
use crate::domain::font::{FontId, GlyphBitmap, GlyphRun};
use crate::domain::framebuffer::Framebuffer;
use crate::domain::geometry::{Point, Rect, Size, SurfaceSize};
use crate::domain::tier::BackendTier;
use crate::domain::unit::{AU_PER_PX, Au};
use crate::infrastructure::font::SyntheticFontProvider;

/// The colour a frame starts as: opaque white, the default canvas of a page.
const CANVAS: Color = Color::WHITE;

/// A CPU rasterizer with a clip stack, an opacity stack, and a bound font
/// provider (v0.5 B3).
pub struct SoftwareCpuBackend {
    state: FrameState,
    frame: Option<Framebuffer>,
    clips: Vec<Rect>,
    opacities: Vec<Opacity>,
    fonts: Arc<dyn FontProvider>,
}

impl core::fmt::Debug for SoftwareCpuBackend {
    // `dyn FontProvider` carries no `Debug` bound (a port trait, not a
    // diagnostic one); named as its registered-adapter count instead of
    // deriving, the same choice `TtfParserProvider`'s own `Debug` makes.
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("SoftwareCpuBackend")
            .field("state", &self.state)
            .field("frame", &self.frame)
            .field("clips", &self.clips)
            .field("opacities", &self.opacities)
            .finish_non_exhaustive()
    }
}

impl SoftwareCpuBackend {
    /// A backend with the deterministic [`SyntheticFontProvider`] bound — what
    /// every golden and conformance test wants, and a safe default for any
    /// caller that has not registered a real font.
    #[must_use]
    pub fn new() -> Self {
        Self::with_font_provider(Arc::new(SyntheticFontProvider::new()))
    }

    /// A backend bound to `fonts` — e.g. a [`TtfParserProvider`] or
    /// [`SystemFontProvider`] with real faces registered.
    ///
    /// [`TtfParserProvider`]: crate::infrastructure::font::TtfParserProvider
    /// [`SystemFontProvider`]: crate::infrastructure::font::SystemFontProvider
    #[must_use]
    pub fn with_font_provider(fonts: Arc<dyn FontProvider>) -> Self {
        Self {
            state: FrameState::Idle,
            frame: None,
            clips: Vec::new(),
            opacities: Vec::new(),
            fonts,
        }
    }

    /// Rasterizes one command into the frame in progress.
    fn draw(&mut self, command: &DisplayCommand) -> Result<(), GraphicsError> {
        match command {
            DisplayCommand::DrawRect {
                rect,
                color,
                corner_radius,
            } => self.fill_rect(*rect, *color, *corner_radius),
            DisplayCommand::DrawText {
                glyphs,
                color,
                font,
            } => self.draw_text(glyphs, *color, *font),
            DisplayCommand::PushClip { region } => {
                self.clips.push(*region);
                Ok(())
            }
            DisplayCommand::PopClip => {
                self.clips.pop();
                Ok(())
            }
            DisplayCommand::PushOpacity { opacity } => {
                self.opacities.push(*opacity);
                Ok(())
            }
            DisplayCommand::PopOpacity => {
                self.opacities.pop();
                Ok(())
            }
            other => Err(GraphicsError::Unsupported {
                tier: BackendTier::Software,
                command: other.kind(),
            }),
        }
    }

    /// Fills `rect` with `color`, clipped and attenuated by the current state.
    fn fill_rect(&mut self, rect: Rect, color: Color, radius: Au) -> Result<(), GraphicsError> {
        let source = color.faded(self.accumulated_opacity());
        let Some(area) = self.clipped(rect) else {
            return Ok(());
        };
        let Some(frame) = self.frame.as_mut() else {
            return Err(GraphicsError::FrameOutOfOrder {
                attempted: FrameOperation::Submit,
                state: self.state,
            });
        };
        paint_rect(frame, area, rect, source, radius);
        Ok(())
    }

    /// Rasterizes every glyph in `glyphs`, tinted by `color`.
    fn draw_text(
        &mut self,
        glyphs: &GlyphRun,
        color: Color,
        font: FontId,
    ) -> Result<(), GraphicsError> {
        for instance in glyphs {
            let bitmap = self.fonts.rasterize(font, instance.glyph())?;
            if bitmap.is_empty() {
                continue;
            }
            self.blit_glyph(&bitmap, instance.position(), color)?;
        }
        Ok(())
    }

    /// Blits one already-rasterized glyph at `pen` (its baseline position),
    /// clipped and attenuated by the current state — the same pipeline
    /// [`Self::fill_rect`] uses.
    fn blit_glyph(
        &mut self,
        bitmap: &GlyphBitmap,
        pen: Point,
        color: Color,
    ) -> Result<(), GraphicsError> {
        let origin = pen.translated(bitmap.bearing().horizontal(), bitmap.bearing().vertical());
        let Some(bounds) = glyph_bounds(origin, bitmap) else {
            return Ok(());
        };
        let source = color.faded(self.accumulated_opacity());
        let Some(area) = self.clipped(bounds) else {
            return Ok(());
        };
        let Some(frame) = self.frame.as_mut() else {
            return Err(GraphicsError::FrameOutOfOrder {
                attempted: FrameOperation::Submit,
                state: self.state,
            });
        };
        paint_glyph(frame, area, origin, bitmap, source);
        Ok(())
    }

    /// `rect` narrowed by every clip in force, or `None` when nothing survives.
    fn clipped(&self, rect: Rect) -> Option<Rect> {
        self.clips
            .iter()
            .try_fold(rect, |narrowed, clip| narrowed.intersection(*clip))
    }

    /// The product of every open opacity layer.
    fn accumulated_opacity(&self) -> Opacity {
        self.opacities
            .iter()
            .fold(Opacity::OPAQUE, |carried, layer| compose(carried, *layer))
    }

    /// Refuses `attempted` unless the backend is in `required`.
    fn require(
        &self,
        attempted: FrameOperation,
        required: FrameState,
    ) -> Result<(), GraphicsError> {
        if self.state == required {
            return Ok(());
        }
        Err(GraphicsError::FrameOutOfOrder {
            attempted,
            state: self.state,
        })
    }
}

/// The rounded 8-bit product of two opacities.
///
/// `(a * b + 127) / 255`, the same rounding rule `Color::faded` uses, so nesting
/// two 50% layers lands where multiplying two 50% colours does.
fn compose(first: Opacity, second: Opacity) -> Opacity {
    let product = u32::from(first.level())
        .saturating_mul(u32::from(second.level()))
        .saturating_add(127)
        .checked_div(u32::from(u8::MAX))
        .unwrap_or(0);
    Opacity::from_level(u8::try_from(product).unwrap_or(u8::MAX))
}

/// The whole surface as a rectangle, for clipping a fill to the buffer.
fn surface_bounds(size: SurfaceSize) -> Option<Rect> {
    let width = Au::from_whole_px(i32::try_from(size.width()).ok()?)?;
    let height = Au::from_whole_px(i32::try_from(size.height()).ok()?)?;
    Some(Rect::new(Point::ORIGIN, Size::new(width, height)?))
}

/// Blends `source` into every pixel `area` touches.
///
/// `area` is the clipped region that bounds the loop; `shape` is the *original*
/// rectangle, because coverage and corner rounding must be measured against the
/// shape the author asked for, not against the clip that happens to cut it.
fn paint_rect(frame: &mut Framebuffer, area: Rect, shape: Rect, source: Color, radius: Au) {
    let (first_column, last_column) =
        raster::pixel_range(area.min_x(), area.max_x(), frame.width());
    let (first_row, last_row) = raster::pixel_range(area.min_y(), area.max_y(), frame.height());
    for row in first_row..last_row {
        for column in first_column..last_column {
            paint_pixel(frame, area, shape, source, radius, column, row);
        }
    }
}

/// Blends one pixel, if any of it is covered.
fn paint_pixel(
    frame: &mut Framebuffer,
    area: Rect,
    shape: Rect,
    source: Color,
    radius: Au,
    column: u32,
    row: u32,
) {
    let clipped = raster::rect_coverage(area, column, row);
    if clipped == 0 {
        return;
    }
    let rounded = raster::corner_coverage(shape, radius, column, row);
    let coverage = scale_by_full(clipped, rounded);
    let Some(destination) = frame.pixel(column, row) else {
        return;
    };
    frame.set_pixel(
        column,
        row,
        raster::blend_over(destination, source, coverage),
    );
}

/// Combines two `0..=4096` coverages into one.
fn scale_by_full(first: u32, second: u32) -> u32 {
    first
        .saturating_mul(second)
        .checked_div(raster::FULL_COVERAGE)
        .unwrap_or(0)
}

impl Default for SoftwareCpuBackend {
    fn default() -> Self {
        Self::new()
    }
}

/// The rectangle `bitmap` occupies on the surface, top-left at `origin`.
fn glyph_bounds(origin: Point, bitmap: &GlyphBitmap) -> Option<Rect> {
    let width = Au::from_whole_px(i32::try_from(bitmap.width()).ok()?)?;
    let height = Au::from_whole_px(i32::try_from(bitmap.height()).ok()?)?;
    Some(Rect::new(origin, Size::new(width, height)?))
}

/// The whole-pixel index `value` falls in.
fn pixel_index(value: Au) -> i64 {
    i64::from(value.raw()).div_euclid(i64::from(AU_PER_PX))
}

/// Blits `bitmap`'s coverage into `frame`, clipped to `area`, tinted by
/// `source`. `origin` is `bitmap`'s unclipped top-left corner, needed to map a
/// destination pixel back to a bitmap cell.
fn paint_glyph(
    frame: &mut Framebuffer,
    area: Rect,
    origin: Point,
    bitmap: &GlyphBitmap,
    source: Color,
) {
    let (first_column, last_column) =
        raster::pixel_range(area.min_x(), area.max_x(), frame.width());
    let (first_row, last_row) = raster::pixel_range(area.min_y(), area.max_y(), frame.height());
    let origin_column = pixel_index(origin.horizontal());
    let origin_row = pixel_index(origin.vertical());
    for row in first_row..last_row {
        for column in first_column..last_column {
            paint_glyph_pixel(
                frame,
                origin_column,
                origin_row,
                bitmap,
                source,
                column,
                row,
            );
        }
    }
}

/// Blends one destination pixel against the bitmap cell it maps to, if any of
/// it is covered.
fn paint_glyph_pixel(
    frame: &mut Framebuffer,
    origin_column: i64,
    origin_row: i64,
    bitmap: &GlyphBitmap,
    source: Color,
    column: u32,
    row: u32,
) {
    let Some(cell_column) = bitmap_cell(column, origin_column) else {
        return;
    };
    let Some(cell_row) = bitmap_cell(row, origin_row) else {
        return;
    };
    let coverage_byte = bitmap.coverage_at(cell_column, cell_row);
    if coverage_byte == 0 {
        return;
    }
    let coverage = scale_byte_to_full(coverage_byte);
    let Some(destination) = frame.pixel(column, row) else {
        return;
    };
    frame.set_pixel(
        column,
        row,
        raster::blend_over(destination, source, coverage),
    );
}

/// `destination_pixel - origin_pixel`, as a bitmap cell index — `None` when
/// the destination pixel lies before the bitmap's own origin.
fn bitmap_cell(destination: u32, origin: i64) -> Option<u32> {
    i64::from(destination)
        .checked_sub(origin)
        .and_then(|value| u32::try_from(value).ok())
}

/// Rescales a `0..=255` byte coverage onto `0..=4096`.
fn scale_byte_to_full(byte: u8) -> u32 {
    u32::from(byte)
        .saturating_mul(raster::FULL_COVERAGE)
        .checked_div(u32::from(u8::MAX))
        .unwrap_or(0)
}

impl RenderBackend for SoftwareCpuBackend {
    fn tier(&self) -> BackendTier {
        BackendTier::Software
    }

    fn begin_frame(&mut self, size: SurfaceSize) -> Result<(), GraphicsError> {
        // Valid from `Idle` and from `Presented`: a backend is reusable, and a
        // new frame after a read-back is the normal case. Only a nested frame is
        // refused.
        if self.state == FrameState::Recording {
            return Err(GraphicsError::FrameOutOfOrder {
                attempted: FrameOperation::BeginFrame,
                state: self.state,
            });
        }
        self.frame = Some(Framebuffer::filled(size, CANVAS).ok_or(
            GraphicsError::ReadbackFailed {
                tier: BackendTier::Software,
            },
        )?);
        self.clips.clear();
        self.opacities.clear();
        if let Some(bounds) = surface_bounds(size) {
            self.clips.push(bounds);
        }
        self.state = FrameState::Recording;
        Ok(())
    }

    fn submit(&mut self, list: &DisplayList) -> Result<(), GraphicsError> {
        self.require(FrameOperation::Submit, FrameState::Recording)?;
        for command in list {
            self.draw(command)?;
        }
        Ok(())
    }

    fn end_frame(&mut self) -> Result<(), GraphicsError> {
        self.require(FrameOperation::EndFrame, FrameState::Recording)?;
        self.state = FrameState::Presented;
        Ok(())
    }

    fn read_back(&self) -> Result<Framebuffer, GraphicsError> {
        self.require(FrameOperation::ReadBack, FrameState::Presented)?;
        self.frame.clone().ok_or(GraphicsError::ReadbackFailed {
            tier: BackendTier::Software,
        })
    }
}

//! [`SoftwareCpuBackend`] — the CPU rasterizer, and the reference every other
//! tier is measured against (`PRD-005:57`, roadmap point `I6`).
//!
//! Always available: no GPU, no driver, no window. That is what makes the tier
//! cascade's last rung real and **C-17** (`PRD-005:90`) meaningful, and what
//! lets the golden-image gate run on a CI runner with no display.
//!
//! ## What it implements, and what it refuses
//!
//! `DrawRect` (with rounded corners), `PushClip`/`PopClip` and
//! `PushOpacity`/`PopOpacity`. `DrawText` arrives in `F4b` with the glyph
//! rasterizer; `DrawImage` and `DrawPath` report
//! [`GraphicsError::Unsupported`] naming the command — the contract is whole
//! from `F4`, the implementation is incremental (v0.3 report §2.3).
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

use crate::application::ports::RenderBackend;
use crate::domain::color::{Color, Opacity};
use crate::domain::command::DisplayCommand;
use crate::domain::display_list::DisplayList;
use crate::domain::error::{FrameOperation, FrameState, GraphicsError};
use crate::domain::framebuffer::Framebuffer;
use crate::domain::geometry::{Point, Rect, Size, SurfaceSize};
use crate::domain::tier::BackendTier;
use crate::domain::unit::Au;

/// The colour a frame starts as: opaque white, the default canvas of a page.
const CANVAS: Color = Color::WHITE;

/// A CPU rasterizer with a clip stack and an opacity stack.
#[derive(Debug)]
pub struct SoftwareCpuBackend {
    state: FrameState,
    frame: Option<Framebuffer>,
    clips: Vec<Rect>,
    opacities: Vec<Opacity>,
}

impl SoftwareCpuBackend {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            state: FrameState::Idle,
            frame: None,
            clips: Vec::new(),
            opacities: Vec::new(),
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

//! [`DisplayListBuilder`] — the only way to construct a
//! [`DisplayList`], and therefore the one boundary where a
//! malformed command can be stopped (`PRD-005:79-81`).
//!
//! ## The two rules, and why they are different
//!
//! Mixing them is the common error, so they are stated once, here (v0.3 report
//! §2.3):
//!
//! | Input                                        | Rule       | Why                                                        |
//! | -------------------------------------------- | ---------- | ---------------------------------------------------------- |
//! | `NaN`, `±inf`                                | **Refuse** | No correct reading; substituting a number hides the defect  |
//! | Negative width or height                     | **Refuse** | The caller computed something wrong                         |
//! | `Pop*` with no matching `Push*`              | **Refuse** | An unbalanced stack corrupts every later command            |
//! | A scope still open at [`DisplayListBuilder::build`] | **Refuse** | Same reason                                          |
//! | Finite but past `Au::MAX_EXTENT`             | **Clamp**  | A legitimate page has a giant box; refusing breaks the page |
//! | `Opacity` outside `[0, 1]`                   | **Clamp**  | `1.5` plainly means "opaque"                                |
//!
//! Because the builder is the only place that accepts [`Px`], the finiteness
//! check happens **exactly once**, inside [`Au::from_px`] — not once per
//! command, and never again downstream.

use crate::domain::color::{Color, Opacity};
use crate::domain::command::DisplayCommand;
use crate::domain::command_index::CommandIndex;
use crate::domain::display_list::DisplayList;
use crate::domain::error::{CommandRejection, GraphicsError};
use crate::domain::font::{FontId, GlyphRun};
use crate::domain::geometry::{Point, Rect, Size};
use crate::domain::image::ImageId;
use crate::domain::path::{Path, Stroke};
use crate::domain::unit::{Au, Px};

/// A rectangle as an author supplies it: four unvalidated lengths.
///
/// The request shape, not a domain type — it exists so the builder has exactly
/// one place to convert author input into geometry, and so a caller writes
/// `PxRect::new(x, y, width, height)` rather than passing four bare `f32`s.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PxRect {
    left: Px,
    top: Px,
    width: Px,
    height: Px,
}

impl PxRect {
    #[must_use]
    pub const fn new(left: Px, top: Px, width: Px, height: Px) -> Self {
        Self {
            left,
            top,
            width,
            height,
        }
    }

    /// The same four lengths from plain numbers, for call sites that have them
    /// as literals.
    #[must_use]
    pub const fn from_px(left: f32, top: f32, width: f32, height: f32) -> Self {
        Self::new(Px::new(left), Px::new(top), Px::new(width), Px::new(height))
    }
}

/// Which kind of scope a `Push*` opened, so the matching `Pop*` can be checked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OpenScope {
    Clip,
    Opacity,
}

impl OpenScope {
    /// The rejection to report when the list ends with this scope still open.
    const fn left_open(self) -> CommandRejection {
        match self {
            Self::Clip => CommandRejection::ClipLeftOpen,
            Self::Opacity => CommandRejection::OpacityLeftOpen,
        }
    }
}

/// Accumulates sanitized commands and enforces a balanced state stack.
#[derive(Debug, Default)]
pub struct DisplayListBuilder {
    commands: Vec<DisplayCommand>,
    open_scopes: Vec<OpenScope>,
}

impl DisplayListBuilder {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            commands: Vec::new(),
            open_scopes: Vec::new(),
        }
    }

    /// How many commands have been accepted so far.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.commands.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// How many scopes are still open — zero is the only state
    /// [`Self::build`] accepts.
    #[must_use]
    pub const fn open_scope_count(&self) -> usize {
        self.open_scopes.len()
    }

    // ---- drawing -----------------------------------------------------------

    /// A filled rectangle with square corners.
    pub fn draw_rect(&mut self, area: PxRect, color: Color) -> Result<(), GraphicsError> {
        self.draw_rounded_rect(area, color, Px::new(0.0))
    }

    /// A filled rectangle with rounded corners.
    ///
    /// A separate method rather than an `Option` parameter: two named methods
    /// read better than one that asks the caller to spell "no radius"
    /// (`ADR-0010`, no flag parameters).
    pub fn draw_rounded_rect(
        &mut self,
        area: PxRect,
        color: Color,
        corner_radius: Px,
    ) -> Result<(), GraphicsError> {
        let rect = self.rect_from(area)?;
        let radius = self.length_from(corner_radius)?;
        self.accept(DisplayCommand::DrawRect {
            rect,
            color,
            corner_radius: radius,
        });
        Ok(())
    }

    /// A run of glyphs already positioned in [`Au`] by whoever laid the text
    /// out. No conversion is needed: the coordinates never were floats.
    pub fn draw_text(
        &mut self,
        glyphs: GlyphRun,
        color: Color,
        font: FontId,
    ) -> Result<(), GraphicsError> {
        self.accept(DisplayCommand::DrawText {
            glyphs,
            color,
            font,
        });
        Ok(())
    }

    /// A decoded image. Accepted by the builder, refused by the v0.3 backend.
    pub fn draw_image(
        &mut self,
        image: ImageId,
        source: PxRect,
        destination: PxRect,
    ) -> Result<(), GraphicsError> {
        let source = self.rect_from(source)?;
        let destination = self.rect_from(destination)?;
        self.accept(DisplayCommand::DrawImage {
            image,
            source,
            destination,
        });
        Ok(())
    }

    /// A vector outline. Accepted by the builder, refused by the v0.3 backend.
    pub fn draw_path(
        &mut self,
        path: Path,
        fill: Option<Color>,
        stroke: Option<Stroke>,
    ) -> Result<(), GraphicsError> {
        self.accept(DisplayCommand::DrawPath { path, fill, stroke });
        Ok(())
    }

    // ---- state scopes ------------------------------------------------------

    pub fn push_clip(&mut self, region: PxRect) -> Result<(), GraphicsError> {
        let region = self.rect_from(region)?;
        self.open_scopes.push(OpenScope::Clip);
        self.accept(DisplayCommand::PushClip { region });
        Ok(())
    }

    pub fn pop_clip(&mut self) -> Result<(), GraphicsError> {
        self.close(OpenScope::Clip, CommandRejection::ClipPopWithoutPush)?;
        self.accept(DisplayCommand::PopClip);
        Ok(())
    }

    /// Opens a composited layer. A non-finite opacity is refused; a finite one
    /// outside `[0, 1]` is clamped.
    pub fn push_opacity(&mut self, opacity: f32) -> Result<(), GraphicsError> {
        let opacity = Opacity::from_unit_interval(opacity)
            .ok_or_else(|| self.reject(CommandRejection::NonFiniteCoordinate))?;
        self.open_scopes.push(OpenScope::Opacity);
        self.accept(DisplayCommand::PushOpacity { opacity });
        Ok(())
    }

    pub fn pop_opacity(&mut self) -> Result<(), GraphicsError> {
        self.close(OpenScope::Opacity, CommandRejection::OpacityPopWithoutPush)?;
        self.accept(DisplayCommand::PopOpacity);
        Ok(())
    }

    // ---- completion --------------------------------------------------------

    /// Seals the list, or refuses it when a scope was left open.
    ///
    /// Refusing the whole list rather than closing the scope silently: a caller
    /// that forgot a `PopClip` has a bug in its painting logic, and papering
    /// over it would produce a plausible-looking wrong picture.
    pub fn build(self) -> Result<DisplayList, GraphicsError> {
        match self.open_scopes.last() {
            Some(scope) => Err(GraphicsError::InvalidCommand {
                index: CommandIndex::from_position(self.commands.len()),
                reason: scope.left_open(),
            }),
            None => Ok(DisplayList::from_sanitized(self.commands)),
        }
    }

    // ---- the sanitizing boundary -------------------------------------------

    /// Converts one author length, refusing a non-finite one.
    ///
    /// This and [`Self::rect_from`] are the *only* callers of
    /// [`Au::from_px`] in the crate, which is what makes "the check happens
    /// exactly once" a structural fact rather than a convention.
    fn length_from(&self, length: Px) -> Result<Au, GraphicsError> {
        Au::from_px(length).ok_or_else(|| self.reject(CommandRejection::NonFiniteCoordinate))
    }

    /// Converts an author rectangle: refuses non-finite coordinates and negative
    /// extents, clamps anything finite that overruns the envelope.
    fn rect_from(&self, area: PxRect) -> Result<Rect, GraphicsError> {
        let left = self.length_from(area.left)?;
        let top = self.length_from(area.top)?;
        let width = self.length_from(area.width)?;
        let height = self.length_from(area.height)?;
        let size = Size::new(width, height)
            .ok_or_else(|| self.reject(CommandRejection::NegativeExtent))?;
        Ok(Rect::new(Point::new(left, top), size))
    }

    /// Pops `expected` off the scope stack, or reports `rejection`.
    fn close(
        &mut self,
        expected: OpenScope,
        rejection: CommandRejection,
    ) -> Result<(), GraphicsError> {
        if self.open_scopes.last() != Some(&expected) {
            return Err(self.reject(rejection));
        }
        self.open_scopes.pop();
        Ok(())
    }

    /// Records a command that has already passed the boundary.
    fn accept(&mut self, command: DisplayCommand) {
        self.commands.push(command);
    }

    /// The error for a command refused at the position it would have occupied.
    fn reject(&self, reason: CommandRejection) -> GraphicsError {
        GraphicsError::InvalidCommand {
            index: CommandIndex::from_position(self.commands.len()),
            reason,
        }
    }
}

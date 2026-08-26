use crate::domain::geometry::Rect;
use css::Color;

/// Declarative 2D render instructions emitted into a DisplayList (PRD-005:60-72).
#[derive(Debug, Clone, PartialEq)]
pub enum RenderCommand {
    /// Fills the surface background with a solid color.
    Clear(Color),
    /// Draws a solid colored rectangle.
    DrawRect { rect: Rect, color: Color },
    /// Draws a rectangular border outline.
    DrawBorder {
        rect: Rect,
        color: Color,
        width: f32,
    },
    /// Draws a text string within a bounding box.
    DrawText {
        text: String,
        rect: Rect,
        color: Color,
        font_size: f32,
    },
}

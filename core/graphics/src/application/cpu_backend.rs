use crate::domain::backend::RenderBackend;
use crate::domain::command::RenderCommand;
use crate::domain::display_list::DisplayList;
use crate::domain::error::GraphicsError;
use crate::domain::geometry::{Position, Rect};
use css::Color;
use std::path::Path;

/// CPU-based pixel buffer renderer for headless environments and software fallback (PRD-005:90, C-17).
pub struct SoftwareCpuBackend {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

impl SoftwareCpuBackend {
    /// Creates a new `SoftwareCpuBackend` with the given dimensions.
    #[must_use]
    pub fn new(width: u32, height: u32) -> Self {
        let pixel_count = (width as usize) * (height as usize) * 4;
        Self {
            width,
            height,
            pixels: vec![255; pixel_count],
        }
    }

    fn put_pixel(&mut self, pos: Position, color: Color) {
        if pos.x() >= self.width || pos.y() >= self.height {
            return;
        }

        let idx = ((pos.y() as usize) * (self.width as usize) + (pos.x() as usize)) * 4;
        let alpha = color.a() as f32 / 255.0;

        if alpha >= 1.0 {
            self.pixels[idx] = color.r();
            self.pixels[idx + 1] = color.g();
            self.pixels[idx + 2] = color.b();
            self.pixels[idx + 3] = color.a();
            return;
        }

        if alpha <= 0.0 {
            return;
        }

        // Alpha blend over existing pixel
        let inv_a = 1.0 - alpha;
        self.pixels[idx] = ((color.r() as f32 * alpha) + (self.pixels[idx] as f32 * inv_a)) as u8;
        self.pixels[idx + 1] =
            ((color.g() as f32 * alpha) + (self.pixels[idx + 1] as f32 * inv_a)) as u8;
        self.pixels[idx + 2] =
            ((color.b() as f32 * alpha) + (self.pixels[idx + 2] as f32 * inv_a)) as u8;
        self.pixels[idx + 3] = 255;
    }

    fn clear_surface(&mut self, color: Color) {
        for y in 0..self.height {
            for x in 0..self.width {
                let idx = ((y as usize) * (self.width as usize) + (x as usize)) * 4;
                self.pixels[idx] = color.r();
                self.pixels[idx + 1] = color.g();
                self.pixels[idx + 2] = color.b();
                self.pixels[idx + 3] = color.a();
            }
        }
    }

    fn draw_filled_rect(&mut self, rect: Rect, color: Color) {
        let x_start = rect.x().max(0.0) as u32;
        let y_start = rect.y().max(0.0) as u32;
        let x_end = rect.right().min(self.width as f32).max(0.0) as u32;
        let y_end = rect.bottom().min(self.height as f32).max(0.0) as u32;

        for y in y_start..y_end {
            for x in x_start..x_end {
                self.put_pixel(Position::new(x, y), color);
            }
        }
    }

    fn draw_outline_border(&mut self, rect: Rect, color: Color, border_w: f32) {
        let bw = border_w.max(1.0);
        // Top
        self.draw_filled_rect(Rect::new(rect.x(), rect.y(), rect.width(), bw), color);
        // Bottom
        self.draw_filled_rect(
            Rect::new(rect.x(), rect.bottom() - bw, rect.width(), bw),
            color,
        );
        // Left
        self.draw_filled_rect(Rect::new(rect.x(), rect.y(), bw, rect.height()), color);
        // Right
        self.draw_filled_rect(
            Rect::new(rect.right() - bw, rect.y(), bw, rect.height()),
            color,
        );
    }

    fn draw_simple_text(&mut self, text: &str, rect: Rect, color: Color, font_size: f32) {
        // Draw shaded text indicator area for headless rasterization
        if text.trim().is_empty() {
            return;
        }

        let line_height = font_size.max(10.0);
        let bar_width = ((text.len() as f32) * (font_size * 0.5)).min(rect.width());
        let text_bar = Rect::new(rect.x(), rect.y() + 2.0, bar_width, line_height * 0.7);
        self.draw_filled_rect(text_bar, color);
    }
}

impl RenderBackend for SoftwareCpuBackend {
    fn name(&self) -> &'static str {
        "SoftwareCpuBackend"
    }

    fn render(&mut self, list: &DisplayList) -> Result<(), GraphicsError> {
        for cmd in list.commands() {
            match cmd {
                RenderCommand::Clear(color) => self.clear_surface(*color),
                RenderCommand::DrawRect { rect, color } => self.draw_filled_rect(*rect, *color),
                RenderCommand::DrawBorder { rect, color, width } => {
                    self.draw_outline_border(*rect, *color, *width);
                }
                RenderCommand::DrawText {
                    text,
                    rect,
                    color,
                    font_size,
                } => {
                    self.draw_simple_text(text, *rect, *color, *font_size);
                }
            }
        }
        Ok(())
    }

    fn save_png(&self, path: &Path) -> Result<(), GraphicsError> {
        image::save_buffer(
            path,
            &self.pixels,
            self.width,
            self.height,
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|e| GraphicsError::EncodingError(e.to_string()))
    }

    fn to_rgba_bytes(&self) -> Result<Vec<u8>, GraphicsError> {
        Ok(self.pixels.clone())
    }

    fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

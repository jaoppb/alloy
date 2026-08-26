use crate::domain::display_list::DisplayList;
use crate::domain::error::GraphicsError;
use std::path::Path;

/// Pure abstract contract for graphics presentation backends (PRD-005:87, C-14).
pub trait RenderBackend: Send + Sync {
    /// Human-readable name of the backend (e.g. "SoftwareCpuBackend", "VulkanBackend").
    fn name(&self) -> &'static str;

    /// Renders the complete display list onto the target surface or buffer.
    fn render(&mut self, list: &DisplayList) -> Result<(), GraphicsError>;

    /// Encodes and saves the current rendered frame as a PNG file.
    fn save_png(&self, path: &Path) -> Result<(), GraphicsError>;

    /// Returns the raw RGBA8 frame bytes.
    fn to_rgba_bytes(&self) -> Result<Vec<u8>, GraphicsError>;

    /// Returns the surface dimensions (width, height) in pixels.
    fn dimensions(&self) -> (u32, u32);
}

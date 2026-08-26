use crate::application::cpu_backend::SoftwareCpuBackend;
use crate::domain::backend::RenderBackend;

/// Multi-tier graphics backend factory ensuring automated software fallback (PRD-005:30-58, C-17).
pub struct GraphicsBackendFactory;

impl GraphicsBackendFactory {
    /// Creates a headless rendering backend with automatic software fallback (PRD-005:90, C-17).
    #[must_use]
    pub fn create_headless(width: u32, height: u32) -> Box<dyn RenderBackend> {
        // In headless environments, automatically return the guaranteed CPU software backend
        Box::new(SoftwareCpuBackend::new(width, height))
    }
}

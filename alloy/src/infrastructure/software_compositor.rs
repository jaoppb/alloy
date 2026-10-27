//! The CPU raster tier (`ADR-0009`) as the compositor's two backends.

use std::sync::Arc;

use graphics::{FontProvider, SoftwareCpuBackend};

use crate::application::compositor::Compositor;

/// A compositor over two [`SoftwareCpuBackend`]s sharing `fonts`.
///
/// One backend rasters the chrome, one the page. They are separate instances because the page
/// backend's images follow the document (`RenderBackend::bind_images`).
#[must_use]
pub fn software_compositor<F: FontProvider>(fonts: &Arc<F>) -> Compositor<SoftwareCpuBackend<F>> {
    Compositor::new(
        SoftwareCpuBackend::with_font_provider(Arc::clone(fonts)),
        SoftwareCpuBackend::with_font_provider(Arc::clone(fonts)),
    )
}

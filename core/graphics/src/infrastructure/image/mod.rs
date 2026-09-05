//! In-memory and empty [`crate::application::ImageProvider`] adapters (v0.5 Phase X).

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::application::ImageProvider;
use crate::domain::error::GraphicsError;
use crate::domain::framebuffer::Framebuffer;
use crate::domain::image::ImageId;

/// An immutable in-memory store of pre-decoded [`Framebuffer`]s.
///
/// Registration is a consuming builder (`with_image`), so the provider is
/// immutable once built: no lock is needed to satisfy `ImageProvider: Send + Sync`,
/// exactly mirroring [`crate::infrastructure::font::TtfParserProvider`].
#[derive(Clone, Debug, Default)]
pub struct InMemoryImageProvider {
    images: BTreeMap<ImageId, Arc<Framebuffer>>,
}

impl InMemoryImageProvider {
    /// Creates a new, empty image store.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            images: BTreeMap::new(),
        }
    }

    /// Registers `frame` under `id`.
    #[must_use]
    pub fn with_image(mut self, id: ImageId, frame: Framebuffer) -> Self {
        self.images.insert(id, Arc::new(frame));
        self
    }
}

impl ImageProvider for InMemoryImageProvider {
    fn get(&self, image: ImageId) -> Result<Arc<Framebuffer>, GraphicsError> {
        self.images
            .get(&image)
            .cloned()
            .ok_or(GraphicsError::ImageUnavailable { image })
    }
}

/// An image provider that holds no images and always reports [`GraphicsError::ImageUnavailable`].
#[derive(Clone, Copy, Debug, Default)]
pub struct EmptyImageProvider;

impl ImageProvider for EmptyImageProvider {
    fn get(&self, image: ImageId) -> Result<Arc<Framebuffer>, GraphicsError> {
        Err(GraphicsError::ImageUnavailable { image })
    }
}

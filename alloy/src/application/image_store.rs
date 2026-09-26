//! The decoded images a live page currently knows about.

use std::collections::HashMap;
use std::sync::Arc;

use graphics::{Framebuffer, GraphicsError, ImageId, ImageProvider};

use crate::application::subresource::placeholder_framebuffer;

/// Decoded images by [`ImageId`], shared by pointer so a relayout snapshots
/// them without copying pixel buffers.
///
/// Wraps the map so the loop never manipulates the backing collection
/// directly (CLAUDE.md: first-class collections, no naked primitives).
#[derive(Clone, Debug, Default)]
pub struct ImageStore {
    frames: HashMap<ImageId, Arc<Framebuffer>>,
}

impl ImageStore {
    /// An empty store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Records the decoded pixels for `id`, replacing any placeholder.
    pub fn insert(&mut self, id: ImageId, frame: Framebuffer) {
        self.frames.insert(id, Arc::new(frame));
    }

    /// Registers a placeholder for `id` unless a frame is already known — a
    /// discovered image must resolve to *something* from the first paint.
    pub fn reserve_placeholder(&mut self, id: ImageId) {
        self.frames
            .entry(id)
            .or_insert_with(|| Arc::new(placeholder_framebuffer()));
    }

    /// Forgets every image (a new navigation starts from nothing).
    pub fn clear(&mut self) {
        self.frames.clear();
    }

    /// Whether `id` has a frame (placeholder or decoded).
    #[must_use]
    pub fn contains(&self, id: ImageId) -> bool {
        self.frames.contains_key(&id)
    }

    /// An immutable [`ImageProvider`] over the current contents.
    #[must_use]
    pub fn snapshot(&self) -> StoreSnapshot {
        StoreSnapshot {
            frames: self.frames.clone(),
            placeholder: Arc::new(placeholder_framebuffer()),
        }
    }
}

/// A point-in-time [`ImageProvider`] built by [`ImageStore::snapshot`]; an id
/// with no entry resolves to the transparent placeholder, never an error.
#[derive(Clone, Debug)]
pub struct StoreSnapshot {
    frames: HashMap<ImageId, Arc<Framebuffer>>,
    placeholder: Arc<Framebuffer>,
}

impl ImageProvider for StoreSnapshot {
    fn get(&self, image: ImageId) -> Result<Arc<Framebuffer>, GraphicsError> {
        let frame = self.frames.get(&image).unwrap_or(&self.placeholder);
        Ok(Arc::clone(frame))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use graphics::{Color, Framebuffer, ImageId, ImageProvider, SurfaceSize};

    use super::ImageStore;

    fn red_pixel() -> Framebuffer {
        let size = SurfaceSize::new(1, 1).unwrap();
        Framebuffer::filled(size, Color::rgb(255, 0, 0)).unwrap()
    }

    #[test]
    fn a_placeholder_never_overwrites_a_decoded_frame() {
        let id = ImageId::new(7);
        let mut store = ImageStore::new();
        store.insert(id, red_pixel());
        store.reserve_placeholder(id);
        let frame = store.snapshot().get(id).unwrap();
        assert_eq!(frame.as_rgba8(), red_pixel().as_rgba8());
    }

    #[test]
    fn an_unknown_id_resolves_to_the_placeholder_instead_of_failing() {
        let snapshot = ImageStore::new().snapshot();
        assert!(snapshot.get(ImageId::new(1)).is_ok());
    }

    #[test]
    fn clear_forgets_every_image() {
        let id = ImageId::new(3);
        let mut store = ImageStore::new();
        store.reserve_placeholder(id);
        store.clear();
        assert!(!store.contains(id));
    }
}

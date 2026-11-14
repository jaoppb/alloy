//! The decoded images a live page currently knows about (moved from `alloy` by #111).

use std::collections::HashMap;
use std::sync::Arc;

use css::SnapshotId;
use graphics::{Color, Framebuffer, GraphicsError, ImageId, ImageProvider, SurfaceSize};

/// The [`ImageId`] of the `<img>` at `node`: its node's index. The one place
/// the key is made — the discoverer requests under it, layout's natural-size
/// lookup and the painter look it up (issue #83).
#[must_use]
pub fn image_id_of(node: SnapshotId) -> ImageId {
    ImageId::new(u32::try_from(node.index()).unwrap_or(u32::MAX))
}

/// The images a page references, by [`ImageId`]: still loading, decoded —
/// shared by pointer so a relayout snapshots them without copying pixel
/// buffers — or failed (issue #83).
///
/// Wraps the map so no caller manipulates the backing collection
/// directly (CLAUDE.md: first-class collections, no naked primitives).
#[derive(Clone, Debug, Default)]
pub struct ImageStore {
    images: HashMap<ImageId, StoredImage>,
}

/// One image's entry.
#[derive(Clone, Debug)]
enum StoredImage {
    Loading,
    Decoded(Arc<Framebuffer>),
    Failed,
}

/// What has become of one image a page references — what layout needs to
/// size an `<img>` and decide whether it shows its `alt` text (issue #83).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageState {
    /// Discovered and requested; no answer yet.
    Loading,
    /// Decoded, with the image's pixel size.
    Decoded(SurfaceSize),
    /// The fetch or the decode failed; it will not be retried.
    Failed,
}

impl StoredImage {
    fn state(&self) -> ImageState {
        match self {
            Self::Loading => ImageState::Loading,
            Self::Decoded(frame) => ImageState::Decoded(frame.size()),
            Self::Failed => ImageState::Failed,
        }
    }

    fn frame(&self) -> Option<Arc<Framebuffer>> {
        match self {
            Self::Decoded(frame) => Some(Arc::clone(frame)),
            Self::Loading | Self::Failed => None,
        }
    }
}

impl ImageStore {
    /// An empty store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Records the decoded pixels for `id`, whatever was known before.
    pub fn insert(&mut self, id: ImageId, frame: Framebuffer) {
        self.images
            .insert(id, StoredImage::Decoded(Arc::new(frame)));
    }

    /// Marks `id` as loading unless anything is already known about it, so
    /// layout tells a requested image from an unavailable one: it waits for the
    /// first instead of showing its `alt` text.
    pub fn mark_loading(&mut self, id: ImageId) {
        self.images.entry(id).or_insert(StoredImage::Loading);
    }

    /// Records that `id` will never be decoded: its fetch or its decode
    /// failed. Decoded pixels are never thrown away for it.
    pub fn mark_failed(&mut self, id: ImageId) {
        let entry = self.images.entry(id).or_insert(StoredImage::Failed);
        if matches!(entry, StoredImage::Decoded(_)) {
            return;
        }
        *entry = StoredImage::Failed;
    }

    /// Forgets every image (a new navigation starts from nothing).
    pub fn clear(&mut self) {
        self.images.clear();
    }

    /// What has become of `id`, or `None` for an image this page never
    /// referenced.
    #[must_use]
    pub fn state(&self, id: ImageId) -> Option<ImageState> {
        self.images.get(&id).map(StoredImage::state)
    }

    /// An immutable [`ImageProvider`] over the decoded images; every other id
    /// resolves to the placeholder.
    #[must_use]
    pub fn snapshot(&self) -> StoreSnapshot {
        let frames = self
            .images
            .iter()
            .filter_map(|(id, stored)| stored.frame().map(|frame| (*id, frame)))
            .collect();
        StoreSnapshot {
            frames,
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

/// A 1×1, fully transparent placeholder.
///
/// The painter emits a `DrawImage` command only for a decoded image, but a
/// backend resolves ids against a snapshot taken at another moment than the
/// display list was built: any id must resolve to *something*, never an
/// error, or one late or superseded image would fail the whole frame.
// "genuinely impossible state" `.expect()` carve-out CLAUDE.md documents:
// 1x1 is a compile-time-known-valid, non-zero surface size (same pattern as
// core/css/src/infrastructure/ua_sheet.rs). Private: an impossible-state
// `.expect()` is only defensible for a module's own callers.
#[allow(clippy::expect_used)]
#[must_use]
fn placeholder_framebuffer() -> Framebuffer {
    let size = SurfaceSize::new(1, 1).expect("1×1 is always a valid, non-zero surface size");
    Framebuffer::filled(size, Color::TRANSPARENT)
        .expect("a freshly built 1×1 framebuffer always fits its own pixel buffer")
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use graphics::{Color, Framebuffer, ImageId, ImageProvider, SurfaceSize};

    use super::{ImageState, ImageStore};

    fn red_pixel() -> Framebuffer {
        let size = SurfaceSize::new(1, 1).unwrap();
        Framebuffer::filled(size, Color::rgb(255, 0, 0)).unwrap()
    }

    #[test]
    fn marking_loading_never_overwrites_a_decoded_frame() {
        let id = ImageId::new(7);
        let mut store = ImageStore::new();
        store.insert(id, red_pixel());
        store.mark_loading(id);
        let frame = store.snapshot().get(id).unwrap();
        assert_eq!(frame.as_rgba8(), red_pixel().as_rgba8());
    }

    #[test]
    fn an_unknown_id_resolves_to_the_placeholder_instead_of_failing() {
        let snapshot = ImageStore::new().snapshot();
        assert!(snapshot.get(ImageId::new(1)).is_ok());
    }

    #[test]
    fn a_reserved_image_is_loading_until_decoded() {
        let id = ImageId::new(2);
        let mut store = ImageStore::new();
        store.mark_loading(id);
        assert_eq!(store.state(id), Some(ImageState::Loading));
        store.insert(id, red_pixel());
        let size = SurfaceSize::new(1, 1).unwrap();
        assert_eq!(store.state(id), Some(ImageState::Decoded(size)));
    }

    #[test]
    fn a_failed_image_is_failed_and_paints_the_placeholder() {
        let id = ImageId::new(4);
        let mut store = ImageStore::new();
        store.mark_loading(id);
        store.mark_failed(id);
        assert_eq!(store.state(id), Some(ImageState::Failed));
        let frame = store.snapshot().get(id).unwrap();
        assert_eq!(frame.size(), SurfaceSize::new(1, 1).unwrap());
    }

    #[test]
    fn a_failure_never_discards_decoded_pixels() {
        let id = ImageId::new(5);
        let mut store = ImageStore::new();
        store.insert(id, red_pixel());
        store.mark_failed(id);
        assert!(matches!(store.state(id), Some(ImageState::Decoded(_))));
    }

    #[test]
    fn an_image_the_page_never_referenced_has_no_state() {
        assert_eq!(ImageStore::new().state(ImageId::new(9)), None);
    }

    #[test]
    fn clear_forgets_every_image() {
        let id = ImageId::new(3);
        let mut store = ImageStore::new();
        store.mark_loading(id);
        store.clear();
        assert_eq!(store.state(id), None);
    }
}

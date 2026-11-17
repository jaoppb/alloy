//! [`ImageSizes`] — the [`css::NaturalSizes`] port answered from a page's
//! [`ImageStore`] (issue #83, ADR-0028).
//!
//! It sits here, beside the pipeline and the painter, because it is where
//! `css` and `browser` meet: `css` never sees the store, `browser` never sees
//! the port. It is a pure mapping with no I/O, built per render from a
//! borrowed store — so an application-layer type, not an infrastructure
//! adapter (`arch-lint` keeps `crate::infrastructure` out of this layer).

use browser::{ImageState, ImageStore, image_id_of};
use css::{NaturalSize, NaturalSizes, ReplacedResource, SnapshotId};

/// What a page's images tell layout about each replaced element.
#[derive(Clone, Copy, Debug)]
pub struct ImageSizes<'store> {
    images: &'store ImageStore,
}

impl<'store> ImageSizes<'store> {
    #[must_use]
    pub const fn new(images: &'store ImageStore) -> Self {
        Self { images }
    }
}

impl NaturalSizes for ImageSizes<'_> {
    /// A decoded image is loaded at its pixel size; a requested one is
    /// loading; a failed one — or a node the page never requested an image
    /// for — is unavailable.
    fn resource_of(&self, node: SnapshotId) -> ReplacedResource {
        match self.images.state(image_id_of(node)) {
            Some(ImageState::Decoded(size)) => decoded(size.width(), size.height()),
            Some(ImageState::Loading) => ReplacedResource::Loading,
            Some(ImageState::Failed) | None => ReplacedResource::Unavailable,
        }
    }
}

/// A decoded image too large to lay out has no usable natural size.
fn decoded(width: u32, height: u32) -> ReplacedResource {
    NaturalSize::from_pixels(width, height)
        .map_or(ReplacedResource::Unavailable, ReplacedResource::Loaded)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use browser::{ImageStore, image_id_of};
    use css::{NaturalSize, NaturalSizes, ReplacedResource, SnapshotId};
    use graphics::{Color, Framebuffer, SurfaceSize};

    use super::ImageSizes;

    /// The ids of four `<img>` nodes, from a parsed document.
    fn four_nodes() -> Vec<SnapshotId> {
        let tree = dom::parse("<img><img><img><img>").unwrap().into_tree();
        let snapshot = css::snapshot(&tree, tree.document());
        snapshot
            .nodes_in_document_order()
            .filter(|&id| snapshot.node(id).and_then(css::NodeRef::tag_str) == Some("img"))
            .collect()
    }

    fn frame(width: u32, height: u32) -> Framebuffer {
        Framebuffer::filled(SurfaceSize::new(width, height).unwrap(), Color::TRANSPARENT).unwrap()
    }

    #[test]
    fn each_image_state_maps_to_its_resource() {
        let nodes = four_nodes();
        let mut images = ImageStore::new();
        images.insert(image_id_of(nodes[0]), frame(40, 20));
        images.mark_loading(image_id_of(nodes[1]));
        images.mark_failed(image_id_of(nodes[2]));
        let sizes = ImageSizes::new(&images);

        let natural = NaturalSize::from_pixels(40, 20).unwrap();
        assert_eq!(
            sizes.resource_of(nodes[0]),
            ReplacedResource::Loaded(natural)
        );
        assert_eq!(sizes.resource_of(nodes[1]), ReplacedResource::Loading);
        assert_eq!(sizes.resource_of(nodes[2]), ReplacedResource::Unavailable);
        assert_eq!(sizes.resource_of(nodes[3]), ReplacedResource::Unavailable);
    }
}

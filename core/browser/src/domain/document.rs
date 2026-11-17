//! The document a context currently shows, with the sheets fetched and the
//! images requested for it.

use css::StyleSheetSet;
use dom::DomTree;
use graphics::{Framebuffer, ImageId};
use network::Url;

use crate::domain::{ImageStore, SubresourceRequest, Subresources};

/// A DOM tree, the base URL its relative references resolve against, and the
/// sheets fetched and images requested for it. A new document starts from none
/// of them.
#[derive(Debug)]
pub struct Document {
    tree: DomTree,
    base_url: Option<Url>,
    sheets: StyleSheetSet,
    images: ImageStore,
}

impl Document {
    /// A page loaded from the network, resolving relative references against
    /// its effective `base_url` (`<base href>` already applied).
    #[must_use]
    pub fn loaded(tree: DomTree, base_url: Url) -> Self {
        Self::with_base(tree, Some(base_url))
    }

    /// A generated page with no URL of its own, such as the error page.
    #[must_use]
    pub fn generated(tree: DomTree) -> Self {
        Self::with_base(tree, None)
    }

    fn with_base(tree: DomTree, base_url: Option<Url>) -> Self {
        Self {
            tree,
            base_url,
            sheets: StyleSheetSet::new(),
            images: ImageStore::new(),
        }
    }

    /// The parsed markup.
    #[must_use]
    pub const fn tree(&self) -> &DomTree {
        &self.tree
    }

    /// Where relative references resolve; `None` for a generated page.
    #[must_use]
    pub const fn base_url(&self) -> Option<&Url> {
        self.base_url.as_ref()
    }

    /// The external stylesheets absorbed so far, at author precedence.
    #[must_use]
    pub const fn sheets(&self) -> &StyleSheetSet {
        &self.sheets
    }

    /// The images this document requested, with what became of each.
    #[must_use]
    pub const fn images(&self) -> &ImageStore {
        &self.images
    }

    /// Marks every image `found` references as loading until its pixels
    /// arrive, so layout waits for it instead of showing its `alt` text (see
    /// [`ImageStore::mark_loading`]).
    pub fn mark_images_loading(&mut self, found: &Subresources) {
        for request in found {
            mark_request_loading(&mut self.images, request);
        }
    }

    /// Records `id`'s decoded pixels.
    pub fn insert_image(&mut self, id: ImageId, frame: Framebuffer) {
        self.images.insert(id, frame);
    }

    /// Records that `id` will never be decoded (issue #83): its `<img>` may
    /// then show its `alt` text.
    pub fn record_image_failure(&mut self, id: ImageId) {
        self.images.mark_failed(id);
    }

    /// Adds an already parsed author `sheet` to the cascade.
    pub fn add_sheet(&mut self, sheet: StyleSheetSet) {
        self.sheets.absorb(sheet);
    }
}

fn mark_request_loading(images: &mut ImageStore, request: &SubresourceRequest) {
    if let SubresourceRequest::Image(image) = request {
        images.mark_loading(image.id());
    }
}

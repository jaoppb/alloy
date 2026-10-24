//! The document a context currently shows, with everything fetched for it.

use css::StyleSheetSet;
use dom::DomTree;
use graphics::{Framebuffer, ImageId};
use network::Url;

use crate::domain::{ImageStore, SubresourceRequest, Subresources};

/// A DOM tree, the base URL its relative references resolve against, and the
/// sheets and images fetched for it. A new document starts from none of them.
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

    /// The images fetched (or reserved) so far.
    #[must_use]
    pub const fn images(&self) -> &ImageStore {
        &self.images
    }

    /// Gives every image `found` references a placeholder until its pixels
    /// arrive: a discovered image must resolve to *something* from the first
    /// paint (see [`ImageStore::reserve_placeholder`]).
    pub fn reserve_placeholders(&mut self, found: &Subresources) {
        for request in found {
            if let SubresourceRequest::Image(image) = request {
                self.images.reserve_placeholder(image.id());
            }
        }
    }

    /// Replaces `id`'s placeholder with its decoded pixels.
    pub fn insert_image(&mut self, id: ImageId, frame: Framebuffer) {
        self.images.insert(id, frame);
    }

    /// Adds an already parsed author `sheet` to the cascade.
    pub fn add_sheet(&mut self, sheet: StyleSheetSet) {
        self.sheets.absorb(sheet);
    }
}

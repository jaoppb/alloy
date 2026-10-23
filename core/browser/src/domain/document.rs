//! The document a context currently shows, with everything fetched for it.

use css::{MediaQuery, Origin, StyleSheetSet};
use dom::DomTree;
use graphics::{Framebuffer, ImageId};
use network::Url;

use crate::domain::ImageStore;

/// Why a fetched stylesheet was not added to the cascade.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum StylesheetRejected {
    /// Its `media=` can never match a screen.
    #[error("media query does not match screen")]
    NeverMatches,
    /// The text is not a stylesheet.
    #[error("stylesheet does not parse: {0}")]
    Unparsable(css::CssError),
}

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

    /// Gives `id` a placeholder until its pixels arrive.
    pub fn reserve_image(&mut self, id: ImageId) {
        self.images.reserve_placeholder(id);
    }

    /// Replaces `id`'s placeholder with its decoded pixels.
    pub fn insert_image(&mut self, id: ImageId, frame: Framebuffer) {
        self.images.insert(id, frame);
    }

    /// Parses `text` and adds it to the cascade, limited to `media`.
    ///
    /// # Errors
    ///
    /// [`StylesheetRejected`] when `media` can never match a screen or `text`
    /// is not a stylesheet; the cascade is left as it was.
    pub fn absorb_stylesheet(
        &mut self,
        text: &str,
        media: &MediaQuery,
    ) -> Result<(), StylesheetRejected> {
        if media.is_never() {
            return Err(StylesheetRejected::NeverMatches);
        }
        let mut sheet =
            css::parse_stylesheet(text, Origin::Author).map_err(StylesheetRejected::Unparsable)?;
        sheet.apply_media(media);
        tracing::debug!(
            rules = sheet.rules().count(),
            notes = sheet.notes().len(),
            bytes = text.len(),
            "stylesheet absorbed"
        );
        self.sheets.absorb(sheet);
        Ok(())
    }
}

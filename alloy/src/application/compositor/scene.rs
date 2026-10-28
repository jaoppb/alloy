//! What the compositor is asked to draw: borrowed views of the session's
//! state, built fresh for each render so the compositor keeps none of it.

use std::sync::Arc;

use browser::Document;
use graphics::SurfaceSize as LayerSize;
use ui::{OmniboxState, OmniboxTheme};
use window::SurfaceSize;

/// The omnibox strip: its state, the theme it is painted with, and the size of
/// the layer it is rastered into.
pub struct ChromeScene<'a> {
    omnibox: &'a OmniboxState,
    theme: &'a OmniboxTheme,
    size: LayerSize,
}

impl<'a> ChromeScene<'a> {
    #[must_use]
    pub const fn new(omnibox: &'a OmniboxState, theme: &'a OmniboxTheme, size: LayerSize) -> Self {
        Self {
            omnibox,
            theme,
            size,
        }
    }

    pub(super) const fn omnibox(&self) -> &OmniboxState {
        self.omnibox
    }

    pub(super) const fn theme(&self) -> &OmniboxTheme {
        self.theme
    }

    pub(super) const fn size(&self) -> LayerSize {
        self.size
    }
}

/// The page: the document to lay out, if one is loaded, and the size the
/// viewport leaves for it — `None` when it leaves no rows (#118).
pub struct ContentScene<'a> {
    document: Option<&'a Document>,
    size: Option<LayerSize>,
}

impl<'a> ContentScene<'a> {
    #[must_use]
    pub const fn new(document: Option<&'a Document>, size: Option<LayerSize>) -> Self {
        Self { document, size }
    }

    pub(super) const fn document(&self) -> Option<&'a Document> {
        self.document
    }

    pub(super) const fn size(&self) -> Option<LayerSize> {
        self.size
    }
}

/// Both layers' inputs, the fonts they share, and the viewport they stack to.
pub struct Scene<'a, F> {
    chrome: ChromeScene<'a>,
    content: ContentScene<'a>,
    fonts: &'a Arc<F>,
    viewport: SurfaceSize,
}

impl<'a, F> Scene<'a, F> {
    #[must_use]
    pub const fn new(
        chrome: ChromeScene<'a>,
        content: ContentScene<'a>,
        fonts: &'a Arc<F>,
        viewport: SurfaceSize,
    ) -> Self {
        Self {
            chrome,
            content,
            fonts,
            viewport,
        }
    }

    pub(super) const fn chrome(&self) -> &ChromeScene<'a> {
        &self.chrome
    }

    pub(super) const fn content(&self) -> &ContentScene<'a> {
        &self.content
    }

    pub(super) const fn fonts(&self) -> &'a Arc<F> {
        self.fonts
    }

    pub(super) const fn viewport(&self) -> SurfaceSize {
        self.viewport
    }
}

//! Applying a finished fetch to the context.

use css::MediaQuery;
use graphics::{Framebuffer, ImageId};

use super::BrowsingContext;
use crate::application::{SubresourceDiscoverer, document_base_url, error_page};
use crate::domain::{
    BrowserEffect, Document, FetchFailure, FetchPayload, FetchRequest, FetchResult, LoadedDocument,
    NavigationId, StylesheetRejected, SubresourceRequest,
};

impl<D: SubresourceDiscoverer> BrowsingContext<D> {
    /// Applies a finished fetch — but only if it belongs to the latest
    /// navigation. A fetch cannot be cancelled, so a result from a page the user
    /// left is dropped here instead; that includes an `ImageId`, a DOM node
    /// index every document reuses, naming an `<img>` of the new page.
    pub fn complete(&mut self, result: FetchResult) -> Vec<BrowserEffect> {
        let navigation = result.navigation();
        if !self.accepts(navigation) {
            tracing::debug!(%navigation, latest = %self.navigations.latest(), "dropping a result of a superseded navigation");
            return Vec::new();
        }
        match result.into_payload() {
            FetchPayload::Document(Ok(loaded)) => self.load_document(loaded, navigation),
            FetchPayload::Document(Err(failure)) => self.show_navigation_error(&failure),
            FetchPayload::Stylesheet(media, Ok(text)) => self.absorb_stylesheet(&text, &media),
            FetchPayload::Stylesheet(_, Err(failure)) => log_failed("stylesheet", &failure),
            FetchPayload::Image(id, Ok(frame)) => self.accept_image(id, frame),
            FetchPayload::Image(_, Err(failure)) => log_failed("image", &failure),
        }
    }

    fn accepts(&self, navigation: NavigationId) -> bool {
        navigation.context() == self.id && self.navigations.is_latest(navigation)
    }

    /// Installs a freshly navigated document under its effective base URL
    /// (`<base href>` applied) and asks for what it references.
    fn load_document(
        &mut self,
        loaded: LoadedDocument,
        navigation: NavigationId,
    ) -> Vec<BrowserEffect> {
        let (tree, navigation_url) = loaded.into_parts();
        let snapshot = css::snapshot(&tree, tree.document());
        let base_url = document_base_url(&snapshot, &navigation_url);
        tracing::info!(url = %navigation_url, base = %base_url, "navigation complete");
        let found = self.discoverer.discover(&snapshot, &base_url);
        let mut document = Document::loaded(tree, base_url);
        let mut effects = Vec::new();
        for request in found {
            tracing::debug!(?request, "subresource discovered");
            reserve_placeholder(&mut document, &request);
            effects.push(BrowserEffect::Fetch(
                FetchRequest::Subresource(request),
                navigation,
            ));
        }
        self.install(document);
        self.stats.count_navigation();
        effects.push(BrowserEffect::DocumentLoaded(self.id, navigation_url));
        effects.push(self.content_damaged());
        effects
    }

    /// Replaces the page with a visible error document — a failed navigation
    /// must be seen, not just logged.
    fn show_navigation_error(&mut self, failure: &FetchFailure) -> Vec<BrowserEffect> {
        tracing::error!(error = %failure, "navigation failed");
        self.stats.count_navigation_error();
        match error_page(&failure.to_string()) {
            Ok(tree) => {
                self.install(Document::generated(tree));
                vec![self.content_damaged()]
            }
            Err(error) => {
                tracing::error!(%error, "the error page could not be built");
                Vec::new()
            }
        }
    }

    fn absorb_stylesheet(&mut self, text: &str, media: &MediaQuery) -> Vec<BrowserEffect> {
        let Some(document) = self.document.as_mut() else {
            return Vec::new();
        };
        match document.absorb_stylesheet(text, media) {
            Ok(()) => {
                self.stats.count_stylesheet();
                vec![self.content_damaged()]
            }
            Err(StylesheetRejected::NeverMatches) => {
                tracing::debug!("stylesheet skipped: media query does not match screen");
                Vec::new()
            }
            Err(error @ StylesheetRejected::Unparsable(_)) => {
                tracing::warn!(%error, bytes = text.len(), "stylesheet rejected");
                Vec::new()
            }
        }
    }

    fn accept_image(&mut self, id: ImageId, frame: Framebuffer) -> Vec<BrowserEffect> {
        let Some(document) = self.document.as_mut() else {
            return Vec::new();
        };
        document.insert_image(id, frame);
        self.stats.count_image();
        vec![self.content_damaged()]
    }

    /// A new document starts from no sheets, images or link regions — and any
    /// region id minted for the old one stops resolving.
    fn install(&mut self, document: Document) {
        self.regions.invalidate();
        self.document = Some(document);
    }

    const fn content_damaged(&self) -> BrowserEffect {
        BrowserEffect::ContentDamaged(self.id)
    }
}

/// A discovered image must resolve to *something* from the very first paint
/// (see `ImageStore::reserve_placeholder`).
fn reserve_placeholder(document: &mut Document, request: &SubresourceRequest) {
    if let SubresourceRequest::Image(image) = request {
        document.reserve_image(image.id());
    }
}

fn log_failed(kind: &str, failure: &FetchFailure) -> Vec<BrowserEffect> {
    tracing::warn!(error = %failure, kind, "subresource fetch failed");
    Vec::new()
}

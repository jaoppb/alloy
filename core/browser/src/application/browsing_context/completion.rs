//! Applying a finished fetch to the context.

use css::MediaQuery;
use graphics::{Framebuffer, ImageId};

use super::BrowsingContext;
use crate::application::{
    SubresourceDiscoverer, document_base_url, error_page, parse_author_stylesheet,
};
use crate::domain::{
    Accepted, BrowserEffect, Document, Effects, FetchFailure, FetchPayload, FetchRequest,
    FetchResult, LoadedDocument, NavigationId, Subresources,
};

impl<D: SubresourceDiscoverer> BrowsingContext<D> {
    /// Applies a finished fetch — but only if it belongs to the latest
    /// navigation. A fetch cannot be cancelled, so a result from a page the
    /// user left is dropped here instead; that includes an `ImageId`, a DOM node
    /// index every document reuses, naming an `<img>` of the new page.
    pub fn complete(&mut self, result: FetchResult) -> Effects {
        let navigation = result.navigation();
        if !self.accepts(navigation) {
            tracing::debug!(%navigation, latest = %self.navigations.latest(), "dropping a superseded result");
            return Effects::none();
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
        self.navigations.is_latest(navigation)
    }

    /// Installs a freshly navigated document under its effective base URL
    /// (`<base href>` applied) and asks for what it references.
    fn load_document(&mut self, loaded: LoadedDocument, navigation: NavigationId) -> Effects {
        let (tree, navigation_url) = loaded.into_parts();
        let snapshot = css::snapshot(&tree, tree.document());
        let base_url = document_base_url(&snapshot, &navigation_url);
        tracing::info!(url = %navigation_url, base = %base_url, "navigation complete");
        let found = self.discoverer.discover(&snapshot, &base_url);
        let mut document = Document::loaded(tree, base_url);
        document.reserve_placeholders(&found);
        let mut effects = fetches_for(found, navigation);
        self.install(document);
        self.stats.record(Accepted::Navigation);
        effects.push(BrowserEffect::DocumentLoaded(self.id, navigation_url));
        effects.push(self.content_damaged());
        effects
    }

    /// Replaces the page with a visible error document — a failed navigation
    /// must be seen, not just logged.
    fn show_navigation_error(&mut self, failure: &FetchFailure) -> Effects {
        tracing::error!(error = %failure, "navigation failed");
        self.stats.record(Accepted::NavigationError);
        match error_page(&failure.to_string()) {
            Ok(tree) => {
                self.install(Document::generated(tree));
                Effects::from_iter([self.content_damaged()])
            }
            Err(error) => {
                tracing::error!(%error, "the error page could not be built");
                Effects::none()
            }
        }
    }

    fn absorb_stylesheet(&mut self, text: &str, media: &MediaQuery) -> Effects {
        let sheet = match parse_author_stylesheet(text, media) {
            Ok(sheet) => sheet,
            Err(rejected) => {
                tracing::debug!(%rejected, bytes = text.len(), "stylesheet not absorbed");
                return Effects::none();
            }
        };
        let Some(document) = self.document_to_update("stylesheet") else {
            return Effects::none();
        };
        document.add_sheet(sheet);
        self.stats.record(Accepted::Stylesheet);
        Effects::from_iter([self.content_damaged()])
    }

    fn accept_image(&mut self, id: ImageId, frame: Framebuffer) -> Effects {
        let Some(document) = self.document_to_update("image") else {
            return Effects::none();
        };
        document.insert_image(id, frame);
        self.stats.record(Accepted::Image);
        Effects::from_iter([self.content_damaged()])
    }

    /// The document a subresource of `kind` may be added to; none yet means the
    /// result arrived with no page to belong to, which is logged, not applied.
    fn document_to_update(&mut self, kind: &str) -> Option<&mut Document> {
        if self.document.is_none() {
            tracing::debug!(kind, "subresource arrived before any document; dropped");
        }
        self.document.as_mut()
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

/// One fetch per discovered subresource, all under the navigation that found them.
fn fetches_for(found: Subresources, navigation: NavigationId) -> Effects {
    found
        .into_iter()
        .inspect(|request| tracing::debug!(?request, "subresource discovered"))
        .map(|request| BrowserEffect::Fetch(FetchRequest::Subresource(request), navigation))
        .collect()
}

fn log_failed(kind: &str, failure: &FetchFailure) -> Effects {
    tracing::warn!(error = %failure, kind, "subresource fetch failed");
    Effects::none()
}

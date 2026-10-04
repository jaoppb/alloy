//! Folding drained background-fetch results into the [`Session`].

use std::sync::Arc;
use std::sync::mpsc::Sender;

use css::{Origin, StyleSheetSet};
use dom::DomTree;
use graphics::FontProvider;
use network::{HttpTransport, RequestPolicy, Url};

use super::Session;
use crate::application::event_loop::worker::{LoopMessage, spawn_subresource_fetch};
use crate::application::subresource::{
    SubresourceDiscoverer, SubresourceRequest, document_base_url,
};
use crate::error::AlloyError;

impl<F, T, P, D> Session<F, T, P, D>
where
    F: FontProvider + 'static,
    T: HttpTransport + 'static,
    P: RequestPolicy + 'static,
    D: SubresourceDiscoverer,
{
    /// Applies one drained background-fetch result, spawning whatever
    /// follow-up fetches it reveals (a fresh document's subresources).
    pub fn apply(&mut self, message: LoopMessage, sender: &Sender<LoopMessage>) {
        match message {
            LoopMessage::Navigation(Ok((dom_tree, navigation_url))) => {
                self.load_document(dom_tree, &navigation_url, sender);
            }
            LoopMessage::Navigation(Err(error)) => {
                tracing::error!(%error, "navigation failed");
                self.stats.navigation_errors = self.stats.navigation_errors.saturating_add(1);
                self.show_navigation_error(&error);
            }
            LoopMessage::Stylesheet(Ok(text)) => self.absorb_stylesheet(&text),
            LoopMessage::Stylesheet(Err(error)) => {
                tracing::warn!(%error, "stylesheet fetch failed");
            }
            LoopMessage::Image(id, Ok(framebuffer)) => {
                self.images.insert(id, framebuffer);
                self.dirty = true;
                self.stats.images_loaded = self.stats.images_loaded.saturating_add(1);
            }
            LoopMessage::Image(id, Err(error)) => {
                tracing::warn!(%error, %id, "image fetch failed");
            }
        }
    }

    /// Installs a freshly navigated document under its effective base URL
    /// (`<base href>` applied) and starts fetching what it references.
    fn load_document(
        &mut self,
        dom_tree: DomTree,
        navigation_url: &Url,
        sender: &Sender<LoopMessage>,
    ) {
        let snapshot = css::snapshot(&dom_tree, dom_tree.document());
        let base_url = document_base_url(&snapshot, navigation_url);
        tracing::info!(url = %navigation_url, base = %base_url, "navigation complete");
        self.reset_document_state();
        self.spawn_subresources(&snapshot, &base_url, sender);
        self.base_url = Some(base_url);
        self.dom_tree = Some(dom_tree);
        self.dirty = true;
        self.stats.navigations = self.stats.navigations.saturating_add(1);
    }

    /// A new document starts from no sheets, images or links.
    fn reset_document_state(&mut self) {
        self.extra_sheets = StyleSheetSet::new();
        self.images.clear();
        self.links.clear();
    }

    /// Replaces the page with a visible error document — a failed navigation
    /// must be seen, not just logged.
    fn show_navigation_error(&mut self, error: &AlloyError) {
        let escaped_error = error.to_string().replace('&', "&amp;").replace('<', "&lt;");
        let error_html = format!(
            "<!DOCTYPE html><html><head><title>Navigation Error</title><style>body {{ margin: 32px; background-color: #fdf2e9; color: #78281f; }} h1 {{ color: #c0392b; }} .error-box {{ background-color: #ffffff; padding: 16px; border-width: 2px; }}</style></head><body><h1>Navigation Error</h1><div class=\"error-box\"><p><strong>Failed to load:</strong> {escaped_error}</p></div></body></html>"
        );
        let Ok(error_tree) = html::parse(&error_html) else {
            return;
        };
        self.reset_document_state();
        self.dom_tree = Some(error_tree);
        self.dirty = true;
    }

    fn absorb_stylesheet(&mut self, text: &str) {
        let sheet = match css::parse_stylesheet(text, Origin::Author) {
            Ok(sheet) => sheet,
            Err(error) => {
                tracing::warn!(%error, bytes = text.len(), "stylesheet parse failed");
                return;
            }
        };
        tracing::debug!(
            rules = sheet.rules().count(),
            notes = sheet.notes().len(),
            bytes = text.len(),
            "stylesheet absorbed"
        );
        self.extra_sheets.absorb(sheet);
        self.dirty = true;
        self.stats.stylesheets_loaded = self.stats.stylesheets_loaded.saturating_add(1);
    }

    /// Asks the discoverer what `snapshot` references, resolved against the
    /// document's effective `base_url` (the same one link clicks use),
    /// registers a placeholder for every image found (see
    /// `subresource::placeholder_framebuffer`), and spawns one worker thread
    /// per subresource.
    fn spawn_subresources(
        &mut self,
        snapshot: &css::DomSnapshot,
        base_url: &Url,
        sender: &Sender<LoopMessage>,
    ) {
        let found = self.services.discoverer().discover(snapshot, base_url);
        for request in found {
            tracing::debug!(?request, "subresource discovered");
            if let SubresourceRequest::Image(image) = &request {
                self.images.reserve_placeholder(image.id());
            }
            spawn_subresource_fetch(request, Arc::clone(self.services.transport()), sender);
        }
    }
}

//! Session state, stats tracking, and background message handling.

use std::sync::Arc;
use std::sync::mpsc::Sender;

use css::{Origin, StyleSheetSet};
use dom::DomTree;
use graphics::{FontProvider, Framebuffer, ImageId};
use network::{HttpTransport, RequestPolicy, Url};
use window::PhysicalPosition;

use super::worker::spawn_subresource_fetch;
use crate::application::browser_services::BrowserServices;
use crate::application::image_store::ImageStore;
use crate::application::pipeline::LinkTarget;
use crate::application::subresource::{SubresourceDiscoverer, SubresourceRequest};
use crate::error::AlloyError;

/// What a background fetch produced, drained by the loop's own thread.
pub enum LoopMessage {
    Navigation(Result<(DomTree, Url), AlloyError>),
    Stylesheet(Result<String, AlloyError>),
    Image(ImageId, Result<Framebuffer, AlloyError>),
}

/// What a run of the loop did so far.
///
/// Instrumented for I4's coalescing proofs (resize, subresource bursts) and
/// for a caller (or a test) that needs to wait for a specific piece of
/// background work to land before looking at the presented frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LoopStats {
    /// How many times this run actually re-laid-out and presented a frame. A
    /// bare `RedrawRequested` repaint re-blits the cached frame and does not
    /// count here — the I4 coalescing proof is about relayouts only.
    pub relayouts: usize,
    /// How many navigations have successfully parsed into the document tree.
    pub navigations: usize,
    /// How many navigations failed and fell back to the error card.
    pub navigation_errors: usize,
    /// How many external `<link rel=stylesheet>` sheets were absorbed into the
    /// cascade.
    pub stylesheets_loaded: usize,
    /// How many `<img>` fetches have decoded and replaced their placeholder.
    pub images_loaded: usize,
}

/// Everything one pump cycle can mutate, bundled so `pump_once` stays under
/// the arity limit: the accumulated document state, the current viewport, and
/// the running relayout count.
///
/// Rebuilding a fresh display list from `dom_tree`/`extra_sheets`/`images` on
/// every relayout is the same "immutable snapshot in, immutable snapshot out"
/// discipline the render pipeline itself uses (`ADR-0010:114-117`), applied
/// to the state a live session must keep between frames.
pub struct Session<F, T, P, D> {
    pub services: BrowserServices<F, T, P, D>,
    pub dom_tree: Option<DomTree>,
    pub base_url: Option<Url>,
    pub extra_sheets: StyleSheetSet,
    pub images: ImageStore,
    pub links: Vec<LinkTarget>,
    pub pointer_pos: Option<PhysicalPosition>,
    pub dirty: bool,
    pub viewport: window::SurfaceSize,
    pub last_frame: Option<CachedFrame>,
    pub stats: LoopStats,
}

/// The pixels of the last frame `relayout_and_present` produced, kept so a
/// `RedrawRequested` can re-blit them without re-running the whole
/// `render_dom_with_links` pipeline (cascade → layout → paint → raster →
/// readback). This is the "repaint is cheap, relayout is not" split the event
/// loop rests on.
pub struct CachedFrame {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u32>,
}

impl<F, T, P, D> Session<F, T, P, D>
where
    F: FontProvider + 'static,
    T: HttpTransport + 'static,
    P: RequestPolicy + 'static,
    D: SubresourceDiscoverer,
{
    pub fn new(viewport: window::SurfaceSize, services: BrowserServices<F, T, P, D>) -> Self {
        Self {
            services,
            dom_tree: None,
            base_url: None,
            extra_sheets: StyleSheetSet::new(),
            images: ImageStore::new(),
            links: Vec::new(),
            pointer_pos: None,
            dirty: false,
            viewport,
            last_frame: None,
            stats: LoopStats::default(),
        }
    }

    /// Applies one drained background-fetch result, spawning whatever
    /// follow-up fetches it reveals (a fresh document's subresources).
    pub fn apply(&mut self, message: LoopMessage, sender: &Sender<LoopMessage>) {
        match message {
            LoopMessage::Navigation(Ok((dom_tree, base_url))) => {
                tracing::info!(url = %base_url, "navigation complete");
                self.reset_document_state();
                self.base_url = Some(base_url.clone());
                self.spawn_subresources(&dom_tree, &base_url, sender);
                self.dom_tree = Some(dom_tree);
                self.dirty = true;
                self.stats.navigations = self.stats.navigations.saturating_add(1);
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

    /// Asks the discoverer what `dom_tree` references, registers a
    /// placeholder for every image found (see
    /// `subresource::placeholder_framebuffer`), and spawns one worker thread
    /// per subresource.
    fn spawn_subresources(
        &mut self,
        dom_tree: &DomTree,
        base_url: &Url,
        sender: &Sender<LoopMessage>,
    ) {
        let snapshot = css::snapshot(dom_tree, dom_tree.document());
        let found = self.services.discoverer().discover(&snapshot, base_url);
        for request in found {
            tracing::debug!(?request, "subresource discovered");
            if let SubresourceRequest::Image(image) = &request {
                self.images.reserve_placeholder(image.id());
            }
            spawn_subresource_fetch(request, Arc::clone(self.services.transport()), sender);
        }
    }

    pub const fn record_relayout(&mut self) {
        self.stats.relayouts = self.stats.relayouts.saturating_add(1);
    }
}

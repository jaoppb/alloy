//! Fixtures shared by the event-loop test modules: a session over a fake web, a
//! loaded page, a headless window, and the pump.
#![allow(clippy::unwrap_used)]
use std::collections::HashMap;
use std::sync::Arc;

use browser::{
    FetchFailure, FetchPayload, FetchRequest, FetchResult, LoadedDocument, NavigationId,
};
use graphics::{SoftwareCpuBackend, SyntheticFontProvider};
use network::Url;
use window::{HeadlessWindowSystem, RecordingPresenter, SurfaceSize, WindowSystem as _};

use super::session::Session;
use super::{initial_window_attributes, pump_once};
use crate::application::browser_services::BrowserServices;
use crate::application::compositor::Compositor;
use crate::application::fetch_channel::{FetchOutbox, fetch_channel};
use crate::application::paint::DEFAULT_FONT;
use crate::application::pipeline::DEFAULT_FONT_SIZE;
use crate::application::ports::{FetchExecutor, IdlePause};

/// The pages a test's fake network serves, by address. Anything else is a
/// failed fetch, like a host that does not answer.
#[derive(Default)]
pub(super) struct FakeWeb {
    pages: HashMap<Url, &'static str>,
}

impl FakeWeb {
    pub(super) fn new() -> Self {
        Self::default()
    }

    pub(super) fn with_page(mut self, url: Url, markup: &'static str) -> Self {
        self.pages.insert(url, markup);
        self
    }

    fn document(&self, url: &Url) -> Result<LoadedDocument, FetchFailure> {
        let markup = self
            .pages
            .get(url)
            .ok_or_else(|| FetchFailure::new(format!("the fake web has no page at {url}")))?;
        let tree = dom::parse(markup).unwrap().into_tree();
        Ok(LoadedDocument::new(tree, url.clone()))
    }
}

/// A [`FetchExecutor`] that answers on the caller's own thread, from a
/// [`FakeWeb`]: the result is already waiting on the inbox when `execute`
/// returns, so a test pumps the loop and asserts — no thread to outwait, no
/// sleep, and none of the HTTP stack.
pub(super) struct InlineFetchExecutor {
    web: FakeWeb,
    outbox: FetchOutbox,
}

impl InlineFetchExecutor {
    /// Reports `result` as if a fetch had just finished; tests send results of their own.
    pub(super) fn deliver(&self, result: FetchResult) {
        self.outbox.send(result);
    }
}

impl FetchExecutor for InlineFetchExecutor {
    fn execute(&self, request: FetchRequest, navigation: NavigationId) {
        let payload = match &request {
            FetchRequest::Document(document) => {
                FetchPayload::Document(self.web.document(document.url()))
            }
            FetchRequest::Subresource(_) => {
                request.failed(FetchFailure::new("the fake web serves documents only"))
            }
        };
        self.outbox.send(FetchResult::new(navigation, payload));
    }
}

/// Never waits: a pump test has nothing to idle for.
pub(super) struct NoPause;

impl IdlePause for NoPause {
    fn pause(&self) {}
}

pub(super) type TestSession = Session<
    SyntheticFontProvider,
    InlineFetchExecutor,
    NoPause,
    SoftwareCpuBackend<SyntheticFontProvider>,
>;

pub(super) fn session_over(web: FakeWeb, viewport: SurfaceSize) -> TestSession {
    let font_provider =
        Arc::new(SyntheticFontProvider::new().with_size(DEFAULT_FONT, DEFAULT_FONT_SIZE));
    let compositor = Compositor::new(
        SoftwareCpuBackend::with_font_provider(Arc::clone(&font_provider)),
        SoftwareCpuBackend::with_font_provider(Arc::clone(&font_provider)),
    );
    let (outbox, inbox) = fetch_channel();
    let fetch_executor = InlineFetchExecutor { web, outbox };
    let services = BrowserServices::new(font_provider, fetch_executor, inbox, NoPause);
    Session::new(viewport, services, compositor)
}

/// Hands `result` to the session as if a fetch had just finished.
pub(super) fn deliver(session: &TestSession, result: FetchResult) {
    session.fetch_executor().deliver(result);
}

/// The URL a link click navigated to, after the pump that applies the result
/// the inline executor already produced.
pub(super) fn completed_navigation(
    system: &mut HeadlessWindowSystem,
    presenter: &mut RecordingPresenter,
    session: &mut TestSession,
    navigations_before_click: usize,
) -> network::Url {
    pump(system, presenter, session);
    assert!(
        session.stats().navigations > navigations_before_click,
        "the click's navigation never arrived"
    );
    session
        .base_url()
        .cloned()
        .expect("an applied navigation sets the document base")
}

/// A finished navigation to `url`, stamped with the one the session accepts.
pub(super) fn navigation_to(session: &TestSession, markup: &str, url: &str) -> FetchResult {
    let document = dom::parse(markup).unwrap().into_tree();
    let loaded = LoadedDocument::new(document, network::Url::parse(url).unwrap());
    FetchResult::new(
        session.current_navigation_for_test(),
        FetchPayload::Document(Ok(loaded)),
    )
}

/// Installs `markup` as a navigated document at `url`, the same way a
/// finished navigation does.
pub(super) fn load(session: &mut TestSession, markup: &str, url: &str) {
    deliver(session, navigation_to(session, markup, url));
    session.drain_messages();
}

pub(super) fn loaded_session(viewport: SurfaceSize) -> TestSession {
    let mut session = session_over(FakeWeb::new(), viewport);
    load(
        &mut session,
        "<html><body>hi</body></html>",
        "http://example.com/",
    );
    session
}

pub(super) fn pump(
    system: &mut HeadlessWindowSystem,
    presenter: &mut RecordingPresenter,
    session: &mut TestSession,
) {
    pump_once(system, presenter, session).unwrap();
}

/// A headless window and a loaded page, before the first pump.
pub(super) fn loaded_window() -> (HeadlessWindowSystem, RecordingPresenter, TestSession) {
    let attributes = initial_window_attributes().unwrap();
    let mut system = HeadlessWindowSystem::new();
    system.create_window(&attributes).unwrap();
    let session = loaded_session(attributes.initial_size());
    (system, RecordingPresenter::new(), session)
}

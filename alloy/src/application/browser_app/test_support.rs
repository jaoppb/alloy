//! Fixtures shared by the application test modules: a browser over a fake web, a
//! loaded page, a headless window, and the pump.
#![allow(clippy::unwrap_used)]
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use browser::{
    FetchFailure, FetchPayload, FetchRequest, FetchResult, LoadedDocument, NavigationId,
};
use graphics::{SoftwareCpuBackend, SyntheticFontProvider};
use network::Url;
use ui::Keymap;
use window::{HeadlessWindowSystem, RecordingPresenter, SurfaceSize, WindowSystem as _};

use crate::application::browser_app::BrowserApp;
use crate::application::browser_services::BrowserServices;
use crate::application::compositor::Compositor;
use crate::application::event_loop::pump_once;
use crate::application::fetch_channel::{FetchOutbox, fetch_channel};
use crate::application::paint::DEFAULT_FONT;
use crate::application::pipeline::DEFAULT_FONT_SIZE;
use crate::application::ports::{FetchExecutor, IdlePause};
use crate::composition::initial_window_attributes;

/// The pages a test's fake network serves, by address. Anything else is a
/// failed fetch, like a host that does not answer.
#[derive(Default)]
pub struct FakeWeb {
    pages: HashMap<Url, &'static str>,
}

impl FakeWeb {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_page(mut self, url: Url, markup: &'static str) -> Self {
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
pub struct InlineFetchExecutor {
    web: FakeWeb,
    outbox: FetchOutbox,
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
pub struct NoPause;

impl IdlePause for NoPause {
    fn pause(&self) {}
}

type TestBrowser = BrowserApp<
    SyntheticFontProvider,
    InlineFetchExecutor,
    NoPause,
    SoftwareCpuBackend<SyntheticFontProvider>,
>;

/// A browser plus the outbox a test reports finished fetches on, as a worker
/// thread would.
pub struct TestApp {
    app: TestBrowser,
    outbox: FetchOutbox,
}

impl Deref for TestApp {
    type Target = TestBrowser;

    fn deref(&self) -> &TestBrowser {
        &self.app
    }
}

impl DerefMut for TestApp {
    fn deref_mut(&mut self) -> &mut TestBrowser {
        &mut self.app
    }
}

pub fn app_over(web: FakeWeb, viewport: SurfaceSize) -> TestApp {
    let font_provider =
        Arc::new(SyntheticFontProvider::new().with_size(DEFAULT_FONT, DEFAULT_FONT_SIZE));
    let compositor = Compositor::new(
        SoftwareCpuBackend::with_font_provider(Arc::clone(&font_provider)),
        SoftwareCpuBackend::with_font_provider(Arc::clone(&font_provider)),
    );
    let (outbox, inbox) = fetch_channel();
    let fetch_executor = InlineFetchExecutor {
        web,
        outbox: outbox.clone(),
    };
    let services = BrowserServices::new(font_provider, fetch_executor, inbox, NoPause);
    let app = BrowserApp::new(viewport, services, compositor, Keymap::default());
    TestApp { app, outbox }
}

/// Hands `result` to the app as if a fetch had just finished.
pub fn deliver(app: &TestApp, result: FetchResult) {
    app.outbox.send(result);
}

/// The URL a link click navigated to, after the pump that applies the result
/// the inline executor already produced.
pub fn completed_navigation(
    system: &mut HeadlessWindowSystem,
    presenter: &mut RecordingPresenter,
    app: &mut TestApp,
    navigations_before_click: usize,
) -> network::Url {
    pump(system, presenter, app);
    assert!(
        app.stats().navigations() > navigations_before_click,
        "the click's navigation never arrived"
    );
    app.browsing
        .document()
        .and_then(browser::Document::base_url)
        .cloned()
        .expect("an applied navigation sets the document base")
}

/// A finished navigation to `url`, stamped with the one the app accepts.
pub fn navigation_to(app: &TestApp, markup: &str, url: &str) -> FetchResult {
    let document = dom::parse(markup).unwrap().into_tree();
    let loaded = LoadedDocument::new(document, network::Url::parse(url).unwrap());
    FetchResult::new(
        app.browsing.current_navigation(),
        FetchPayload::Document(Ok(loaded)),
    )
}

/// Installs `markup` as a navigated document at `url`, the same way a
/// finished navigation does.
pub fn load(app: &mut TestApp, markup: &str, url: &str) {
    deliver(app, navigation_to(app, markup, url));
    drain(app);
}

/// Applies every finished fetch waiting, without a pump.
fn drain(app: &mut TestApp) {
    while let Some(result) = app.next_ready_fetch() {
        app.complete_fetch(result);
    }
}

pub fn loaded_app(viewport: SurfaceSize) -> TestApp {
    let mut app = app_over(FakeWeb::new(), viewport);
    load(
        &mut app,
        "<html><body>hi</body></html>",
        "http://example.com/",
    );
    app
}

pub fn pump(
    system: &mut HeadlessWindowSystem,
    presenter: &mut RecordingPresenter,
    app: &mut TestApp,
) {
    pump_once(system, presenter, &mut app.app).unwrap();
}

/// A headless window and a loaded page, before the first pump.
pub fn loaded_window() -> (HeadlessWindowSystem, RecordingPresenter, TestApp) {
    let attributes = initial_window_attributes().unwrap();
    let mut system = HeadlessWindowSystem::new();
    system.create_window(&attributes).unwrap();
    let app = loaded_app(attributes.initial_size());
    (system, RecordingPresenter::new(), app)
}

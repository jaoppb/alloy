//! Fixtures shared by the event-loop test modules: a session over a mock
//! transport, a loaded page, a headless window, and the pump.
#![allow(clippy::unwrap_used)]

use std::sync::Arc;

use browser::{FetchPayload, FetchRequest, FetchResult, LoadedDocument, NavigationId};
use graphics::SyntheticFontProvider;
use network::{AllowAllPolicy, MockTransport};
use window::{HeadlessWindowSystem, RecordingPresenter, SurfaceSize, WindowSystem as _};

use super::session::Session;
use super::{initial_window_attributes, pump_once};
use crate::application::browser_services::BrowserServices;
use crate::application::fetch_channel::{FetchOutbox, fetch_channel};
use crate::application::paint::DEFAULT_FONT;
use crate::application::pipeline::DEFAULT_FONT_SIZE;
use crate::application::ports::{FetchExecutor, IdlePause};
use crate::infrastructure::{HttpFetcher, PngImageDecoder};

/// A [`FetchExecutor`] that fetches on the caller's own thread: the result is
/// already waiting on the inbox when `execute` returns, so a test pumps the
/// loop and asserts — no thread to outwait, no sleep.
pub(super) struct InlineFetchExecutor {
    fetcher: HttpFetcher<MockTransport, AllowAllPolicy, PngImageDecoder>,
    outbox: FetchOutbox,
}

impl InlineFetchExecutor {
    /// Where this executor reports; tests also send results of their own.
    pub(super) const fn outbox(&self) -> &FetchOutbox {
        &self.outbox
    }
}

impl FetchExecutor for InlineFetchExecutor {
    fn execute(&self, request: FetchRequest, navigation: NavigationId) {
        let payload = self.fetcher.fetch(request);
        self.outbox.send(FetchResult::new(navigation, payload));
    }
}

/// Never waits: a pump test has nothing to idle for.
pub(super) struct NoPause;

impl IdlePause for NoPause {
    fn pause(&self) {}
}

pub(super) type TestSession = Session<SyntheticFontProvider, InlineFetchExecutor, NoPause>;

pub(super) fn session_over(transport: MockTransport, viewport: SurfaceSize) -> TestSession {
    let font_provider =
        Arc::new(SyntheticFontProvider::new().with_size(DEFAULT_FONT, DEFAULT_FONT_SIZE));
    let (outbox, inbox) = fetch_channel();
    let fetch_executor = InlineFetchExecutor {
        fetcher: HttpFetcher::new(transport, AllowAllPolicy::new(), PngImageDecoder),
        outbox,
    };
    let services = BrowserServices::new(font_provider, fetch_executor, inbox, NoPause);
    Session::new(viewport, services)
}

/// Hands `result` to the session as if a fetch had just finished.
pub(super) fn deliver(session: &TestSession, result: FetchResult) {
    session.fetch_executor().outbox().send(result);
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
    let mut session = session_over(MockTransport::new(), viewport);
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

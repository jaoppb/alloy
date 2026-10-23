//! Fixtures shared by the event-loop test modules: a session over a mock
//! transport, a loaded page, a headless window, and the pump.
#![allow(clippy::unwrap_used)]

use std::sync::Arc;

use browser::{FetchPayload, FetchResult, LoadedDocument, MarkupDiscoverer};
use graphics::SyntheticFontProvider;
use network::{AllowAllPolicy, MockTransport};
use window::{HeadlessWindowSystem, RecordingPresenter, SurfaceSize, WindowSystem as _};

use super::session::Session;
use super::{initial_window_attributes, pump_once};
use crate::application::browser_services::BrowserServices;
use crate::application::paint::DEFAULT_FONT;
use crate::application::pipeline::DEFAULT_FONT_SIZE;

pub(super) type TestSession =
    Session<SyntheticFontProvider, MockTransport, AllowAllPolicy, MarkupDiscoverer>;

pub(super) fn session_over(transport: MockTransport, viewport: SurfaceSize) -> TestSession {
    let font_provider =
        Arc::new(SyntheticFontProvider::new().with_size(DEFAULT_FONT, DEFAULT_FONT_SIZE));
    let services = BrowserServices::new(
        font_provider,
        Arc::new(transport),
        Arc::new(AllowAllPolicy::new()),
    );
    Session::new(viewport, services)
}

/// The URL a link click navigated to: keeps pumping until the click's
/// navigation — the first one past `navigations_before_click` — has been
/// applied, so the outcome does not depend on how fast its thread runs.
pub(super) fn completed_navigation(
    system: &mut HeadlessWindowSystem,
    presenter: &mut RecordingPresenter,
    session: &mut TestSession,
    navigations_before_click: usize,
) -> network::Url {
    for _ in 0..5000 {
        if session.stats().navigations > navigations_before_click {
            break;
        }
        pump(system, presenter, session);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
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
/// finished navigation thread does.
pub(super) fn load(session: &mut TestSession, markup: &str, url: &str) {
    session
        .outbox_for_test()
        .send(navigation_to(session, markup, url))
        .unwrap();
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

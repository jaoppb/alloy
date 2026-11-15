//! Each effect reaches the sibling it is meant for, and nothing else.
#![allow(clippy::unwrap_used)]

use std::cell::RefCell;

use browser::{
    BrowsingContext, FetchPayload, FetchRequest, FetchResult, LoadedDocument, MarkupDiscoverer,
    NavigationId,
};
use network::Url;
use platform::HostOs;
use ui::{Keymap, UiController, UiEffect, UiEffects};
use window::WindowTitle;

use super::EffectExecutor;
use crate::application::compositor::PendingDamage;
use crate::application::pending_title::PendingTitle;
use crate::application::ports::FetchExecutor;

/// Remembers every request instead of fetching it.
#[derive(Default)]
struct RecordingFetcher {
    started: RefCell<Vec<(FetchRequest, NavigationId)>>,
}

impl FetchExecutor for RecordingFetcher {
    fn execute(&self, request: FetchRequest, navigation: NavigationId) {
        self.started.borrow_mut().push((request, navigation));
    }
}

/// The state an executor borrows.
struct Fixture {
    browsing: BrowsingContext<MarkupDiscoverer>,
    ui: UiController,
    damage: PendingDamage,
    title: PendingTitle,
    fetcher: RecordingFetcher,
}

impl Fixture {
    fn new() -> Self {
        Self {
            browsing: BrowsingContext::new(MarkupDiscoverer),
            ui: UiController::new(Keymap::default(), HostOs::Other),
            damage: PendingDamage::default(),
            title: PendingTitle::default(),
            fetcher: RecordingFetcher::default(),
        }
    }

    fn executor(&mut self) -> EffectExecutor<'_, RecordingFetcher> {
        EffectExecutor::new(
            &mut self.browsing,
            &mut self.ui,
            &mut self.damage,
            &mut self.title,
            &self.fetcher,
        )
    }
}

fn url(text: &str) -> Url {
    Url::parse(text).unwrap()
}

#[test]
fn a_damaged_chrome_is_chrome_damage_and_not_content_damage() {
    let mut fixture = Fixture::new();

    fixture
        .executor()
        .execute_ui(UiEffects::from(UiEffect::ChromeDamaged));

    assert!(fixture.damage.needs_chrome());
    assert!(!fixture.damage.needs_content());
}

#[test]
fn navigating_starts_the_document_fetch_and_shows_the_address() {
    let mut fixture = Fixture::new();

    fixture.executor().navigate(url("http://example.com/a"));

    assert_eq!(fixture.fetcher.started.borrow().len(), 1);
    assert_eq!(fixture.ui.omnibox().text(), "http://example.com/a");
    assert!(fixture.damage.needs_chrome());
}

#[test]
fn a_ui_navigate_effect_navigates_the_browsing_context() {
    let mut fixture = Fixture::new();

    fixture
        .executor()
        .execute_ui(UiEffects::from(UiEffect::Navigate(url(
            "http://example.com/b",
        ))));

    assert_eq!(fixture.fetcher.started.borrow().len(), 1);
    assert_eq!(fixture.browsing.stats().navigations(), 0);
}

#[test]
fn a_finished_document_damages_the_content_and_counts_one_navigation() {
    let mut fixture = Fixture::new();
    fixture.executor().navigate(url("http://example.com/c"));
    let page = dom::parse("<html><body>hi</body></html>")
        .unwrap()
        .into_tree();
    let loaded = LoadedDocument::new(page, url("http://example.com/c"));
    let navigation = fixture.browsing.current_navigation();

    fixture.executor().complete(FetchResult::new(
        navigation,
        FetchPayload::Document(Ok(loaded)),
    ));

    assert!(fixture.damage.needs_content());
    assert_eq!(fixture.browsing.stats().navigations(), 1);
}

fn complete_navigation(fixture: &mut Fixture, markup: &str, at: &str) {
    fixture.executor().navigate(url(at));
    let page = dom::parse(markup).unwrap().into_tree();
    let loaded = LoadedDocument::new(page, url(at));
    let navigation = fixture.browsing.current_navigation();
    fixture.executor().complete(FetchResult::new(
        navigation,
        FetchPayload::Document(Ok(loaded)),
    ));
}

#[test]
fn a_loaded_documents_title_waits_for_the_loop_to_name_the_window() {
    let mut fixture = Fixture::new();

    complete_navigation(&mut fixture, "<title>x</title>", "http://example.com/x");

    assert_eq!(fixture.title.take(), Some(WindowTitle::from("x")));
    assert_eq!(fixture.title.take(), None, "a title is applied once");
}

#[test]
fn only_the_latest_title_is_kept_when_two_documents_load_in_one_cycle() {
    let mut fixture = Fixture::new();

    complete_navigation(&mut fixture, "<title>first</title>", "http://example.com/1");
    complete_navigation(
        &mut fixture,
        "<title>second</title>",
        "http://example.com/2",
    );

    assert_eq!(fixture.title.take(), Some(WindowTitle::from("second")));
}

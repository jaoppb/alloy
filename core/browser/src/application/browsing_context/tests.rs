//! The browsing context driven purely by commands and results: no thread, no
//! channel, no sleep. A "late" fetch is simply a `complete` call made after
//! the navigation that started it was superseded.

use std::sync::Arc;

use css::MediaQuery;
use graphics::{Au, Color, Framebuffer, ImageId, Point, Rect, Size, SurfaceSize};
use network::Url;

use super::BrowsingContext;
use crate::domain::{
    BrowserEffect, ContextId, FetchFailure, FetchPayload, FetchRequest, FetchResult, LinkTarget,
    LoadedDocument, NavigationId, SubresourceRequest,
};
use crate::infrastructure::MarkupDiscoverer;

type TestContext = BrowsingContext<MarkupDiscoverer>;

fn context() -> TestContext {
    BrowsingContext::new(Arc::new(MarkupDiscoverer))
}

fn url(text: &str) -> Url {
    Url::parse(text).unwrap()
}

fn page(path: &str) -> Url {
    url(&format!("http://example.com/{path}"))
}

/// The navigation a `navigate` / `follow_link` call started.
fn started(effects: &[BrowserEffect]) -> NavigationId {
    effects
        .iter()
        .find_map(|effect| match effect {
            BrowserEffect::Fetch(FetchRequest::Document(_), navigation) => Some(*navigation),
            _ => None,
        })
        .expect("the call started a navigation")
}

fn document_result(navigation: NavigationId, markup: &str, at: &Url) -> FetchResult {
    let tree = dom::parse(markup).unwrap().into_tree();
    let loaded = LoadedDocument::new(tree, at.clone());
    FetchResult::new(navigation, FetchPayload::Document(Ok(loaded)))
}

fn failure_result(navigation: NavigationId, message: &str) -> FetchResult {
    let failure = FetchFailure::new(message);
    FetchResult::new(navigation, FetchPayload::Document(Err(failure)))
}

fn stylesheet_result(navigation: NavigationId, media: MediaQuery, text: &str) -> FetchResult {
    FetchResult::new(
        navigation,
        FetchPayload::Stylesheet(media, Ok(text.to_owned())),
    )
}

fn pixel() -> Framebuffer {
    let size = SurfaceSize::new(1, 1).unwrap();
    Framebuffer::filled(size, Color::rgb(255, 0, 0)).unwrap()
}

fn image_result(navigation: NavigationId, id: u32) -> FetchResult {
    FetchResult::new(
        navigation,
        FetchPayload::Image(ImageId::new(id), Ok(pixel())),
    )
}

/// Navigates to `at` and completes it with `markup`; returns the navigation id.
fn load(context: &mut TestContext, markup: &str, at: &Url) -> NavigationId {
    let navigation = started(&context.navigate(at.clone()));
    context.complete(document_result(navigation, markup, at));
    navigation
}

fn link(href: &str) -> LinkTarget {
    let au = |pixels| Au::from_whole_px(pixels).unwrap();
    let area = Rect::new(Point::new(au(0), au(0)), Size::new(au(10), au(10)).unwrap());
    LinkTarget::new(area, href)
}

fn base_url(context: &TestContext) -> Option<String> {
    context
        .document()
        .and_then(|document| document.base_url())
        .map(ToString::to_string)
}

fn damaged(effects: &[BrowserEffect]) -> bool {
    effects
        .iter()
        .any(|effect| matches!(effect, BrowserEffect::ContentDamaged(_)))
}

#[test]
fn navigating_announces_the_url_and_asks_for_the_document_under_a_fresh_id() {
    let mut context = context();
    let target = page("a.html");

    let effects = context.navigate(target.clone());

    let navigation = started(&effects);
    assert_eq!(navigation, context.current_navigation());
    assert_eq!(
        effects,
        vec![
            BrowserEffect::NavigationStarted(ContextId::ROOT, target.clone()),
            BrowserEffect::Fetch(FetchRequest::Document(target), navigation),
        ]
    );
}

#[test]
fn a_completed_document_is_installed_and_reports_what_to_fetch_next() {
    let mut context = context();
    let target = page("index.html");
    let navigation = started(&context.navigate(target.clone()));

    let effects = context.complete(document_result(
        navigation,
        r#"<html><head><link rel="stylesheet" href="a.css"></head><body><img src="p.png"></body></html>"#,
        &target,
    ));

    let fetched: Vec<_> = effects
        .iter()
        .filter_map(|effect| match effect {
            BrowserEffect::Fetch(FetchRequest::Subresource(request), stamp) => {
                Some((request.clone(), *stamp))
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        fetched.len(),
        2,
        "one stylesheet and one image: {effects:?}"
    );
    assert!(
        fetched.iter().all(|(_, stamp)| *stamp == navigation),
        "subresources report under the navigation that found them"
    );
    assert!(matches!(fetched[0].0, SubresourceRequest::Stylesheet(_)));
    assert!(matches!(fetched[1].0, SubresourceRequest::Image(_)));
    assert!(effects.contains(&BrowserEffect::DocumentLoaded(ContextId::ROOT, target)));
    assert!(damaged(&effects));
    assert_eq!(context.stats().navigations(), 1);
}

#[test]
fn every_discovered_image_has_a_placeholder_before_its_pixels_arrive() {
    let mut context = context();
    let target = page("index.html");
    let navigation = started(&context.navigate(target.clone()));

    let effects = context.complete(document_result(
        navigation,
        r#"<body><img src="/p.png"></body>"#,
        &target,
    ));

    let image_id = effects
        .iter()
        .find_map(|effect| match effect {
            BrowserEffect::Fetch(
                FetchRequest::Subresource(SubresourceRequest::Image(image)),
                _,
            ) => Some(image.id()),
            _ => None,
        })
        .expect("the <img> was discovered");
    assert!(context.document().unwrap().images().contains(image_id));
}

#[test]
fn the_documents_base_href_not_the_navigation_url_becomes_its_base() {
    let mut context = context();
    load(
        &mut context,
        r#"<head><base href="https://cdn.example/app/"></head>"#,
        &url("https://example.com/index.html"),
    );

    assert_eq!(
        base_url(&context).as_deref(),
        Some("https://cdn.example/app/")
    );
}

#[test]
fn a_subresource_of_a_page_the_user_left_is_not_applied_to_the_next_one() {
    let mut context = context();
    let page_a = load(&mut context, "<body>a</body>", &page("a.html"));
    context.navigate(page("b.html"));

    let late_sheet = context.complete(stylesheet_result(
        page_a,
        MediaQuery::always(),
        "p { color: red }",
    ));
    let late_image = context.complete(image_result(page_a, 12));

    assert!(
        late_sheet.is_empty() && late_image.is_empty(),
        "dropped silently"
    );
    assert_eq!(context.stats().stylesheets_loaded(), 0);
    assert_eq!(context.stats().images_loaded(), 0);
}

#[test]
fn a_slower_earlier_navigation_does_not_replace_the_latest_one() {
    let mut context = context();
    let slow = started(&context.navigate(page("slow.html")));
    let fast = started(&context.navigate(page("fast.html")));
    context.complete(document_result(
        fast,
        "<body>fast</body>",
        &page("fast.html"),
    ));

    let effects = context.complete(document_result(
        slow,
        "<body>slow</body>",
        &page("slow.html"),
    ));

    assert!(effects.is_empty());
    assert_eq!(base_url(&context), Some(page("fast.html").to_string()));
    assert_eq!(
        context.stats().navigations(),
        1,
        "the slow one is not counted"
    );
}

#[test]
fn a_failure_of_a_superseded_navigation_does_not_replace_the_current_page() {
    let mut context = context();
    let slow = started(&context.navigate(page("slow.html")));
    let fast = started(&context.navigate(page("fast.html")));
    context.complete(document_result(
        fast,
        "<body>fast</body>",
        &page("fast.html"),
    ));

    let effects = context.complete(failure_result(slow, "connection reset"));

    assert!(effects.is_empty());
    assert_eq!(
        context.stats().navigation_errors(),
        0,
        "an error nobody is waiting for must not become the error page"
    );
    assert_eq!(base_url(&context), Some(page("fast.html").to_string()));
}

#[test]
fn a_failed_navigation_replaces_the_page_with_the_error_page_showing_the_message() {
    let mut context = context();
    load(&mut context, "<body>old</body>", &page("old.html"));
    let navigation = started(&context.navigate(page("broken.html")));

    let effects = context.complete(failure_result(navigation, "<script>&"));

    assert!(damaged(&effects));
    assert_eq!(context.stats().navigation_errors(), 1);
    let tree = context.document().unwrap().tree();
    let texts: Vec<_> = tree
        .descendants(tree.document())
        .filter_map(|node| tree.text(node).ok())
        .map(|text| text.as_str().to_owned())
        .collect();
    assert!(texts.contains(&"<script>&".to_owned()), "{texts:?}");
    assert!(
        !tree
            .descendants(tree.document())
            .any(|node| tree.tag(node).is_ok_and(|tag| tag.as_str() == "script")),
        "the message must not become markup"
    );
}

#[test]
fn a_stylesheet_for_the_current_page_is_absorbed_and_damages_the_content() {
    let mut context = context();
    let navigation = load(&mut context, "<body>hi</body>", &page("a.html"));

    let effects = context.complete(stylesheet_result(
        navigation,
        MediaQuery::always(),
        "p { color: red }",
    ));

    assert!(damaged(&effects));
    assert_eq!(context.stats().stylesheets_loaded(), 1);
    assert_eq!(context.document().unwrap().sheets().rules().count(), 1);
}

#[test]
fn a_stylesheet_with_never_matching_media_is_not_absorbed() {
    let mut context = context();
    let navigation = load(&mut context, "<body>hi</body>", &page("a.html"));

    let effects = context.complete(stylesheet_result(
        navigation,
        MediaQuery::never(),
        "body { background: red; }",
    ));

    assert!(effects.is_empty());
    assert_eq!(context.stats().stylesheets_loaded(), 0);
}

#[test]
fn a_decoded_image_replaces_its_placeholder_and_damages_the_content() {
    let mut context = context();
    let navigation = load(&mut context, "<body>hi</body>", &page("a.html"));

    let effects = context.complete(image_result(navigation, 7));

    assert!(damaged(&effects));
    assert_eq!(context.stats().images_loaded(), 1);
    assert!(
        context
            .document()
            .unwrap()
            .images()
            .contains(ImageId::new(7))
    );
}

#[test]
fn a_failed_subresource_is_logged_and_changes_nothing() {
    let mut context = context();
    let navigation = load(&mut context, "<body>hi</body>", &page("a.html"));
    let failed = FetchResult::new(
        navigation,
        FetchPayload::Image(ImageId::new(1), Err(FetchFailure::new("404"))),
    );

    assert!(context.complete(failed).is_empty());
    assert_eq!(context.stats().images_loaded(), 0);
}

#[test]
fn a_completed_layout_mints_regions_under_a_new_epoch_each_time() {
    let mut context = context();
    load(&mut context, "<body>hi</body>", &page("a.html"));

    let first = context.layout_completed(vec![link("/a")]);
    let second = context.layout_completed(vec![link("/a")]);

    assert_ne!(first, second);
    assert_eq!(context.content_regions().count(), 1);
}

#[test]
fn following_a_link_resolves_against_the_base_and_supersedes_what_is_in_flight() {
    let mut context = context();
    let current = load(
        &mut context,
        r#"<head><base href="https://cdn.example/app/"></head><body>x</body>"#,
        &url("https://example.com/index.html"),
    );
    context.layout_completed(vec![link("docs.html")]);
    let region = context.content_regions().next().unwrap().id();
    let target = url("https://cdn.example/app/docs.html");

    let effects = context.follow_link(region);

    let navigation = started(&effects);
    assert_eq!(
        effects,
        vec![
            BrowserEffect::NavigationStarted(ContextId::ROOT, target.clone()),
            BrowserEffect::Fetch(FetchRequest::Document(target), navigation),
        ]
    );
    assert!(
        context
            .complete(stylesheet_result(current, MediaQuery::always(), "p {}"))
            .is_empty(),
        "a click that navigates cuts off everything still in flight"
    );
}

#[test]
fn following_an_in_page_anchor_does_nothing() {
    let mut context = context();
    load(&mut context, "<body>x</body>", &page("a.html"));
    context.layout_completed(vec![link("#section")]);
    let region = context.content_regions().next().unwrap().id();
    let before = context.current_navigation();

    assert!(context.follow_link(region).is_empty());
    assert_eq!(
        context.current_navigation(),
        before,
        "no navigation started"
    );
}

#[test]
fn a_link_followed_with_a_stale_layout_epoch_is_ignored() {
    let mut context = context();
    load(&mut context, "<body>old</body>", &page("old.html"));
    context.layout_completed(vec![link("/old-target")]);
    let stale = context.content_regions().next().unwrap().id();
    // The document is swapped but nothing has laid it out yet.
    load(&mut context, "<body>new</body>", &page("new.html"));
    let before = context.current_navigation();

    let effects = context.follow_link(stale);

    assert!(
        effects.is_empty(),
        "a click on the old layout must go nowhere"
    );
    assert_eq!(context.current_navigation(), before);
}

#[test]
fn a_document_swap_forgets_the_old_documents_regions() {
    let mut context = context();
    load(&mut context, "<body>old</body>", &page("old.html"));
    context.layout_completed(vec![link("/a"), link("/b")]);

    load(&mut context, "<body>new</body>", &page("new.html"));

    assert_eq!(context.content_regions().count(), 0);
}

#[test]
fn a_link_followed_before_any_document_is_ignored() {
    let mut context = context();
    context.layout_completed(vec![link("/a")]);
    let region = context.content_regions().next().unwrap().id();

    assert!(context.follow_link(region).is_empty());
}

#[test]
fn a_region_from_before_a_relayout_does_not_follow_the_new_layouts_link_at_its_index() {
    let mut context = context();
    load(&mut context, "<body>x</body>", &page("a.html"));
    context.layout_completed(vec![link("/before")]);
    let before = context.content_regions().next().unwrap().id();
    context.layout_completed(vec![link("/after")]);

    assert!(context.follow_link(before).is_empty());
}

//! Pointer clicks through the pump: links, chrome and navigation supersession.
#![allow(clippy::unwrap_used)]

use browser::{FetchPayload, FetchResult};
use ui::OmniboxTheme;
use window::{HeadlessWindowSystem, RecordingPresenter, WindowEvent, WindowSystem as _};

use super::test_support::{
    FakeWeb, TestApp, app_over, completed_navigation, deliver, load, loaded_app, pump,
};
use crate::composition::initial_window_attributes;

/// A page whose only content is a 100x50 block link at the top.
const LINK_PAGE: &str = concat!(
    "<html><body><a href=\"target.html\" ",
    "style=\"display: block; width: 100px; height: 50px;\">Click me</a></body></html>"
);

#[test]
fn clicking_a_link_triggers_navigation_to_resolved_url() {
    let attributes = initial_window_attributes().unwrap();
    let mut system = HeadlessWindowSystem::new();
    system.create_window(&attributes).unwrap();

    let mut presenter = RecordingPresenter::new();
    let target_url = network::Url::parse("http://example.com/target.html").unwrap();
    let mut app = app_over(
        FakeWeb::new().with_page(target_url, "<html><body>target</body></html>"),
        attributes.initial_size(),
    );
    load(&mut app, LINK_PAGE, "http://example.com/index.html");

    // First pump: renders the document and collects its link areas
    pump(&mut system, &mut presenter, &mut app);

    assert!(
        app.browsing.content_regions().next().is_some(),
        "link target must be collected"
    );

    // Move pointer over the link and click
    let navigations_before_click = app.stats().navigations();
    system.schedule(WindowEvent::PointerMoved {
        position: window::PhysicalPosition::new(20.0, window_y(20.0)),
    });
    system.schedule(WindowEvent::PointerButton {
        button: window::PointerButton::Left,
        pressed: true,
    });

    pump(&mut system, &mut presenter, &mut app);

    let expected = network::Url::parse("http://example.com/target.html").unwrap();
    assert_eq!(
        completed_navigation(
            &mut system,
            &mut presenter,
            &mut app,
            navigations_before_click
        ),
        expected
    );
}

#[test]
fn a_link_click_resolves_against_the_documents_base_href_not_the_navigation_url() {
    let attributes = initial_window_attributes().unwrap();
    let mut system = HeadlessWindowSystem::new();
    system.create_window(&attributes).unwrap();

    let mut presenter = RecordingPresenter::new();
    let expected = network::Url::parse("https://cdn.example/app/docs.html").unwrap();
    let mut app = app_over(
        FakeWeb::new().with_page(expected.clone(), "<html><body>docs</body></html>"),
        attributes.initial_size(),
    );
    load(
        &mut app,
        "<html><head><base href=\"https://cdn.example/app/\"></head><body>\
         <a href=\"docs.html\" style=\"display: block; width: 100px; height: 50px;\">Docs</a>\
         </body></html>",
        "https://example.com/index.html",
    );

    assert_eq!(
        app.browsing
            .document()
            .and_then(browser::Document::base_url)
            .map(ToString::to_string)
            .as_deref(),
        Some("https://cdn.example/app/"),
        "the app keeps the `<base href>`, not the navigation URL"
    );

    pump(&mut system, &mut presenter, &mut app);
    let navigations_before_click = app.stats().navigations();
    system.schedule(WindowEvent::PointerMoved {
        position: window::PhysicalPosition::new(20.0, window_y(20.0)),
    });
    system.schedule(WindowEvent::PointerButton {
        button: window::PointerButton::Left,
        pressed: true,
    });
    pump(&mut system, &mut presenter, &mut app);

    assert_eq!(
        completed_navigation(
            &mut system,
            &mut presenter,
            &mut app,
            navigations_before_click
        ),
        expected
    );
}

/// Where a point `document_y` pixels into the page sits in the window: the
/// page starts below the default theme's chrome bar.
fn window_y(document_y: f64) -> f64 {
    f64::from(OmniboxTheme::DEFAULT.chrome_height().to_px().get()) + document_y
}

fn page_url(path: &str) -> String {
    format!("http://example.com/{path}")
}

fn pump_queued(app: &mut TestApp) {
    let attributes = initial_window_attributes().unwrap();
    let mut system = HeadlessWindowSystem::new();
    system.create_window(&attributes).unwrap();
    let mut presenter = RecordingPresenter::new();
    pump(&mut system, &mut presenter, app);
}

/// `NavigationId` supersession is proved purely in `core/browser`; this
/// proves the app routes worker results through it.
#[test]
fn the_session_drops_a_result_stamped_for_a_navigation_the_user_left() {
    let mut app = loaded_app(initial_window_attributes().unwrap().initial_size());
    let left = app.browsing.current_navigation();
    app.navigate(network::Url::parse(&page_url("next.html")).unwrap());
    let late_sheet =
        FetchPayload::Stylesheet(css::MediaQuery::always(), Ok("p { color: red }".into()));
    deliver(&app, FetchResult::new(left, late_sheet));

    pump_queued(&mut app);

    assert_eq!(app.stats().stylesheets_loaded(), 0);
    assert_ne!(app.browsing.current_navigation(), left);
}

#[test]
fn clicks_in_chrome_region_target_omnibox_and_do_not_trigger_links() {
    let attributes = initial_window_attributes().unwrap();
    let mut system = HeadlessWindowSystem::new();
    system.create_window(&attributes).unwrap();
    let mut presenter = RecordingPresenter::new();
    let mut app = app_over(FakeWeb::new(), attributes.initial_size());
    load(&mut app, LINK_PAGE, &page_url("index.html"));
    pump(&mut system, &mut presenter, &mut app);
    assert!(!app.ui.omnibox().is_focused());
    let navigations_before_click = app.stats().navigations();

    // y = 20 is inside the chrome bar, above the link at the top of the page.
    system.schedule(WindowEvent::PointerMoved {
        position: window::PhysicalPosition::new(20.0, 20.0),
    });
    system.schedule(WindowEvent::PointerButton {
        button: window::PointerButton::Left,
        pressed: true,
    });
    pump(&mut system, &mut presenter, &mut app);

    assert!(
        app.ui.omnibox().is_focused(),
        "clicking within the chrome bar must focus the omnibox"
    );
    assert_eq!(
        app.stats().navigations(),
        navigations_before_click,
        "clicking in the chrome area must not trigger links underneath in the document"
    );
}

#[test]
fn clicks_below_chrome_region_trigger_link_with_offset_compensation() {
    let attributes = initial_window_attributes().unwrap();
    let mut system = HeadlessWindowSystem::new();
    system.create_window(&attributes).unwrap();
    let mut presenter = RecordingPresenter::new();
    let target = network::Url::parse("http://example.com/target.html").unwrap();
    let mut app = app_over(
        FakeWeb::new().with_page(target.clone(), "<html><body>target</body></html>"),
        attributes.initial_size(),
    );
    load(&mut app, LINK_PAGE, "http://example.com/index.html");
    pump(&mut system, &mut presenter, &mut app);
    let navigations_before_click = app.stats().navigations();

    system.schedule(WindowEvent::PointerMoved {
        position: window::PhysicalPosition::new(20.0, window_y(20.0)),
    });
    system.schedule(WindowEvent::PointerButton {
        button: window::PointerButton::Left,
        pressed: true,
    });
    pump(&mut system, &mut presenter, &mut app);

    assert_eq!(
        completed_navigation(
            &mut system,
            &mut presenter,
            &mut app,
            navigations_before_click
        ),
        target,
        "clicking below chrome height must trigger the link with offset compensation"
    );
}

/// Whether `point` (document space) lies inside `area`.
fn covers(area: graphics::Rect, point: graphics::Point) -> bool {
    let (x, y) = (point.horizontal(), point.vertical());
    area.min_x() <= x && x < area.max_x() && area.min_y() <= y && y < area.max_y()
}

#[test]
fn a_link_that_wraps_has_one_region_per_line_and_none_over_the_text_before_it() {
    let attributes = initial_window_attributes().unwrap();
    let mut system = HeadlessWindowSystem::new();
    system.create_window(&attributes).unwrap();
    let mut presenter = RecordingPresenter::new();
    let mut app = app_over(FakeWeb::new(), attributes.initial_size());
    load(
        &mut app,
        "<html><body style=\"margin: 0\"><div style=\"width: 90px\">xx \
         <a href=\"next.html\">bbbb cccc dddd eeee ffff gggg</a></div></body></html>",
        "http://example.com/index.html",
    );
    pump(&mut system, &mut presenter, &mut app);

    let areas: Vec<graphics::Rect> = app
        .browsing
        .content_regions()
        .map(|region| region.bounds())
        .collect();
    let first_line = areas
        .iter()
        .min_by_key(|area| area.min_y())
        .copied()
        .unwrap();
    assert!(
        first_line.min_x() > graphics::Au::ZERO,
        "the link's first line starts after \"xx \""
    );
    assert!(
        areas.iter().any(|area| area.min_y() >= first_line.max_y()),
        "the link wraps onto further lines, each with its own region"
    );

    let middle =
        graphics::Au::from_raw(first_line.min_y().raw() + first_line.size().height().raw() / 2);
    let over_xx = graphics::Point::new(graphics::Au::from_raw(1), middle);
    assert!(
        !areas.iter().any(|area| covers(*area, over_xx)),
        "a click on \"xx\", before the link on its first line, must miss it"
    );
    let on_link = graphics::Point::new(first_line.min_x(), middle);
    assert!(areas.iter().any(|area| covers(*area, on_link)));
}

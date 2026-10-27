use graphics::{Au, Point, Rect, Size};
use region::{ContentRegion, ContextId, LayoutEpoch, RegionId};
use window::{PhysicalPosition, SurfaceSize};

use super::{UiHitTarget, UiNode, UiNodeChildren, UiNodeKind, UiRenderTree};
use crate::domain::theme::{ChromeHeight, OmniboxTheme};

/// A link area in document space.
fn link(left: i32, top: i32, width: i32, height: i32) -> Rect {
    let au = |pixels| Au::from_whole_px(pixels).expect("fits the Au envelope");
    Rect::new(
        Point::new(au(left), au(top)),
        Size::new(au(width), au(height)).expect("non-negative size"),
    )
}

/// The regions for `links`, numbered in paint order the way the browser would.
fn minted(links: Vec<Rect>) -> Vec<ContentRegion> {
    links
        .into_iter()
        .enumerate()
        .map(|(index, bounds)| {
            let id = RegionId::new(ContextId::ROOT, LayoutEpoch::INITIAL.next(), index);
            ContentRegion::new(id, bounds)
        })
        .collect()
}

/// The window `y` of a point `document_y` pixels into the page, which
/// starts where the theme's chrome bar ends.
fn window_y(document_y: f64) -> f64 {
    f64::from(OmniboxTheme::DEFAULT.chrome_height().to_px().get()) + document_y
}

fn sample_tree(regions: &[ContentRegion]) -> UiRenderTree {
    let size = SurfaceSize::new(800, 600).expect("valid size");
    UiRenderTree::build(size, &OmniboxTheme::DEFAULT, regions.iter().copied()).expect("valid tree")
}

#[test]
fn tree_construction_sets_correct_bounds_for_all_elements() {
    let regions = minted(vec![link(10, 10, 20, 20)]);
    let tree = sample_tree(&regions);

    assert_eq!(tree.root().kind(), &UiNodeKind::Window);
    assert_eq!(tree.root().children().len(), 2);

    let chrome = &tree.root().children().iter().next().unwrap();
    assert_eq!(chrome.kind(), &UiNodeKind::Chrome);
    assert_eq!(chrome.children().len(), 1);

    let omnibox = &chrome.children().iter().next().unwrap();
    assert_eq!(omnibox.kind(), &UiNodeKind::Omnibox);

    let content = &tree.root().children().iter().nth(1).unwrap();
    assert_eq!(content.kind(), &UiNodeKind::Content);
    assert_eq!(content.children().len(), 1);

    let link_node = &content.children().iter().next().unwrap();
    assert_eq!(link_node.kind(), &UiNodeKind::Link(regions[0].id()));
}

#[test]
fn click_in_omnibox_returns_omnibox_target() {
    let tree = sample_tree(&[]);
    let hit = tree.hit_test(PhysicalPosition::new(20.0, 20.0));
    assert_eq!(hit, Some(UiHitTarget::Omnibox));
}

#[test]
fn click_in_chrome_margin_returns_chrome_target() {
    let tree = sample_tree(&[]);
    // x = 4 lies in the chrome's left margin, before the omnibox input box begins
    let hit = tree.hit_test(PhysicalPosition::new(4.0, 20.0));
    assert_eq!(hit, Some(UiHitTarget::Chrome));
}

#[test]
fn click_in_content_link_returns_link_target() {
    let regions = minted(vec![link(10, 10, 20, 20)]);
    let tree = sample_tree(&regions);

    let hit = tree.hit_test(PhysicalPosition::new(20.0, window_y(20.0)));
    assert_eq!(hit, Some(UiHitTarget::PageLink(regions[0].id())));
}

#[test]
fn click_in_content_outside_links_returns_content_target() {
    let regions = minted(vec![link(10, 10, 20, 20)]);
    let tree = sample_tree(&regions);

    // Click outside the link area
    let hit = tree.hit_test(PhysicalPosition::new(100.0, 200.0));
    assert_eq!(hit, Some(UiHitTarget::PageContent));
}

#[test]
fn click_in_chrome_does_not_hit_link_positioned_at_document_y_zero() {
    let regions = minted(vec![link(0, 0, 100, 50)]);
    let tree = sample_tree(&regions);

    // Click at window y = 20.0 (in chrome)
    let hit = tree.hit_test(PhysicalPosition::new(20.0, 20.0));
    assert_eq!(
        hit,
        Some(UiHitTarget::Omnibox),
        "chrome must shield document links positioned at top"
    );
}

#[test]
fn later_painted_overlapping_link_wins() {
    let regions = minted(vec![link(0, 0, 50, 50), link(10, 10, 10, 10)]);
    let tree = sample_tree(&regions);

    // Inside the overlapping area, at document (15, 15).
    assert_eq!(
        tree.hit_test(PhysicalPosition::new(15.0, window_y(15.0))),
        Some(UiHitTarget::PageLink(regions[1].id()))
    );

    // Outside the top link, inside the bottom one, at document (40, 40).
    assert_eq!(
        tree.hit_test(PhysicalPosition::new(40.0, window_y(40.0))),
        Some(UiHitTarget::PageLink(regions[0].id()))
    );
}

#[test]
fn chrome_geometry_follows_the_theme_it_is_built_with() {
    let tall_chrome = OmniboxTheme::DEFAULT.with_chrome_height(ChromeHeight::from_au(
        Au::from_whole_px(60).expect("fits the Au envelope"),
    ));
    let regions = minted(vec![link(0, 0, 100, 50)]);
    let size = SurfaceSize::new(800, 600).expect("valid size");
    let tree =
        UiRenderTree::build(size, &tall_chrome, regions.iter().copied()).expect("valid tree");

    assert_eq!(
        tree.hit_test(PhysicalPosition::new(4.0, 50.0)),
        Some(UiHitTarget::Chrome),
        "a 60 px chrome bar must still cover y = 50"
    );
    assert_eq!(
        tree.hit_test(PhysicalPosition::new(20.0, 65.0)),
        Some(UiHitTarget::PageLink(regions[0].id())),
        "the page must start where the themed chrome bar ends"
    );
    let omnibox_bounds = tall_chrome.input_box_bounds(Au::from_whole_px(800).expect("fits"));
    let omnibox_node = tree
        .root()
        .children()
        .iter()
        .next()
        .and_then(|chrome| chrome.children().iter().next().map(UiNode::bounds));
    assert_eq!(
        omnibox_node, omnibox_bounds,
        "the hit-tested omnibox must be the box the painter draws"
    );
}

#[test]
fn non_finite_coordinates_return_none() {
    let regions = minted(vec![link(0, 0, 50, 50)]);
    let tree = sample_tree(&regions);

    assert_eq!(tree.hit_test(PhysicalPosition::new(f64::NAN, 50.0)), None);
    assert_eq!(
        tree.hit_test(PhysicalPosition::new(5.0, f64::INFINITY)),
        None
    );
    assert_eq!(
        tree.hit_test(PhysicalPosition::new(f64::NAN, f64::NAN)),
        None
    );
}

#[test]
fn out_of_window_coordinates_return_none() {
    let tree = sample_tree(&[]);

    assert_eq!(tree.hit_test(PhysicalPosition::new(-5.0, 20.0)), None);
    assert_eq!(tree.hit_test(PhysicalPosition::new(850.0, 20.0)), None);
    assert_eq!(tree.hit_test(PhysicalPosition::new(20.0, 650.0)), None);
}

#[test]
fn ui_node_children_from_iter_and_reverse_iteration() {
    let bounds = Rect::default();
    let first = UiNode::new(UiNodeKind::Window, bounds);
    let second = UiNode::new(UiNodeKind::Chrome, bounds);
    let children: UiNodeChildren = [first, second].into_iter().collect();

    assert_eq!(children.len(), 2);
    assert!(!children.is_empty());

    let last_kind = children.reverse_iter().next().map(UiNode::kind);
    assert_eq!(last_kind, Some(&UiNodeKind::Chrome));
}

#[test]
fn narrow_viewport_handles_zero_box_width_without_panic() {
    let narrow_size = SurfaceSize::new(10, 100).expect("positive surface size");
    let tree = UiRenderTree::build(narrow_size, &OmniboxTheme::DEFAULT, []).expect("valid tree");
    // Too narrow for the input box, so (5, 20) hits the bare chrome bar.
    assert_eq!(
        tree.hit_test(PhysicalPosition::new(5.0, 20.0)),
        Some(UiHitTarget::Chrome)
    );
}

#[test]
fn fallback_tree_returns_none_for_all_positions() {
    let size = SurfaceSize::new(800, 30).expect("positive size");
    let fallback = UiRenderTree::fallback(size, &OmniboxTheme::DEFAULT);
    assert_eq!(fallback.viewport(), size);
    assert_eq!(fallback.hit_test(PhysicalPosition::new(20.0, 20.0)), None);
}

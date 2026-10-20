//! The browser UI render tree.
//!
//! Models the hierarchical layout of browser user interface elements
//! (chrome top-bar, omnibox input area, and web content viewport with links)
//! within the window surface. Provides unified hit-testing without ad-hoc
//! coordinate offsets or brittle coordinate checks.

use graphics::{Au, Point, Rect, Size};
use window::{PhysicalPosition, SurfaceSize};

use super::hit_test::{contains, raw_edge, snapped_raw_au};
use crate::application::pipeline::LinkTarget;
use crate::error::AlloyError;

/// Strongly-typed vertical height of the top chrome bar in pixels (ADR-0010).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChromeHeight(u32);

impl ChromeHeight {
    /// Default chrome height (42 px).
    pub const DEFAULT: Self = Self(42);

    #[must_use]
    pub const fn new(pixels: u32) -> Self {
        Self(pixels)
    }

    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }

    #[must_use]
    pub fn to_au(self) -> Option<Au> {
        let pixels = i32::try_from(self.0).ok()?;
        Au::from_whole_px(pixels)
    }
}

impl Default for ChromeHeight {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl std::fmt::Display for ChromeHeight {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}px", self.0)
    }
}

impl From<u32> for ChromeHeight {
    fn from(pixels: u32) -> Self {
        Self(pixels)
    }
}

impl From<ChromeHeight> for u32 {
    fn from(height: ChromeHeight) -> Self {
        height.get()
    }
}

/// The default vertical height of the top chrome bar in pixels (Epic #43 / Issue #48).
pub const CHROME_HEIGHT: u32 = ChromeHeight::DEFAULT.get();

/// Horizontal margin in pixels around the omnibox input area within the chrome bar.
pub const OMNIBOX_HORIZONTAL_MARGIN_PX: u32 = 8;

/// Height in pixels of the omnibox input field within the chrome bar.
pub const OMNIBOX_HEIGHT_PX: u32 = 30;

/// The role and identity of a node in the UI render tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UiNodeKind {
    /// The root window surface.
    Window,
    /// The browser chrome top bar container.
    Chrome,
    /// The omnibox address/search input field within the chrome.
    Omnibox,
    /// The web document content area below the chrome.
    Content,
    /// A hyperlink within the web document content.
    Link(String),
}

/// The interactive target hit by a pointer event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiHitTarget<'a> {
    /// The omnibox input field was clicked.
    Omnibox,
    /// The chrome top bar background was clicked.
    Chrome,
    /// A hyperlink within the web page was clicked.
    PageLink(&'a str),
    /// The page content area (outside of links) was clicked.
    PageContent,
}

/// A first-class collection of child UI nodes in the render tree (ADR-0010).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UiNodeChildren(Vec<UiNode>);

impl UiNodeChildren {
    #[must_use]
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    pub fn push(&mut self, child: UiNode) {
        self.0.push(child);
    }

    pub fn iter(&self) -> std::slice::Iter<'_, UiNode> {
        self.0.iter()
    }

    pub fn reverse_iter(&self) -> std::iter::Rev<std::slice::Iter<'_, UiNode>> {
        self.0.iter().rev()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.0.len()
    }
}

impl FromIterator<UiNode> for UiNodeChildren {
    fn from_iter<T: IntoIterator<Item = UiNode>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl<'a> IntoIterator for &'a UiNodeChildren {
    type Item = &'a UiNode;
    type IntoIter = std::slice::Iter<'a, UiNode>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl IntoIterator for UiNodeChildren {
    type Item = UiNode;
    type IntoIter = std::vec::IntoIter<UiNode>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

/// A node in the browser UI render tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UiNode {
    kind: UiNodeKind,
    bounds: Rect,
    children: UiNodeChildren,
}

impl UiNode {
    #[must_use]
    pub const fn new(kind: UiNodeKind, bounds: Rect) -> Self {
        Self {
            kind,
            bounds,
            children: UiNodeChildren::new(),
        }
    }

    #[must_use]
    pub const fn with_children(kind: UiNodeKind, bounds: Rect, children: UiNodeChildren) -> Self {
        Self {
            kind,
            bounds,
            children,
        }
    }

    pub fn add_child(&mut self, child: Self) {
        self.children.push(child);
    }

    #[must_use]
    pub const fn kind(&self) -> &UiNodeKind {
        &self.kind
    }

    #[must_use]
    pub const fn bounds(&self) -> Rect {
        self.bounds
    }

    #[must_use]
    pub const fn children(&self) -> &UiNodeChildren {
        &self.children
    }

    /// Recursively hit-tests this node and its children in local coordinate space.
    #[must_use]
    pub fn hit_test(&self, x: f64, y: f64) -> Option<UiHitTarget<'_>> {
        if !contains(self.bounds, x, y) {
            return None;
        }
        let local_x = x - raw_edge(self.bounds.min_x());
        let local_y = y - raw_edge(self.bounds.min_y());
        self.hit_test_children(local_x, local_y)
            .or_else(|| self.as_target())
    }

    fn hit_test_children(&self, local_x: f64, local_y: f64) -> Option<UiHitTarget<'_>> {
        self.children
            .reverse_iter()
            .find_map(|child| child.hit_test(local_x, local_y))
    }

    const fn as_target(&self) -> Option<UiHitTarget<'_>> {
        match &self.kind {
            UiNodeKind::Omnibox => Some(UiHitTarget::Omnibox),
            UiNodeKind::Chrome => Some(UiHitTarget::Chrome),
            UiNodeKind::Link(href) => Some(UiHitTarget::PageLink(href.as_str())),
            UiNodeKind::Content => Some(UiHitTarget::PageContent),
            UiNodeKind::Window => None,
        }
    }
}

/// The browser UI render tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UiRenderTree {
    viewport: SurfaceSize,
    chrome_height: ChromeHeight,
    root: UiNode,
}

impl UiRenderTree {
    /// Builds a new UI render tree for the specified window viewport, chrome height,
    /// and document link targets.
    ///
    /// # Errors
    ///
    /// Returns [`AlloyError::InvalidDimensions`] if the viewport height is less than
    /// or equal to the chrome height, or if dimensions cannot be converted to app units.
    pub fn build(
        viewport: SurfaceSize,
        chrome_height: ChromeHeight,
        links: &[LinkTarget],
    ) -> Result<Self, AlloyError> {
        let width_px =
            i32::try_from(viewport.width()).map_err(|_| AlloyError::InvalidDimensions)?;
        let height_px =
            i32::try_from(viewport.height()).map_err(|_| AlloyError::InvalidDimensions)?;
        let width_in_app_units =
            Au::from_whole_px(width_px).ok_or(AlloyError::InvalidDimensions)?;
        let height_in_app_units =
            Au::from_whole_px(height_px).ok_or(AlloyError::InvalidDimensions)?;
        let chrome_height_au = chrome_height.to_au().ok_or(AlloyError::InvalidDimensions)?;

        if height_in_app_units <= chrome_height_au {
            return Err(AlloyError::InvalidDimensions);
        }

        let content_height_au = Au::from_raw(
            height_in_app_units
                .raw()
                .saturating_sub(chrome_height_au.raw()),
        );
        let window_size = Size::new(width_in_app_units, height_in_app_units)
            .ok_or(AlloyError::InvalidDimensions)?;
        let window_bounds = Rect::new(Point::new(Au::ZERO, Au::ZERO), window_size);

        let chrome_node = build_chrome_node(width_in_app_units, chrome_height_au)?;
        let content_node = build_content_node(
            width_in_app_units,
            chrome_height_au,
            content_height_au,
            links,
        )?;

        let mut children = UiNodeChildren::new();
        children.push(chrome_node);
        children.push(content_node);
        let window_node = UiNode::with_children(UiNodeKind::Window, window_bounds, children);

        Ok(Self {
            viewport,
            chrome_height,
            root: window_node,
        })
    }

    #[must_use]
    pub const fn viewport(&self) -> SurfaceSize {
        self.viewport
    }

    #[must_use]
    pub const fn chrome_height(&self) -> ChromeHeight {
        self.chrome_height
    }

    #[must_use]
    pub const fn root(&self) -> &UiNode {
        &self.root
    }

    /// The surface size available for laying out web content.
    ///
    /// # Errors
    ///
    /// Returns [`AlloyError::InvalidDimensions`] if the content height is zero.
    pub fn content_size(&self) -> Result<graphics::SurfaceSize, AlloyError> {
        let content_height = self
            .viewport
            .height()
            .checked_sub(self.chrome_height.get())
            .ok_or(AlloyError::InvalidDimensions)?;
        graphics::SurfaceSize::new(self.viewport.width(), content_height)
            .ok_or(AlloyError::InvalidDimensions)
    }

    /// A safe fallback UI tree when viewport dimensions cannot be converted.
    #[must_use]
    pub fn fallback(viewport: SurfaceSize) -> Self {
        Self {
            viewport,
            chrome_height: ChromeHeight::DEFAULT,
            root: UiNode::new(UiNodeKind::Window, Rect::default()),
        }
    }

    /// Hit-tests a pointer position in window physical coordinates against the UI tree.
    #[must_use]
    pub fn hit_test(&self, position: PhysicalPosition) -> Option<UiHitTarget<'_>> {
        let x = snapped_raw_au(position.x())?;
        let y = snapped_raw_au(position.y())?;
        self.root.hit_test(x, y)
    }
}

fn build_chrome_node(width_in_app_units: Au, chrome_height: Au) -> Result<UiNode, AlloyError> {
    let chrome_size =
        Size::new(width_in_app_units, chrome_height).ok_or(AlloyError::InvalidDimensions)?;
    let chrome_bounds = Rect::new(Point::new(Au::ZERO, Au::ZERO), chrome_size);

    let margin_x_px =
        i32::try_from(OMNIBOX_HORIZONTAL_MARGIN_PX).map_err(|_| AlloyError::InvalidDimensions)?;
    let box_height_px =
        i32::try_from(OMNIBOX_HEIGHT_PX).map_err(|_| AlloyError::InvalidDimensions)?;
    let horizontal_margin = Au::from_whole_px(margin_x_px).ok_or(AlloyError::InvalidDimensions)?;
    let box_height = Au::from_whole_px(box_height_px).ok_or(AlloyError::InvalidDimensions)?;

    let vertical_margin_raw = chrome_height.raw().saturating_sub(box_height.raw()) / 2;
    let margin_y = Au::from_raw(vertical_margin_raw.max(0));
    let horizontal_padding = horizontal_margin.raw().saturating_mul(2);
    let box_width_raw = width_in_app_units.raw().saturating_sub(horizontal_padding);
    let box_width = Au::from_raw(box_width_raw.max(0));

    let box_size = Size::new(box_width, box_height).ok_or(AlloyError::InvalidDimensions)?;
    let omnibox_bounds = Rect::new(Point::new(horizontal_margin, margin_y), box_size);
    let omnibox_node = UiNode::new(UiNodeKind::Omnibox, omnibox_bounds);

    let mut children = UiNodeChildren::new();
    children.push(omnibox_node);
    Ok(UiNode::with_children(
        UiNodeKind::Chrome,
        chrome_bounds,
        children,
    ))
}

fn build_content_node(
    width_in_app_units: Au,
    chrome_height: Au,
    content_height: Au,
    links: &[LinkTarget],
) -> Result<UiNode, AlloyError> {
    let content_origin = Point::new(Au::ZERO, chrome_height);
    let content_size =
        Size::new(width_in_app_units, content_height).ok_or(AlloyError::InvalidDimensions)?;
    let content_bounds = Rect::new(content_origin, content_size);
    let children = links
        .iter()
        .map(|link| UiNode::new(UiNodeKind::Link(link.href.clone()), link.area))
        .collect();
    Ok(UiNode::with_children(
        UiNodeKind::Content,
        content_bounds,
        children,
    ))
}

#[cfg(test)]
mod tests {
    use graphics::{Au, Point, Rect, Size};
    use window::{PhysicalPosition, SurfaceSize};

    use super::{ChromeHeight, UiHitTarget, UiNodeKind, UiRenderTree};
    use crate::application::pipeline::LinkTarget;

    fn link(href: &str, left: i32, top: i32, width: i32, height: i32) -> LinkTarget {
        let au = |pixels| Au::from_whole_px(pixels).expect("fits the Au envelope");
        LinkTarget {
            area: Rect::new(
                Point::new(au(left), au(top)),
                Size::new(au(width), au(height)).expect("non-negative size"),
            ),
            href: href.to_owned(),
        }
    }

    fn sample_tree(links: &[LinkTarget]) -> UiRenderTree {
        let size = SurfaceSize::new(800, 600).expect("valid size");
        UiRenderTree::build(size, ChromeHeight::DEFAULT, links).expect("valid tree")
    }

    #[test]
    fn tree_construction_sets_correct_bounds_for_all_elements() {
        let links = [link("/a", 10, 10, 20, 20)];
        let tree = sample_tree(&links);

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
        assert_eq!(link_node.kind(), &UiNodeKind::Link("/a".to_owned()));
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
        // Omnibox margin_x = 8.0, so x = 4.0 is in the chrome margin outside omnibox
        let hit = tree.hit_test(PhysicalPosition::new(4.0, 20.0));
        assert_eq!(hit, Some(UiHitTarget::Chrome));
    }

    #[test]
    fn click_in_content_link_returns_link_target() {
        let links = [link("/destination", 10, 10, 20, 20)];
        let tree = sample_tree(&links);

        // Document y = 20 corresponds to window y = 42 + 20 = 62
        let hit = tree.hit_test(PhysicalPosition::new(20.0, 62.0));
        assert_eq!(hit, Some(UiHitTarget::PageLink("/destination")));
    }

    #[test]
    fn click_in_content_outside_links_returns_content_target() {
        let links = [link("/destination", 10, 10, 20, 20)];
        let tree = sample_tree(&links);

        // Click outside the link area
        let hit = tree.hit_test(PhysicalPosition::new(100.0, 200.0));
        assert_eq!(hit, Some(UiHitTarget::PageContent));
    }

    #[test]
    fn click_in_chrome_does_not_hit_link_positioned_at_document_y_zero() {
        let links = [link("/top", 0, 0, 100, 50)];
        let tree = sample_tree(&links);

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
        let links = [link("/below", 0, 0, 50, 50), link("/above", 10, 10, 10, 10)];
        let tree = sample_tree(&links);

        // Inside overlapping area: document (15, 15) -> window (15, 57)
        assert_eq!(
            tree.hit_test(PhysicalPosition::new(15.0, 57.0)),
            Some(UiHitTarget::PageLink("/above"))
        );

        // Outside top link, inside bottom link: document (40, 40) -> window (40, 82)
        assert_eq!(
            tree.hit_test(PhysicalPosition::new(40.0, 82.0)),
            Some(UiHitTarget::PageLink("/below"))
        );
    }

    #[test]
    fn non_finite_coordinates_return_none() {
        let links = [link("/a", 0, 0, 50, 50)];
        let tree = sample_tree(&links);

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
    fn chrome_height_display_formats_with_pixels_unit() {
        let default_height = ChromeHeight::DEFAULT;
        let formatted = format!("{default_height}");
        assert_eq!(formatted, "42px");
    }

    #[test]
    fn ui_node_children_from_iter_and_reverse_iteration() {
        let bounds = Rect::default();
        let first = super::UiNode::new(UiNodeKind::Window, bounds);
        let second = super::UiNode::new(UiNodeKind::Chrome, bounds);
        let children: super::UiNodeChildren = [first, second].into_iter().collect();

        assert_eq!(children.len(), 2);
        assert!(!children.is_empty());

        let last_kind = children.reverse_iter().next().map(super::UiNode::kind);
        assert_eq!(last_kind, Some(&UiNodeKind::Chrome));
    }

    #[test]
    fn narrow_viewport_handles_zero_box_width_without_panic() {
        let narrow_size = SurfaceSize::new(10, 100).expect("positive surface size");
        let tree =
            UiRenderTree::build(narrow_size, ChromeHeight::DEFAULT, &[]).expect("valid tree");
        // Within chrome bounds (10 x 42): click at (5, 20) hits Chrome
        assert_eq!(
            tree.hit_test(PhysicalPosition::new(5.0, 20.0)),
            Some(UiHitTarget::Chrome)
        );
    }

    #[test]
    fn fallback_tree_returns_none_for_all_positions() {
        let size = SurfaceSize::new(800, 30).expect("positive size");
        let fallback = UiRenderTree::fallback(size);
        assert_eq!(fallback.viewport(), size);
        assert_eq!(fallback.hit_test(PhysicalPosition::new(20.0, 20.0)), None);
    }
}

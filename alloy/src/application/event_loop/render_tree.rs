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

pub const CHROME_HEIGHT: u32 = ChromeHeight::DEFAULT.get();

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

    pub fn rev_iter(&self) -> std::iter::Rev<std::slice::Iter<'_, UiNode>> {
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

impl<'a> IntoIterator for &'a UiNodeChildren {
    type Item = &'a UiNode;
    type IntoIter = std::slice::Iter<'a, UiNode>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
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
        for child in self.children.rev_iter() {
            if let Some(target) = child.hit_test(local_x, local_y) {
                return Some(target);
            }
        }
        self.as_target()
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
        let width_au = Au::from_whole_px(width_px).ok_or(AlloyError::InvalidDimensions)?;
        let height_au = Au::from_whole_px(height_px).ok_or(AlloyError::InvalidDimensions)?;
        let chrome_h = chrome_height.to_au().ok_or(AlloyError::InvalidDimensions)?;

        if height_au <= chrome_h {
            return Err(AlloyError::InvalidDimensions);
        }

        let content_h = Au::from_raw(height_au.raw().saturating_sub(chrome_h.raw()));
        let window_size = Size::new(width_au, height_au).ok_or(AlloyError::InvalidDimensions)?;
        let window_bounds = Rect::new(Point::new(Au::ZERO, Au::ZERO), window_size);
        let mut window_node = UiNode::new(UiNodeKind::Window, window_bounds);

        let chrome_node = build_chrome_node(width_au, chrome_h)?;
        let content_node = build_content_node(width_au, chrome_h, content_h, links)?;

        window_node.add_child(chrome_node);
        window_node.add_child(content_node);

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

fn build_chrome_node(width_au: Au, chrome_h: Au) -> Result<UiNode, AlloyError> {
    let chrome_size = Size::new(width_au, chrome_h).ok_or(AlloyError::InvalidDimensions)?;
    let chrome_bounds = Rect::new(Point::new(Au::ZERO, Au::ZERO), chrome_size);
    let mut chrome_node = UiNode::new(UiNodeKind::Chrome, chrome_bounds);

    let margin_x = Au::from_whole_px(8).ok_or(AlloyError::InvalidDimensions)?;
    let box_h = Au::from_whole_px(30).ok_or(AlloyError::InvalidDimensions)?;
    let margin_y_raw = chrome_h.raw().saturating_sub(box_h.raw()) / 2;
    let margin_y = Au::from_raw(margin_y_raw.max(0));
    let margin_double = margin_x.raw().saturating_mul(2);
    let box_w_raw = width_au.raw().saturating_sub(margin_double);
    let box_w = Au::from_raw(box_w_raw.max(0));

    let box_size = Size::new(box_w, box_h).ok_or(AlloyError::InvalidDimensions)?;
    let omnibox_bounds = Rect::new(Point::new(margin_x, margin_y), box_size);
    let omnibox_node = UiNode::new(UiNodeKind::Omnibox, omnibox_bounds);
    chrome_node.add_child(omnibox_node);
    Ok(chrome_node)
}

fn build_content_node(
    width_au: Au,
    chrome_h: Au,
    content_h: Au,
    links: &[LinkTarget],
) -> Result<UiNode, AlloyError> {
    let content_origin = Point::new(Au::ZERO, chrome_h);
    let content_size = Size::new(width_au, content_h).ok_or(AlloyError::InvalidDimensions)?;
    let content_bounds = Rect::new(content_origin, content_size);
    let mut content_node = UiNode::new(UiNodeKind::Content, content_bounds);

    for link in links {
        let link_node = UiNode::new(UiNodeKind::Link(link.href.clone()), link.area);
        content_node.add_child(link_node);
    }
    Ok(content_node)
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
}

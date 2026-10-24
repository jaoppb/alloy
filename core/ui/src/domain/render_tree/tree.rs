//! The browser UI render tree.
//!
//! Models the hierarchical layout of browser user interface elements
//! (chrome top-bar, omnibox input area, and web content viewport with links)
//! within the window surface. Provides unified hit-testing without ad-hoc
//! coordinate offsets or brittle coordinate checks.

use graphics::{Au, Point, Rect, Size};
use region::ContentRegion;
use window::{PhysicalPosition, SurfaceSize};

use super::builder::{build_chrome_node, build_content_node};
use super::hit_geometry::snapped_raw_au;
use super::hit_target::UiHitTarget;
use super::node::UiNode;
use super::node_children::UiNodeChildren;
use super::node_kind::UiNodeKind;
use super::surface_length::surface_length_to_au;
use crate::domain::error::UiError;
use crate::domain::theme::{ChromeHeight, OmniboxTheme};

/// The browser UI render tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UiRenderTree {
    viewport: SurfaceSize,
    chrome_height: ChromeHeight,
    root: UiNode,
}

impl UiRenderTree {
    /// Builds a new UI render tree for the specified window viewport and
    /// content link regions, with the chrome laid out by the same `theme`
    /// the omnibox painter draws it with.
    ///
    /// # Errors
    ///
    /// Returns [`UiError::InvalidDimensions`] if the viewport height is less than
    /// or equal to the chrome height, or if dimensions cannot be converted to app units.
    pub fn build(
        viewport: SurfaceSize,
        theme: &OmniboxTheme,
        regions: impl IntoIterator<Item = ContentRegion>,
    ) -> Result<Self, UiError> {
        let chrome_height = theme.chrome_height();
        let width_in_app_units = surface_length_to_au(viewport.width())?;
        let height_in_app_units = surface_length_to_au(viewport.height())?;
        let chrome_height_au = chrome_height.to_au();

        if height_in_app_units <= chrome_height_au {
            return Err(UiError::InvalidDimensions);
        }

        let content_height_au = Au::from_raw(
            height_in_app_units
                .raw()
                .saturating_sub(chrome_height_au.raw()),
        );
        let window_size =
            Size::new(width_in_app_units, height_in_app_units).ok_or(UiError::InvalidDimensions)?;
        let window_bounds = Rect::new(Point::new(Au::ZERO, Au::ZERO), window_size);

        let chrome_node = build_chrome_node(width_in_app_units, theme)?;
        let content_node = build_content_node(
            width_in_app_units,
            chrome_height_au,
            content_height_au,
            regions,
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

    /// A safe fallback UI tree when viewport dimensions cannot be converted.
    #[must_use]
    pub fn fallback(viewport: SurfaceSize, theme: &OmniboxTheme) -> Self {
        Self {
            viewport,
            chrome_height: theme.chrome_height(),
            root: UiNode::new(UiNodeKind::Window, Rect::default()),
        }
    }

    /// Hit-tests a pointer position in window physical coordinates against the UI tree.
    #[must_use]
    pub fn hit_test(&self, position: PhysicalPosition) -> Option<UiHitTarget> {
        let x = snapped_raw_au(position.x())?;
        let y = snapped_raw_au(position.y())?;
        self.root.hit_test(x, y)
    }
}

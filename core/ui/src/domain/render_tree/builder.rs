//! The nodes the render tree is assembled from.

use graphics::{Au, Point, Rect, Size};
use region::ContentRegion;

use super::node::UiNode;
use super::node_kind::UiNodeKind;
use crate::domain::error::UiError;
use crate::domain::theme::OmniboxTheme;

pub(super) fn build_chrome_node(
    width_in_app_units: Au,
    theme: &OmniboxTheme,
) -> Result<UiNode, UiError> {
    let chrome_size = Size::new(width_in_app_units, theme.chrome_height().to_au())
        .ok_or(UiError::InvalidDimensions)?;
    let chrome_bounds = Rect::new(Point::new(Au::ZERO, Au::ZERO), chrome_size);
    let children = theme
        .input_box_bounds(width_in_app_units)
        .map(|omnibox_bounds| UiNode::new(UiNodeKind::Omnibox, omnibox_bounds))
        .into_iter()
        .collect();
    Ok(UiNode::with_children(
        UiNodeKind::Chrome,
        chrome_bounds,
        children,
    ))
}

pub(super) fn build_content_node(
    width_in_app_units: Au,
    chrome_height: Au,
    content_height: Au,
    regions: impl IntoIterator<Item = ContentRegion>,
) -> Result<UiNode, UiError> {
    let content_origin = Point::new(Au::ZERO, chrome_height);
    let content_size =
        Size::new(width_in_app_units, content_height).ok_or(UiError::InvalidDimensions)?;
    let content_bounds = Rect::new(content_origin, content_size);
    let children = regions
        .into_iter()
        .map(|region| UiNode::new(UiNodeKind::Link(region.id()), region.bounds()))
        .collect();
    Ok(UiNode::with_children(
        UiNodeKind::Content,
        content_bounds,
        children,
    ))
}

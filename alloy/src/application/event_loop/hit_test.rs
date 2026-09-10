//! Geometric hit-testing of link targets against pointer positions.

use graphics::Au;
use window::PhysicalPosition;

use crate::application::pipeline::LinkTarget;

#[must_use]
#[allow(clippy::as_conversions, clippy::cast_possible_truncation)]
pub fn hit_test(links: &[LinkTarget], position: PhysicalPosition) -> Option<&str> {
    if !position.x().is_finite() || !position.y().is_finite() {
        return None;
    }
    let x_px = position.x().round() as i64;
    let y_px = position.y().round() as i64;
    let x_i32 = i32::try_from(x_px).ok()?;
    let y_i32 = i32::try_from(y_px).ok()?;
    let x_au = Au::from_whole_px(x_i32)?;
    let y_au = Au::from_whole_px(y_i32)?;
    for target in links.iter().rev() {
        let area = target.area;
        if x_au >= area.min_x()
            && x_au < area.max_x()
            && y_au >= area.min_y()
            && y_au < area.max_y()
        {
            return Some(&target.href);
        }
    }
    None
}

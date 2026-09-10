//! Geometric hit-testing of link targets against pointer positions.

use graphics::{Au, Rect};
use window::PhysicalPosition;

use crate::application::pipeline::LinkTarget;

/// The topmost link whose area contains `position`, snapped to the nearest
/// whole pixel.
///
/// The comparison stays in `f64` pixels: `std` has no checked float-to-integer
/// conversion, and widening an `Au` edge with [`f64::from`] needs none.
#[must_use]
pub fn hit_test(links: &[LinkTarget], position: PhysicalPosition) -> Option<&str> {
    if !position.x().is_finite() || !position.y().is_finite() {
        return None;
    }
    let x = position.x().round();
    let y = position.y().round();
    links
        .iter()
        .rev()
        .find(|target| contains(target.area, x, y))
        .map(|target| target.href.as_str())
}

fn contains(area: Rect, x: f64, y: f64) -> bool {
    (edge_in_pixels(area.min_x())..edge_in_pixels(area.max_x())).contains(&x)
        && (edge_in_pixels(area.min_y())..edge_in_pixels(area.max_y())).contains(&y)
}

fn edge_in_pixels(edge: Au) -> f64 {
    f64::from(edge.to_px().get())
}

#[cfg(test)]
mod tests {
    use graphics::{Au, Point, Rect, Size};
    use window::PhysicalPosition;

    use super::hit_test;
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

    fn at(x: f64, y: f64) -> PhysicalPosition {
        PhysicalPosition::new(x, y)
    }

    #[test]
    fn a_link_area_includes_its_top_left_pixel_and_excludes_its_far_edge() {
        let links = [link("/a", 10, 10, 20, 20)];

        assert_eq!(hit_test(&links, at(10.0, 10.0)), Some("/a"));
        assert_eq!(hit_test(&links, at(29.0, 29.0)), Some("/a"));
        assert_eq!(hit_test(&links, at(30.0, 20.0)), None);
        assert_eq!(hit_test(&links, at(20.0, 30.0)), None);
    }

    #[test]
    fn the_later_painted_link_wins_where_two_overlap() {
        let links = [link("/below", 0, 0, 50, 50), link("/above", 10, 10, 10, 10)];

        assert_eq!(hit_test(&links, at(15.0, 15.0)), Some("/above"));
        assert_eq!(hit_test(&links, at(40.0, 40.0)), Some("/below"));
    }

    #[test]
    fn a_non_finite_pointer_position_hits_nothing() {
        let links = [link("/a", 0, 0, 50, 50)];

        assert_eq!(hit_test(&links, at(f64::NAN, 5.0)), None);
        assert_eq!(hit_test(&links, at(5.0, f64::INFINITY)), None);
    }
}

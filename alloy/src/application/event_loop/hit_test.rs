//! Geometric hit-testing of link targets against pointer positions.

use graphics::{AU_PER_PX, Au, Rect};
use window::PhysicalPosition;

use crate::application::pipeline::LinkTarget;

/// The topmost link whose area contains `position`, snapped to the nearest
/// whole pixel.
///
/// Compares in raw `Au` units carried in `f64` (ADR-0016): the snapped pointer
/// times [`AU_PER_PX`], and every edge widened from its `i32` raw value — both
/// exact in `f64`, so this is the integer `Au` comparison, without a lossy
/// `f32` (`Au::to_px`) or a float-to-integer cast on the way.
#[must_use]
pub fn hit_test(links: &[LinkTarget], position: PhysicalPosition) -> Option<&str> {
    let x = snapped_raw_au(position.x())?;
    let y = snapped_raw_au(position.y())?;
    links
        .iter()
        .rev()
        .find(|target| contains(target.area, x, y))
        .map(|target| target.href.as_str())
}

/// A pointer coordinate in whole pixels, as raw `Au` — `None` when it is not
/// finite and so lies inside no area.
fn snapped_raw_au(pixels: f64) -> Option<f64> {
    pixels
        .is_finite()
        .then(|| pixels.round() * f64::from(AU_PER_PX))
}

fn contains(area: Rect, x: f64, y: f64) -> bool {
    (raw_edge(area.min_x())..raw_edge(area.max_x())).contains(&x)
        && (raw_edge(area.min_y())..raw_edge(area.max_y())).contains(&y)
}

fn raw_edge(edge: Au) -> f64 {
    f64::from(edge.raw())
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

    /// A link whose geometry is given in raw `Au`, for edges that do not
    /// fall on a whole pixel.
    fn raw_link(href: &str, left: i32, top: i32, width: i32, height: i32) -> LinkTarget {
        let origin = Point::new(Au::from_raw(left), Au::from_raw(top));
        let extent = Size::new(Au::from_raw(width), Au::from_raw(height));
        LinkTarget {
            area: Rect::new(origin, extent.expect("non-negative size")),
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

    #[test]
    fn a_fractional_pointer_snaps_to_the_nearest_whole_pixel() {
        let links = [link("/a", 10, 10, 20, 20)];

        assert_eq!(
            hit_test(&links, at(9.6, 9.6)),
            Some("/a"),
            "9.6 snaps to 10"
        );
        assert_eq!(hit_test(&links, at(9.4, 15.0)), None, "9.4 snaps to 9");
        assert_eq!(
            hit_test(&links, at(29.4, 29.4)),
            Some("/a"),
            "29.4 snaps to 29"
        );
        assert_eq!(
            hit_test(&links, at(29.6, 15.0)),
            None,
            "29.6 snaps to 30, the far edge"
        );
    }

    #[test]
    fn an_edge_between_whole_pixels_is_compared_exactly() {
        // min at 10.25px (raw 656), max at 20.75px (raw 1328).
        let links = [raw_link("/a", 656, 656, 672, 672)];

        assert_eq!(hit_test(&links, at(10.0, 15.0)), None, "raw 640 < 656");
        assert_eq!(hit_test(&links, at(11.0, 15.0)), Some("/a"));
        assert_eq!(
            hit_test(&links, at(20.0, 15.0)),
            Some("/a"),
            "raw 1280 < 1328"
        );
        assert_eq!(hit_test(&links, at(21.0, 15.0)), None, "raw 1344 >= 1328");
    }

    #[test]
    fn edges_past_f32_precision_stay_exact() {
        // 2^24 raw Au is where f32 stops holding every integer; an edge one
        // past a whole pixel there must still exclude that pixel.
        let links = [raw_link("/far", 16_777_217, 0, 640, 64)];

        assert_eq!(
            hit_test(&links, at(262_144.0, 0.0)),
            None,
            "raw 2^24 < edge"
        );
        assert_eq!(hit_test(&links, at(262_145.0, 0.0)), Some("/far"));
    }
}

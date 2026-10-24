//! The geometric primitives of hit-testing, shared by the UI render tree.
//!
//! Compares in raw `Au` units carried in `f64` (ADR-0016): the snapped pointer
//! times [`AU_PER_PX`], and every edge widened from its `i32` raw value — both
//! exact in `f64`, so this is the integer `Au` comparison, without a lossy
//! `f32` (`Au::to_px`) or a float-to-integer cast on the way.

use graphics::{AU_PER_PX, Au, Rect};

/// A pointer coordinate in whole pixels, as raw `Au` — `None` when it is not
/// finite and so lies inside no area.
#[must_use]
pub(super) fn snapped_raw_au(pixels: f64) -> Option<f64> {
    pixels
        .is_finite()
        .then(|| pixels.round() * f64::from(AU_PER_PX))
}

#[must_use]
pub(super) fn contains(area: Rect, x: f64, y: f64) -> bool {
    (raw_edge(area.min_x())..raw_edge(area.max_x())).contains(&x)
        && (raw_edge(area.min_y())..raw_edge(area.max_y())).contains(&y)
}

#[must_use]
pub(super) fn raw_edge(edge: Au) -> f64 {
    f64::from(edge.raw())
}

#[cfg(test)]
mod tests {
    use graphics::{Au, Point, Rect, Size};

    use super::{contains, snapped_raw_au};

    fn area(left: i32, top: i32, width: i32, height: i32) -> Rect {
        let au = |pixels| Au::from_whole_px(pixels).expect("fits the Au envelope");
        Rect::new(
            Point::new(au(left), au(top)),
            Size::new(au(width), au(height)).expect("non-negative size"),
        )
    }

    /// An area whose geometry is given in raw `Au`, for edges that do not
    /// fall on a whole pixel.
    fn raw_area(left: i32, top: i32, width: i32, height: i32) -> Rect {
        let origin = Point::new(Au::from_raw(left), Au::from_raw(top));
        let extent = Size::new(Au::from_raw(width), Au::from_raw(height));
        Rect::new(origin, extent.expect("non-negative size"))
    }

    /// Whether a pointer at (`x`, `y`) pixels lands in `area`.
    fn hits(area: Rect, x: f64, y: f64) -> bool {
        match (snapped_raw_au(x), snapped_raw_au(y)) {
            (Some(x), Some(y)) => contains(area, x, y),
            _ => false,
        }
    }

    #[test]
    fn an_area_includes_its_top_left_pixel_and_excludes_its_far_edge() {
        let link = area(10, 10, 20, 20);

        assert!(hits(link, 10.0, 10.0));
        assert!(hits(link, 29.0, 29.0));
        assert!(!hits(link, 30.0, 20.0));
        assert!(!hits(link, 20.0, 30.0));
    }

    #[test]
    fn a_non_finite_pointer_position_hits_nothing() {
        let link = area(0, 0, 50, 50);

        assert!(!hits(link, f64::NAN, 5.0));
        assert!(!hits(link, 5.0, f64::INFINITY));
    }

    #[test]
    fn a_fractional_pointer_snaps_to_the_nearest_whole_pixel() {
        let link = area(10, 10, 20, 20);

        assert!(hits(link, 9.6, 9.6), "9.6 snaps to 10");
        assert!(!hits(link, 9.4, 15.0), "9.4 snaps to 9");
        assert!(hits(link, 29.4, 29.4), "29.4 snaps to 29");
        assert!(!hits(link, 29.6, 15.0), "29.6 snaps to 30, the far edge");
    }

    #[test]
    fn an_edge_between_whole_pixels_is_compared_exactly() {
        // min at 10.25px (raw 656), max at 20.75px (raw 1328).
        let link = raw_area(656, 656, 672, 672);

        assert!(!hits(link, 10.0, 15.0), "raw 640 < 656");
        assert!(hits(link, 11.0, 15.0));
        assert!(hits(link, 20.0, 15.0), "raw 1280 < 1328");
        assert!(!hits(link, 21.0, 15.0), "raw 1344 >= 1328");
    }

    #[test]
    fn edges_past_f32_precision_stay_exact() {
        // 2^24 raw Au is where f32 stops holding every integer; an edge one
        // past a whole pixel there must still exclude that pixel.
        let link = raw_area(16_777_217, 0, 640, 64);

        assert!(!hits(link, 262_144.0, 0.0), "raw 2^24 < edge");
        assert!(hits(link, 262_145.0, 0.0));
    }
}

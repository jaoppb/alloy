//! Integer-exact rectangle coverage and `src-over` composition.
//!
//! ## Why the arithmetic is exact
//!
//! A pixel spans exactly `AU_PER_PX` units on each axis, and every rectangle
//! edge is already an [`Au`]. So the area of the overlap between a pixel and a
//! rectangle is `overlap_x * overlap_y`, a product of two integers in `0..=64` —
//! an exact coverage in `0..=4096`, computed identically on every platform. No
//! floating point appears anywhere in this file, which is the whole reason a
//! golden image can match byte for byte on three operating systems
//! (`ADR-0016`, v0.3 report §2.5).
//!
//! ## Why the canvas stays opaque
//!
//! [`crate::RenderBackend::begin_frame`] fills the surface with an opaque
//! colour, and every command composites over it. Destination alpha is therefore
//! always `255`, which collapses `src-over` to a plain lerp and removes the
//! un-premultiply division that would otherwise round differently at low alpha.
//! A page rendered to a PNG is exactly this case; a transparent canvas would
//! need the general form, and is not something v0.3 offers.

use crate::domain::color::Color;
use crate::domain::geometry::Rect;
use crate::domain::unit::{AU_PER_PX, Au};

/// The coverage of a pixel entirely inside a shape: `64 × 64`.
pub(super) const FULL_COVERAGE: u32 = 4_096;

/// How much of pixel `index` on one axis lies inside `[start, end)`, in `Au`.
///
/// Ranges `0..=64`, and is `0` for any index that cannot be placed — which is
/// how an out-of-range column silently contributes nothing instead of panicking.
pub(super) fn axis_overlap(index: u32, start: Au, end: Au) -> u32 {
    let Ok(index) = i32::try_from(index) else {
        return 0;
    };
    let Some(pixel_start) = index.checked_mul(AU_PER_PX) else {
        return 0;
    };
    let Some(pixel_end) = pixel_start.checked_add(AU_PER_PX) else {
        return 0;
    };
    let lower = pixel_start.max(start.raw());
    let upper = pixel_end.min(end.raw());
    let overlap = upper.saturating_sub(lower).clamp(0, AU_PER_PX);
    u32::try_from(overlap).unwrap_or(0)
}

/// The half-open range of pixel indices a span touches, clipped to `limit`.
pub(super) fn pixel_range(start: Au, end: Au, limit: u32) -> (u32, u32) {
    let first = to_index(start.raw().div_euclid(AU_PER_PX), limit);
    let last = to_index(end.raw().div_euclid(AU_PER_PX).saturating_add(1), limit);
    (first, last.max(first))
}

/// Clamps a signed pixel index into `0..=limit`.
fn to_index(value: i32, limit: u32) -> u32 {
    u32::try_from(value).unwrap_or(0).min(limit)
}

/// The exact coverage of pixel `(column, row)` by `rect`, in `0..=4096`.
pub(super) fn rect_coverage(rect: Rect, column: u32, row: u32) -> u32 {
    let horizontal = axis_overlap(column, rect.min_x(), rect.max_x());
    let vertical = axis_overlap(row, rect.min_y(), rect.max_y());
    horizontal.saturating_mul(vertical)
}

/// Composites `source` over `destination` at `coverage`, on an opaque canvas.
///
/// `coverage` is in `0..=4096` and `source`'s own alpha attenuates it further,
/// so a half-covered pixel of a half-transparent colour lands at a quarter — the
/// same answer on every platform, because every step is integer.
pub(super) fn blend_over(destination: Color, source: Color, coverage: u32) -> Color {
    let coverage = coverage.min(FULL_COVERAGE);
    let alpha = scale(u32::from(source.alpha()), coverage, FULL_COVERAGE);
    if alpha == 0 {
        return destination;
    }
    Color::rgba(
        mix(destination.red(), source.red(), alpha),
        mix(destination.green(), source.green(), alpha),
        mix(destination.blue(), source.blue(), alpha),
        u8::MAX,
    )
}

/// `source * alpha + destination * (255 - alpha)`, rounded to nearest.
fn mix(destination: u8, source: u8, alpha: u32) -> u8 {
    let inverse = u32::from(u8::MAX).saturating_sub(alpha);
    let weighted = u32::from(source)
        .saturating_mul(alpha)
        .saturating_add(u32::from(destination).saturating_mul(inverse))
        .saturating_add(127);
    let rounded = weighted.checked_div(u32::from(u8::MAX)).unwrap_or(0);
    u8::try_from(rounded).unwrap_or(u8::MAX)
}

/// `value * numerator / denominator`, rounded to nearest, saturating at `255`.
fn scale(value: u32, numerator: u32, denominator: u32) -> u32 {
    if denominator == 0 {
        return 0;
    }
    let half = denominator.checked_div(2).unwrap_or(0);
    value
        .saturating_mul(numerator)
        .saturating_add(half)
        .checked_div(denominator)
        .unwrap_or(0)
        .min(u32::from(u8::MAX))
}

/// How many samples per axis a corner pixel is probed with.
///
/// A fixed grid, so the answer is a pure function of the geometry — the same
/// reason [`crate::Au`] exists. Straight edges keep the exact analytic coverage
/// above; only the pixels inside a corner box pay for supersampling, and only
/// when a radius was actually asked for.
const CORNER_SAMPLES: i32 = 4;

/// The `Au` width of one sub-cell of the corner sample grid: `64 / 4`.
const CORNER_CELL: i64 = 16;

/// The fraction of pixel `(column, row)` that survives rounding `rect`'s corners
/// at `radius`, in `0..=4096`.
///
/// Returns [`FULL_COVERAGE`] for every pixel outside a corner box, which is the
/// overwhelming majority — a rounded rectangle is mostly straight.
pub(super) fn corner_coverage(rect: Rect, radius: Au, column: u32, row: u32) -> u32 {
    if radius.is_zero() || radius.is_negative() {
        return FULL_COVERAGE;
    }
    let Some(centre) = nearest_corner_centre(rect, radius, column, row) else {
        return FULL_COVERAGE;
    };
    sample_disc(centre, radius, column, row)
}

/// The centre of the corner arc governing `(column, row)`, or `None` when the
/// pixel lies on a straight run.
fn nearest_corner_centre(rect: Rect, radius: Au, column: u32, row: u32) -> Option<(Au, Au)> {
    let left = rect.min_x().saturating_add(radius);
    let right = rect.max_x().saturating_sub(radius);
    let top = rect.min_y().saturating_add(radius);
    let bottom = rect.max_y().saturating_sub(radius);
    let horizontal = corner_axis(column, rect.min_x(), left, right)?;
    let vertical = corner_axis(row, rect.min_y(), top, bottom)?;
    Some((horizontal, vertical))
}

/// The arc centre on one axis, or `None` when the pixel sits between the two
/// corners on that axis and is therefore on a straight edge.
fn corner_axis(index: u32, _start: Au, near: Au, far: Au) -> Option<Au> {
    let Ok(index) = i32::try_from(index) else {
        return None;
    };
    let pixel_start = index.checked_mul(AU_PER_PX)?;
    let pixel_end = pixel_start.saturating_add(AU_PER_PX);
    if pixel_end <= near.raw() {
        return Some(near);
    }
    if pixel_start >= far.raw() {
        return Some(far);
    }
    None
}

/// Counts how many of the `CORNER_SAMPLES²` sample points fall inside the disc.
///
/// The inside test is `dx² + dy² <= r²` on `i64`, so it is exact and cannot
/// overflow for any `Au` the envelope admits.
fn sample_disc(centre: (Au, Au), radius: Au, column: u32, row: u32) -> u32 {
    let (centre_x, centre_y) = centre;
    let radius_squared = i64::from(radius.raw()).saturating_mul(i64::from(radius.raw()));
    let mut inside = 0_u32;
    for sample_y in 0..CORNER_SAMPLES {
        for sample_x in 0..CORNER_SAMPLES {
            let point_x = sample_position(column, sample_x);
            let point_y = sample_position(row, sample_y);
            let delta_x = point_x.saturating_sub(i64::from(centre_x.raw()));
            let delta_y = point_y.saturating_sub(i64::from(centre_y.raw()));
            let distance_squared = delta_x
                .saturating_mul(delta_x)
                .saturating_add(delta_y.saturating_mul(delta_y));
            if distance_squared <= radius_squared {
                inside = inside.saturating_add(1);
            }
        }
    }
    let total = u32::try_from(CORNER_SAMPLES.saturating_mul(CORNER_SAMPLES)).unwrap_or(1);
    scale_coverage(inside, total)
}

/// The `Au` position of sample `slot` within pixel `index`, taken at the centre
/// of each sub-cell so no sample lands exactly on a boundary.
fn sample_position(index: u32, slot: i32) -> i64 {
    let pixel_start = i64::from(index).saturating_mul(i64::from(AU_PER_PX));
    let offset = CORNER_CELL
        .saturating_mul(i64::from(slot))
        .saturating_add(CORNER_CELL.checked_div(2).unwrap_or(0));
    pixel_start.saturating_add(offset)
}

/// Rescales a `inside / total` fraction onto `0..=4096`.
fn scale_coverage(inside: u32, total: u32) -> u32 {
    if total == 0 {
        return FULL_COVERAGE;
    }
    inside
        .saturating_mul(FULL_COVERAGE)
        .checked_div(total)
        .unwrap_or(FULL_COVERAGE)
}

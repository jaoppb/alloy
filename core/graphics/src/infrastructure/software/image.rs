//! Integer box sampling and composition for `DrawImage` (`PRD-005`, `ADR-0016`).

use crate::domain::color::{Color, Opacity};
use crate::domain::framebuffer::Framebuffer;
use crate::domain::geometry::Rect;
use crate::domain::unit::AU_PER_PX;
use crate::infrastructure::software::raster;

/// Blits and scales `image` from `source` into `destination`, clipped by `area`
/// and attenuated by `opacity`.
///
/// Uses integer-exact arithmetic: coordinates and dimensions are mapped via
/// integer ratios without floating-point math, ensuring determinism across all targets.
pub(super) fn paint_image(
    frame: &mut Framebuffer,
    area: Rect,
    destination: Rect,
    source: Rect,
    image: &Framebuffer,
    opacity: Opacity,
) {
    if destination.is_empty() || source.is_empty() || image.width() == 0 || image.height() == 0 {
        return;
    }

    let (first_col, last_col) = raster::pixel_range(area.min_x(), area.max_x(), frame.width());
    let (first_row, last_row) = raster::pixel_range(area.min_y(), area.max_y(), frame.height());

    let dst_origin_x = i64::from(destination.min_x().raw());
    let dst_origin_y = i64::from(destination.min_y().raw());
    let dst_w = i64::from(destination.size().width().raw());
    let dst_h = i64::from(destination.size().height().raw());

    let src_origin_x = i64::from(source.min_x().raw());
    let src_origin_y = i64::from(source.min_y().raw());
    let src_w = i64::from(source.size().width().raw());
    let src_h = i64::from(source.size().height().raw());

    let img_w = i64::from(image.width());
    let img_h = i64::from(image.height());
    let au_per_px = i64::from(AU_PER_PX);

    for row in first_row..last_row {
        for column in first_col..last_col {
            let clipped_coverage = raster::rect_coverage(area, column, row);
            if clipped_coverage == 0 {
                continue;
            }

            let col_i64 = i64::from(column);
            let row_i64 = i64::from(row);

            let cell_left = col_i64
                .saturating_mul(au_per_px)
                .max(i64::from(destination.min_x().raw()));
            let cell_right = col_i64
                .saturating_add(1)
                .saturating_mul(au_per_px)
                .min(i64::from(destination.max_x().raw()));
            let cell_top = row_i64
                .saturating_mul(au_per_px)
                .max(i64::from(destination.min_y().raw()));
            let cell_bottom = row_i64
                .saturating_add(1)
                .saturating_mul(au_per_px)
                .min(i64::from(destination.max_y().raw()));

            if cell_left >= cell_right || cell_top >= cell_bottom {
                continue;
            }

            let delta_left = cell_left.saturating_sub(dst_origin_x);
            let sample_left = src_origin_x.saturating_add(
                delta_left
                    .saturating_mul(src_w)
                    .checked_div(dst_w)
                    .unwrap_or(0),
            );

            let delta_right = cell_right.saturating_sub(dst_origin_x);
            let sample_right = src_origin_x.saturating_add(
                delta_right
                    .saturating_mul(src_w)
                    .checked_div(dst_w)
                    .unwrap_or(0),
            );

            let delta_top = cell_top.saturating_sub(dst_origin_y);
            let sample_top = src_origin_y.saturating_add(
                delta_top
                    .saturating_mul(src_h)
                    .checked_div(dst_h)
                    .unwrap_or(0),
            );

            let delta_bottom = cell_bottom.saturating_sub(dst_origin_y);
            let sample_bottom = src_origin_y.saturating_add(
                delta_bottom
                    .saturating_mul(src_h)
                    .checked_div(dst_h)
                    .unwrap_or(0),
            );

            let col_start = sample_left
                .div_euclid(au_per_px)
                .clamp(0, img_w.saturating_sub(1));
            let col_end = sample_right
                .saturating_add(au_per_px.saturating_sub(1))
                .div_euclid(au_per_px)
                .clamp(col_start.saturating_add(1), img_w);
            let row_start = sample_top
                .div_euclid(au_per_px)
                .clamp(0, img_h.saturating_sub(1));
            let row_end = sample_bottom
                .saturating_add(au_per_px.saturating_sub(1))
                .div_euclid(au_per_px)
                .clamp(row_start.saturating_add(1), img_h);

            let sample_color = sample_box(image, col_start, col_end, row_start, row_end);
            let Some(color) = sample_color else {
                continue;
            };

            let source_color = color.faded(opacity);
            let Some(dest_color) = frame.pixel(column, row) else {
                continue;
            };
            let blended = raster::blend_over(dest_color, source_color, clipped_coverage);
            frame.set_pixel(column, row, blended);
        }
    }
}

fn sample_box(
    image: &Framebuffer,
    col_start: i64,
    col_end: i64,
    row_start: i64,
    row_end: i64,
) -> Option<Color> {
    if col_end == col_start.saturating_add(1) && row_end == row_start.saturating_add(1) {
        let c = u32::try_from(col_start).ok()?;
        let r = u32::try_from(row_start).ok()?;
        return image.pixel(c, r);
    }

    let mut sum_r = 0_u32;
    let mut sum_g = 0_u32;
    let mut sum_b = 0_u32;
    let mut sum_a = 0_u32;
    let mut count = 0_u32;

    for r_i64 in row_start..row_end {
        let Ok(r) = u32::try_from(r_i64) else {
            continue;
        };
        for c_i64 in col_start..col_end {
            let Ok(c) = u32::try_from(c_i64) else {
                continue;
            };
            if let Some(pixel) = image.pixel(c, r) {
                sum_r = sum_r.saturating_add(u32::from(pixel.red()));
                sum_g = sum_g.saturating_add(u32::from(pixel.green()));
                sum_b = sum_b.saturating_add(u32::from(pixel.blue()));
                sum_a = sum_a.saturating_add(u32::from(pixel.alpha()));
                count = count.saturating_add(1);
            }
        }
    }

    if count == 0 {
        return None;
    }

    let avg_r = u8::try_from(sum_r.checked_div(count).unwrap_or(0)).unwrap_or(0);
    let avg_g = u8::try_from(sum_g.checked_div(count).unwrap_or(0)).unwrap_or(0);
    let avg_b = u8::try_from(sum_b.checked_div(count).unwrap_or(0)).unwrap_or(0);
    let avg_a = u8::try_from(sum_a.checked_div(count).unwrap_or(0)).unwrap_or(0);
    Some(Color::rgba(avg_r, avg_g, avg_b, avg_a))
}

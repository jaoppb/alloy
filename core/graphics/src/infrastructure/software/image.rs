//! Integer box sampling and composition for `DrawImage` (`PRD-005`, `ADR-0016`).

use crate::domain::color::{Color, Opacity};
use crate::domain::framebuffer::Framebuffer;
use crate::domain::geometry::Rect;
use crate::domain::unit::AU_PER_PX;
use crate::infrastructure::software::raster;

/// Maps source geometry to destination geometry using exact integer ratios.
#[derive(Clone, Copy)]
struct MappingContext {
    destination_origin_x: i64,
    destination_origin_y: i64,
    destination_width: i64,
    destination_height: i64,
    source_origin_x: i64,
    source_origin_y: i64,
    source_width: i64,
    source_height: i64,
    image_width: i64,
    image_height: i64,
}

/// Blits and scales `image` from `source` into `destination`, clipped by `area`
/// and attenuated by `opacity`.
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

    let (first_column, last_column) =
        raster::pixel_range(area.min_x(), area.max_x(), frame.width());
    let (first_row, last_row) = raster::pixel_range(area.min_y(), area.max_y(), frame.height());
    let mapping = create_mapping_context(destination, source, image);
    let mut paint_context = PaintContext {
        frame,
        area,
        destination,
        mapping,
        image,
        opacity,
    };

    for row_index in first_row..last_row {
        paint_image_row(&mut paint_context, first_column, last_column, row_index);
    }
}

struct PaintContext<'a> {
    frame: &'a mut Framebuffer,
    area: Rect,
    destination: Rect,
    mapping: MappingContext,
    image: &'a Framebuffer,
    opacity: Opacity,
}

fn create_mapping_context(destination: Rect, source: Rect, image: &Framebuffer) -> MappingContext {
    MappingContext {
        destination_origin_x: i64::from(destination.min_x().raw()),
        destination_origin_y: i64::from(destination.min_y().raw()),
        destination_width: i64::from(destination.size().width().raw()),
        destination_height: i64::from(destination.size().height().raw()),
        source_origin_x: i64::from(source.min_x().raw()),
        source_origin_y: i64::from(source.min_y().raw()),
        source_width: i64::from(source.size().width().raw()),
        source_height: i64::from(source.size().height().raw()),
        image_width: i64::from(image.width()),
        image_height: i64::from(image.height()),
    }
}

fn paint_image_row(
    context: &mut PaintContext<'_>,
    first_column: u32,
    last_column: u32,
    row_index: u32,
) {
    for column_index in first_column..last_column {
        paint_image_pixel(context, column_index, row_index);
    }
}

fn paint_image_pixel(context: &mut PaintContext<'_>, column_index: u32, row_index: u32) {
    let clipped_coverage = raster::rect_coverage(context.area, column_index, row_index);
    if clipped_coverage == 0 {
        return;
    }

    let Some(sample_window) = calculate_sample_window(
        context.destination,
        context.mapping,
        column_index,
        row_index,
    ) else {
        return;
    };
    let Some(sample_color) = sample_box(context.image, sample_window) else {
        return;
    };
    let Some(destination_color) = context.frame.pixel(column_index, row_index) else {
        return;
    };

    let source_color = sample_color.faded(context.opacity);
    let blended_color = raster::blend_over(destination_color, source_color, clipped_coverage);
    context
        .frame
        .set_pixel(column_index, row_index, blended_color);
}

#[derive(Clone, Copy)]
struct SampleWindow {
    sample_left: i64,
    sample_right: i64,
    sample_top: i64,
    sample_bottom: i64,
    column_start: i64,
    column_end: i64,
    row_start: i64,
    row_end: i64,
}

fn calculate_sample_window(
    destination: Rect,
    context: MappingContext,
    column_index: u32,
    row_index: u32,
) -> Option<SampleWindow> {
    let au_per_pixel = i64::from(AU_PER_PX);
    let column_i64 = i64::from(column_index);
    let row_i64 = i64::from(row_index);

    let cell_left = column_i64
        .saturating_mul(au_per_pixel)
        .max(i64::from(destination.min_x().raw()));
    let cell_right = column_i64
        .saturating_add(1)
        .saturating_mul(au_per_pixel)
        .min(i64::from(destination.max_x().raw()));
    let cell_top = row_i64
        .saturating_mul(au_per_pixel)
        .max(i64::from(destination.min_y().raw()));
    let cell_bottom = row_i64
        .saturating_add(1)
        .saturating_mul(au_per_pixel)
        .min(i64::from(destination.max_y().raw()));

    if cell_left >= cell_right || cell_top >= cell_bottom {
        return None;
    }

    let delta_left = cell_left.saturating_sub(context.destination_origin_x);
    let delta_right = cell_right.saturating_sub(context.destination_origin_x);
    let delta_top = cell_top.saturating_sub(context.destination_origin_y);
    let delta_bottom = cell_bottom.saturating_sub(context.destination_origin_y);

    let sample_left = context.source_origin_x.saturating_add(
        delta_left
            .saturating_mul(context.source_width)
            .checked_div(context.destination_width)?,
    );
    let sample_right = context.source_origin_x.saturating_add(
        delta_right
            .saturating_mul(context.source_width)
            .checked_div(context.destination_width)?,
    );
    let sample_top = context.source_origin_y.saturating_add(
        delta_top
            .saturating_mul(context.source_height)
            .checked_div(context.destination_height)?,
    );
    let sample_bottom = context.source_origin_y.saturating_add(
        delta_bottom
            .saturating_mul(context.source_height)
            .checked_div(context.destination_height)?,
    );

    let maximum_source_width = context.image_width.saturating_mul(au_per_pixel);
    let maximum_source_height = context.image_height.saturating_mul(au_per_pixel);
    if sample_right <= 0
        || sample_left >= maximum_source_width
        || sample_bottom <= 0
        || sample_top >= maximum_source_height
    {
        return None;
    }

    let column_start = sample_left.div_euclid(au_per_pixel);
    let column_end = sample_right
        .saturating_add(au_per_pixel.saturating_sub(1))
        .div_euclid(au_per_pixel)
        .max(column_start.saturating_add(1));
    let row_start = sample_top.div_euclid(au_per_pixel);
    let row_end = sample_bottom
        .saturating_add(au_per_pixel.saturating_sub(1))
        .div_euclid(au_per_pixel)
        .max(row_start.saturating_add(1));

    Some(SampleWindow {
        sample_left,
        sample_right,
        sample_top,
        sample_bottom,
        column_start,
        column_end,
        row_start,
        row_end,
    })
}

fn sample_box(image: &Framebuffer, window: SampleWindow) -> Option<Color> {
    if window.column_end == window.column_start.saturating_add(1)
        && window.row_end == window.row_start.saturating_add(1)
    {
        let column = u32::try_from(window.column_start).ok()?;
        let row = u32::try_from(window.row_start).ok()?;
        return image.pixel(column, row);
    }

    accumulate_samples(image, window)
}

fn accumulate_samples(image: &Framebuffer, window: SampleWindow) -> Option<Color> {
    let au_per_pixel = i64::from(AU_PER_PX);
    let mut accumulated_red = 0_u64;
    let mut accumulated_green = 0_u64;
    let mut accumulated_blue = 0_u64;
    let mut accumulated_alpha = 0_u64;
    let mut total_weight = 0_u64;

    for row_index in window.row_start..window.row_end {
        let Ok(row_u32) = u32::try_from(row_index) else {
            continue;
        };
        let pixel_top = row_index.saturating_mul(au_per_pixel);
        let pixel_bottom = pixel_top.saturating_add(au_per_pixel);
        let overlap_vertical = pixel_bottom
            .min(window.sample_bottom)
            .saturating_sub(pixel_top.max(window.sample_top))
            .max(0);
        if overlap_vertical == 0 {
            continue;
        }

        for column_index in window.column_start..window.column_end {
            let Ok(column_u32) = u32::try_from(column_index) else {
                continue;
            };
            let pixel_left = column_index.saturating_mul(au_per_pixel);
            let pixel_right = pixel_left.saturating_add(au_per_pixel);
            let overlap_horizontal = pixel_right
                .min(window.sample_right)
                .saturating_sub(pixel_left.max(window.sample_left))
                .max(0);
            if overlap_horizontal == 0 {
                continue;
            }

            let weight =
                u64::try_from(overlap_horizontal.saturating_mul(overlap_vertical)).unwrap_or(0);
            if let Some(pixel) = image.pixel(column_u32, row_u32) {
                accumulated_red =
                    accumulated_red.saturating_add(u64::from(pixel.red()).saturating_mul(weight));
                accumulated_green = accumulated_green
                    .saturating_add(u64::from(pixel.green()).saturating_mul(weight));
                accumulated_blue =
                    accumulated_blue.saturating_add(u64::from(pixel.blue()).saturating_mul(weight));
                accumulated_alpha = accumulated_alpha
                    .saturating_add(u64::from(pixel.alpha()).saturating_mul(weight));
                total_weight = total_weight.saturating_add(weight);
            }
        }
    }

    if total_weight == 0 {
        return None;
    }

    let average_red = u8::try_from(accumulated_red.checked_div(total_weight)?).unwrap_or(0);
    let average_green = u8::try_from(accumulated_green.checked_div(total_weight)?).unwrap_or(0);
    let average_blue = u8::try_from(accumulated_blue.checked_div(total_weight)?).unwrap_or(0);
    let average_alpha = u8::try_from(accumulated_alpha.checked_div(total_weight)?).unwrap_or(0);

    Some(Color::rgba(
        average_red,
        average_green,
        average_blue,
        average_alpha,
    ))
}

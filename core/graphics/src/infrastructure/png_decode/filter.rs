//! Scanline unfiltering algorithms and reconstruction (RFC 2083 §6).

use crate::domain::framebuffer::Framebuffer;
use crate::domain::geometry::SurfaceSize;
use crate::infrastructure::png_decode::error::PngDecodeError;
use crate::infrastructure::png_decode::header::{COLOR_TYPE_RGB, ImageHeader};

/// Reconstructs scanlines from uncompressed deflation stream into a [`Framebuffer`].
pub(super) fn unfilter_scanlines(
    header: &ImageHeader,
    uncompressed: &[u8],
) -> Result<Framebuffer, PngDecodeError> {
    let width_usize = usize::try_from(header.width).map_err(|_| invalid_dim(header))?;
    let height_usize = usize::try_from(header.height).map_err(|_| invalid_dim(header))?;
    let bytes_per_pixel = pixel_stride(header.color_type);

    let row_bytes = width_usize
        .checked_mul(bytes_per_pixel)
        .ok_or_else(|| invalid_dim(header))?;
    let scanline_bytes = row_bytes
        .checked_add(1)
        .ok_or_else(|| invalid_dim(header))?;

    let total_pixel_bytes = width_usize
        .checked_mul(height_usize)
        .and_then(|count| count.checked_mul(4))
        .ok_or_else(|| invalid_dim(header))?;

    let mut pixels = Vec::with_capacity(total_pixel_bytes);
    let mut prior_row = vec![0_u8; row_bytes];
    let mut current_row = vec![0_u8; row_bytes];
    let mut cursor = uncompressed;

    for row_index in 0..header.height {
        let filter_type = *cursor.first().ok_or(PngDecodeError::Truncated)?;
        let scanline = cursor
            .get(1..scanline_bytes)
            .ok_or(PngDecodeError::Truncated)?;
        cursor = cursor
            .get(scanline_bytes..)
            .ok_or(PngDecodeError::Truncated)?;

        reconstruct_row(
            filter_type,
            row_index,
            scanline,
            &prior_row,
            &mut current_row,
            bytes_per_pixel,
        )?;
        append_pixel_data(&mut pixels, &current_row, bytes_per_pixel);
        std::mem::swap(&mut prior_row, &mut current_row);
    }

    let surface_size =
        SurfaceSize::new(header.width, header.height).ok_or_else(|| invalid_dim(header))?;
    Framebuffer::from_rgba8(surface_size, pixels).ok_or_else(|| invalid_dim(header))
}

fn reconstruct_row(
    filter_type: u8,
    row_index: u32,
    scanline: &[u8],
    prior_row: &[u8],
    current_row: &mut [u8],
    bytes_per_pixel: usize,
) -> Result<(), PngDecodeError> {
    for byte_index in 0..current_row.len() {
        let raw_byte = *scanline.get(byte_index).ok_or(PngDecodeError::Truncated)?;
        let left = sample_neighbor(current_row, byte_index, bytes_per_pixel);
        let above = *prior_row.get(byte_index).unwrap_or(&0);
        let upper_left = sample_neighbor(prior_row, byte_index, bytes_per_pixel);

        let restored_byte = match filter_type {
            0 => raw_byte,
            1 => raw_byte.wrapping_add(left),
            2 => raw_byte.wrapping_add(above),
            3 => raw_byte.wrapping_add(average(left, above)),
            4 => raw_byte.wrapping_add(paeth_predictor(left, above, upper_left)),
            _ => {
                return Err(PngDecodeError::InvalidFilter {
                    filter: filter_type,
                    row: row_index,
                });
            }
        };

        if let Some(target) = current_row.get_mut(byte_index) {
            *target = restored_byte;
        }
    }
    Ok(())
}

fn sample_neighbor(buffer: &[u8], index: usize, bytes_per_pixel: usize) -> u8 {
    let Some(offset) = index.checked_sub(bytes_per_pixel) else {
        return 0;
    };
    *buffer.get(offset).unwrap_or(&0)
}

fn average(left: u8, above: u8) -> u8 {
    let sum = u16::from(left).saturating_add(u16::from(above));
    let half = sum.checked_div(2).unwrap_or(0);
    u8::try_from(half).unwrap_or(0)
}

fn paeth_predictor(left: u8, above: u8, upper_left: u8) -> u8 {
    let left_signed = i32::from(left);
    let above_signed = i32::from(above);
    let upper_left_signed = i32::from(upper_left);
    let estimate = left_signed
        .saturating_add(above_signed)
        .saturating_sub(upper_left_signed);
    let distance_left = estimate.saturating_sub(left_signed).abs();
    let distance_above = estimate.saturating_sub(above_signed).abs();
    let distance_upper_left = estimate.saturating_sub(upper_left_signed).abs();

    match (
        distance_left <= distance_above,
        distance_left <= distance_upper_left,
        distance_above <= distance_upper_left,
    ) {
        (true, true, _) => left,
        (_, _, true) => above,
        _ => upper_left,
    }
}

fn append_pixel_data(destination: &mut Vec<u8>, row_data: &[u8], bytes_per_pixel: usize) {
    match bytes_per_pixel {
        3 => {
            for rgb in row_data.chunks_exact(3) {
                let red = *rgb.first().unwrap_or(&0);
                let green = *rgb.get(1).unwrap_or(&0);
                let blue = *rgb.get(2).unwrap_or(&0);
                destination.extend_from_slice(&[red, green, blue, 255]);
            }
        }
        _ => {
            destination.extend_from_slice(row_data);
        }
    }
}

const fn pixel_stride(color_type: u8) -> usize {
    match color_type {
        COLOR_TYPE_RGB => 3,
        _ => 4,
    }
}

const fn invalid_dim(header: &ImageHeader) -> PngDecodeError {
    PngDecodeError::InvalidDimensions {
        width: header.width,
        height: header.height,
    }
}

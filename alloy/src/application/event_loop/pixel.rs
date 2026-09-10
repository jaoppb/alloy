//! Pixel formatting and premultiplication for presented frames.

use graphics::Framebuffer;

#[must_use]
pub fn premultiply_channel(channel: u8, alpha: u8) -> u8 {
    let product = u32::from(channel).saturating_mul(u32::from(alpha));
    let scaled = product.checked_div(255).unwrap_or(0);
    u8::try_from(scaled).unwrap_or(u8::MAX)
}

#[must_use]
pub fn pack_argb(alpha: u8, red: u8, green: u8, blue: u8) -> u32 {
    let alpha = u32::from(alpha).checked_shl(24).unwrap_or(0);
    let red = u32::from(red).checked_shl(16).unwrap_or(0);
    let green = u32::from(green).checked_shl(8).unwrap_or(0);
    alpha | red | green | u32::from(blue)
}

/// Straight-alpha `RGBA8` (`core/graphics`'s wire format) to premultiplied
/// `0xAARRGGBB` (`window::FrameView`'s — see its own doc comment).
#[must_use]
pub fn frame_pixels(framebuffer: &Framebuffer) -> Vec<u32> {
    let mut pixels = Vec::new();
    for chunk in framebuffer.as_rgba8().chunks_exact(4) {
        let (red, green, blue, alpha) = match chunk {
            [red, green, blue, alpha] => (*red, *green, *blue, *alpha),
            _ => continue,
        };
        let packed = pack_argb(
            alpha,
            premultiply_channel(red, alpha),
            premultiply_channel(green, alpha),
            premultiply_channel(blue, alpha),
        );
        pixels.push(packed);
    }
    pixels
}

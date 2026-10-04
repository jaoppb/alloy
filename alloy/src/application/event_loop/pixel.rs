//! Pixel formatting and premultiplication for presented frames.

use graphics::Framebuffer;

/// Straight-alpha `RGBA8` (`core/graphics`'s wire format) to premultiplied
/// `0xAARRGGBB` (`window::FrameView`'s — see its own doc comment).
pub fn frame_pixels(framebuffer: &Framebuffer) -> Vec<u32> {
    let (pixels, _) = framebuffer.as_rgba8().as_chunks::<4>();
    pixels.iter().copied().map(premultiplied_argb).collect()
}

fn premultiplied_argb([red, green, blue, alpha]: [u8; 4]) -> u32 {
    pack_argb(
        alpha,
        premultiply_channel(red, alpha),
        premultiply_channel(green, alpha),
        premultiply_channel(blue, alpha),
    )
}

fn premultiply_channel(channel: u8, alpha: u8) -> u8 {
    let product = u32::from(channel).saturating_mul(u32::from(alpha));
    let scaled = product.checked_div(255).unwrap_or(0);
    u8::try_from(scaled).unwrap_or(u8::MAX)
}

fn pack_argb(alpha: u8, red: u8, green: u8, blue: u8) -> u32 {
    let alpha = u32::from(alpha).checked_shl(24).unwrap_or(0);
    let red = u32::from(red).checked_shl(16).unwrap_or(0);
    let green = u32::from(green).checked_shl(8).unwrap_or(0);
    alpha | red | green | u32::from(blue)
}

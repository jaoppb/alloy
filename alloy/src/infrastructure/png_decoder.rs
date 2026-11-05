//! [`ImageDecoder`] over `graphics::png` — the one place `alloy` decodes bytes.

use graphics::Framebuffer;

use crate::application::ports::ImageDecoder;
use crate::error::AlloyError;

/// Decodes 8-bit RGB/RGBA PNGs with `graphics::png::decode`.
#[derive(Clone, Copy, Debug, Default)]
pub struct PngImageDecoder;

impl ImageDecoder for PngImageDecoder {
    fn decode(&self, bytes: &[u8]) -> Result<Framebuffer, AlloyError> {
        Ok(graphics::png::decode(bytes)?)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use graphics::{Color, Framebuffer, SurfaceSize};

    use super::PngImageDecoder;
    use crate::application::ports::ImageDecoder;
    use crate::error::AlloyError;

    #[test]
    fn bytes_that_are_not_a_png_are_a_typed_error() {
        let result = PngImageDecoder.decode(b"not a png");

        assert!(matches!(result, Err(AlloyError::ImageDecode(_))));
    }

    #[test]
    fn an_encoded_framebuffer_decodes_back_to_the_same_pixels() {
        let size = SurfaceSize::new(2, 1).unwrap();
        let original = Framebuffer::filled(size, Color::TRANSPARENT).unwrap();
        let bytes = graphics::png::encode(&original);

        let decoded = PngImageDecoder.decode(&bytes).unwrap();

        assert_eq!(decoded, original);
    }
}

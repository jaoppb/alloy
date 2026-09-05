//! Render pipeline: parses HTML, computes styles, calculates layout, paints to
//! a display list, and rasterizes via the software CPU backend.
//!
//! [`render_dom`] is the reusable core both the headless PNG path (v0.5 I2,
//! [`render_html_to_png`]) and the native-window event loop
//! (v0.5 I4, [`crate::application::event_loop`]) build on — the only
//! difference between the two is what happens to the [`graphics::Framebuffer`]
//! it produces (encoded to a PNG file, or blitted to a live window).

use std::collections::BTreeMap;
use std::sync::Arc;

use css::{
    BlockLayout, CascadeResolver, LayoutEngine, StyleSheetSet, UaCascade, ViewportConstraints,
};
use graphics::{
    Au, DisplayListBuilder, Framebuffer, ImageId, ImageProvider, InMemoryImageProvider,
    RenderBackend, SoftwareCpuBackend, SurfaceSize, SyntheticFontProvider,
};

use crate::application::paint::{DEFAULT_FONT, paint_box_tree};
use crate::error::AlloyError;

/// The font size used for synthetic font registration in headless rendering (16px).
const DEFAULT_FONT_SIZE: Au = Au::from_raw(16 * graphics::AU_PER_PX);

/// Sizing and layout configuration for headless HTML rendering.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RenderOptions {
    /// Viewport width in pixels.
    pub width: u32,
    /// Viewport height in pixels.
    pub height: u32,
}

impl RenderOptions {
    /// Default viewport width (800px).
    pub const DEFAULT_WIDTH: u32 = 800;
    /// Default viewport height (600px).
    pub const DEFAULT_HEIGHT: u32 = 600;

    /// Creates new render options with the specified dimensions.
    #[must_use]
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// Converts width and height into a valid [`SurfaceSize`], or returns
    /// [`AlloyError::InvalidDimensions`] if either dimension is zero.
    pub fn surface_size(&self) -> Result<SurfaceSize, AlloyError> {
        SurfaceSize::new(self.width, self.height).ok_or(AlloyError::InvalidDimensions)
    }
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            width: Self::DEFAULT_WIDTH,
            height: Self::DEFAULT_HEIGHT,
        }
    }
}

/// Renders HTML source text to a PNG byte vector using the specified options.
pub fn render_html_to_png(html_str: &str, options: &RenderOptions) -> Result<Vec<u8>, AlloyError> {
    let surface_size = options.surface_size()?;
    let dom_tree = html::parse(html_str)?;
    let framebuffer = render_dom(
        &dom_tree,
        StyleSheetSet::default(),
        &BTreeMap::new(),
        surface_size,
    )?;
    Ok(graphics::png::encode(&framebuffer))
}

/// Convenience function to render HTML with the given dimensions to PNG bytes.
pub fn run_render(source_html: &str, width: u32, height: u32) -> Result<Vec<u8>, AlloyError> {
    let options = RenderOptions::new(width, height);
    render_html_to_png(source_html, &options)
}

/// Renders an already-parsed document: snapshot, cascade, layout, paint,
/// rasterize.
///
/// `extra_sheets` is absorbed into the document's own `<style>`/`style=`
/// rules at `Origin::Author` precedence — how the v0.5 I4 event loop feeds in
/// a fetched `<link rel=stylesheet>` without `core/css` ever fetching
/// anything itself. `images` is looked up once per node whose box is
/// [`css::IntrinsicSize::Pending`] (every `<img>`/`<video>`/… box, loaded or
/// not, `core/css/src/domain/computed/intrinsic.rs`): a discovered image with
/// no entry yet must still resolve to *something* (a placeholder — see
/// `crate::application::subresource::placeholder_framebuffer`), or the whole
/// frame fails with `GraphicsError::ImageUnavailable` before a single fetch
/// has had a chance to complete.
pub fn render_dom(
    dom_tree: &dom::DomTree,
    extra_sheets: StyleSheetSet,
    images: &BTreeMap<ImageId, Framebuffer>,
    surface_size: SurfaceSize,
) -> Result<Framebuffer, AlloyError> {
    let snapshot = css::snapshot(dom_tree, dom_tree.document());
    let mut sheets = css::collect_style_sheets(&snapshot)?;
    sheets.absorb(extra_sheets);
    let styled_tree = UaCascade::new().resolve(&snapshot, &sheets)?;
    let constraints = make_constraints(surface_size)?;
    let box_tree = BlockLayout::new().layout(&styled_tree, &constraints)?;

    let font_provider =
        Arc::new(SyntheticFontProvider::new().with_size(DEFAULT_FONT, DEFAULT_FONT_SIZE));
    let mut builder = DisplayListBuilder::new();
    paint_box_tree(
        &box_tree,
        &styled_tree,
        font_provider.as_ref(),
        &mut builder,
    )?;
    let display_list = builder.build()?;

    let image_provider: Arc<dyn ImageProvider> = Arc::new(build_image_provider(images));
    let mut backend = SoftwareCpuBackend::with_providers(font_provider, image_provider);
    backend.begin_frame(surface_size)?;
    backend.submit(&display_list)?;
    backend.end_frame()?;
    Ok(backend.read_back()?)
}

/// Folds every currently-known image into a fresh, immutable provider — the
/// same "rebuild from an immutable snapshot" discipline `UaCascade`/
/// `BlockLayout` already use, applied to the one piece of render state that
/// changes over a page's lifetime (`core/graphics`'s `InMemoryImageProvider`
/// is a consuming builder specifically so no lock is needed here).
fn build_image_provider(images: &BTreeMap<ImageId, Framebuffer>) -> InMemoryImageProvider {
    images
        .iter()
        .fold(InMemoryImageProvider::new(), |provider, (&id, frame)| {
            provider.with_image(id, frame.clone())
        })
}

fn make_constraints(surface_size: SurfaceSize) -> Result<ViewportConstraints, AlloyError> {
    let width_px =
        i32::try_from(surface_size.width()).map_err(|_| AlloyError::InvalidDimensions)?;
    let height_px =
        i32::try_from(surface_size.height()).map_err(|_| AlloyError::InvalidDimensions)?;
    let width_au = Au::from_whole_px(width_px).ok_or(AlloyError::InvalidDimensions)?;
    let height_au = Au::from_whole_px(height_px).ok_or(AlloyError::InvalidDimensions)?;
    Ok(ViewportConstraints::new(width_au, height_au))
}

//! Render pipeline: parses HTML, computes styles, calculates layout, paints to
//! a display list, and rasterizes via the software CPU backend.
//!
//! [`render_dom`] is the reusable core both the headless PNG path (v0.5 I2,
//! [`render_html_to_png`]) and the native-window event loop
//! (v0.5 I4, [`crate::application::event_loop`]) build on — the only
//! difference between the two is what happens to the [`graphics::Framebuffer`]
//! it produces (encoded to a PNG file, or blitted to a live window).

use std::sync::Arc;

use css::{
    BlockLayout, CascadeResolver, DomSnapshot, FontBackedMeasurer, LayoutBoxTree, LayoutEngine,
    StyleSheetSet, TextMeasurer, UaCascade, ViewportConstraints,
};
use graphics::{
    Au, DisplayListBuilder, FontProvider, Framebuffer, Rect, RenderBackend, SoftwareCpuBackend,
    SurfaceSize, SyntheticFontProvider,
};

use crate::application::image_store::ImageStore;
use crate::application::paint::{DEFAULT_FONT, paint_box_tree};
use crate::application::runtime_font::RuntimeFontProvider;
use crate::error::AlloyError;

/// The font size used for font registration in rendering (16px).
pub const DEFAULT_FONT_SIZE: Au = Au::from_raw(16 * graphics::AU_PER_PX);

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

/// Resolves the default font provider for runtime rendering — see
/// [`RuntimeFontProvider::resolve`].
#[must_use]
pub fn default_runtime_font_provider() -> Arc<RuntimeFontProvider> {
    Arc::new(RuntimeFontProvider::resolve())
}

/// Renders HTML source text to a PNG byte vector using the specified options and synthetic font.
pub fn render_html_to_png(html_str: &str, options: &RenderOptions) -> Result<Vec<u8>, AlloyError> {
    let surface_size = options.surface_size()?;
    let dom_tree = html::parse(html_str)?;
    let framebuffer = render_dom(
        &dom_tree,
        StyleSheetSet::default(),
        &ImageStore::new(),
        surface_size,
    )?;
    Ok(graphics::png::encode(&framebuffer))
}

/// Renders HTML source text to a PNG byte vector using a custom [`FontProvider`].
pub fn render_html_with_font_provider<F: FontProvider + 'static>(
    html_str: &str,
    options: &RenderOptions,
    font_provider: Arc<F>,
) -> Result<Vec<u8>, AlloyError> {
    let surface_size = options.surface_size()?;
    let dom_tree = html::parse(html_str)?;
    let framebuffer = render_dom_with_font_provider(
        &dom_tree,
        StyleSheetSet::default(),
        &ImageStore::new(),
        surface_size,
        font_provider,
    )?;
    Ok(graphics::png::encode(&framebuffer))
}

/// Convenience function to render HTML with the given dimensions to PNG bytes.
pub fn run_render(source_html: &str, width: u32, height: u32) -> Result<Vec<u8>, AlloyError> {
    let options = RenderOptions::new(width, height);
    render_html_to_png(source_html, &options)
}

/// A clickable link region within the rendered document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkTarget {
    /// Bounding rectangle in document space (`Au`).
    pub area: Rect,
    /// The link destination (`href`).
    pub href: String,
}

/// Renders an already-parsed document using the deterministic [`SyntheticFontProvider`].
///
/// `extra_sheets` is absorbed into the document's own `<style>`/`style=`
/// rules at `Origin::Author` precedence — how the v0.5 I4 event loop feeds in
/// a fetched `<link rel=stylesheet>` without `core/css` ever fetching
/// anything itself. `images` maps discovered images to their pixel data; missing
/// images fallback cleanly to a placeholder without crashing.
pub fn render_dom(
    dom_tree: &dom::DomTree,
    extra_sheets: StyleSheetSet,
    images: &ImageStore,
    surface_size: SurfaceSize,
) -> Result<Framebuffer, AlloyError> {
    let font_provider =
        Arc::new(SyntheticFontProvider::new().with_size(DEFAULT_FONT, DEFAULT_FONT_SIZE));
    let (framebuffer, _) = render_document(
        dom_tree,
        extra_sheets,
        images,
        surface_size,
        font_provider,
        &BlockLayout::monospace(),
    )?;
    Ok(framebuffer)
}

/// Renders an already-parsed document with a specified [`FontProvider`] and real font metrics.
pub fn render_dom_with_font_provider<F: FontProvider + 'static>(
    dom_tree: &dom::DomTree,
    extra_sheets: StyleSheetSet,
    images: &ImageStore,
    surface_size: SurfaceSize,
    font_provider: Arc<F>,
) -> Result<Framebuffer, AlloyError> {
    let (framebuffer, _) =
        render_dom_with_links(dom_tree, extra_sheets, images, surface_size, font_provider)?;
    Ok(framebuffer)
}

/// Renders an already-parsed document with real font metrics, returning both
/// the rendered [`Framebuffer`] and clickable [`LinkTarget`]s.
pub fn render_dom_with_links<F: FontProvider + 'static>(
    dom_tree: &dom::DomTree,
    extra_sheets: StyleSheetSet,
    images: &ImageStore,
    surface_size: SurfaceSize,
    font_provider: Arc<F>,
) -> Result<(Framebuffer, Vec<LinkTarget>), AlloyError> {
    // `FontBackedMeasurer` (core/css) still takes a trait object; the `Arc<F>`
    // coerces at this one boundary, so alloy names no `dyn` itself.
    let measured_fonts = Arc::clone(&font_provider);
    let measurer = FontBackedMeasurer::new(measured_fonts, DEFAULT_FONT);
    render_document(
        dom_tree,
        extra_sheets,
        images,
        surface_size,
        font_provider,
        &BlockLayout::new(measurer),
    )
}

fn render_document<F: FontProvider + 'static, M: TextMeasurer + Send + Sync>(
    dom_tree: &dom::DomTree,
    extra_sheets: StyleSheetSet,
    images: &ImageStore,
    surface_size: SurfaceSize,
    font_provider: Arc<F>,
    layout: &BlockLayout<M>,
) -> Result<(Framebuffer, Vec<LinkTarget>), AlloyError> {
    let snapshot = css::snapshot(dom_tree, dom_tree.document());
    let mut sheets = css::collect_style_sheets(&snapshot)?;
    sheets.absorb(extra_sheets);
    let styled_tree = UaCascade::new().resolve(&snapshot, &sheets)?;
    let constraints = make_constraints(surface_size)?;
    let box_tree = layout.layout(&styled_tree, &constraints)?;

    let link_targets = collect_link_targets(&box_tree, &snapshot);

    let mut builder = DisplayListBuilder::new();
    paint_box_tree(
        &box_tree,
        &styled_tree,
        images,
        font_provider.as_ref(),
        &mut builder,
    )?;
    let display_list = builder.build()?;

    let image_provider = Arc::new(images.snapshot());
    let mut backend = SoftwareCpuBackend::with_providers(font_provider, image_provider);
    backend.begin_frame(surface_size)?;
    backend.submit(&display_list)?;
    backend.end_frame()?;
    let framebuffer = backend.read_back()?;
    Ok((framebuffer, link_targets))
}

fn collect_link_targets(box_tree: &LayoutBoxTree, snapshot: &DomSnapshot) -> Vec<LinkTarget> {
    let mut targets = Vec::new();
    for laid_out in box_tree.boxes_in_document_order() {
        let border_box = laid_out.border_box();
        if border_box.is_empty() {
            continue;
        }
        let mut current_id = Some(laid_out.node());
        while let Some(id) = current_id {
            let Some(node) = snapshot.node(id) else {
                break;
            };
            if node.tag_str() == Some("a")
                && let Some(href) = node.attribute("href")
            {
                targets.push(LinkTarget {
                    area: border_box,
                    href: href.to_string(),
                });
                break;
            }
            current_id = node.parent();
        }
    }
    targets
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

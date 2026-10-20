//! Session state: the live document, its subresources, and the frame last
//! presented from them. How background-fetch results change that state lives
//! in [`messages`].

use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};

use css::StyleSheetSet;
use dom::DomTree;
use graphics::{DisplayListBuilder, FontProvider, RenderBackend, SoftwareCpuBackend};
use network::{HttpTransport, RequestPolicy, Url};
use window::{PhysicalPosition, Presenter, SurfaceSize};

mod messages;

use super::frame::CachedFrame;
pub use super::render_tree::{CHROME_HEIGHT, ChromeHeight, UiHitTarget, UiRenderTree};
use super::stats::LoopStats;
use super::worker::{LoopMessage, spawn_navigation};
use crate::application::browser_services::BrowserServices;
use crate::application::image_store::ImageStore;
use crate::application::omnibox::OmniboxState;
use crate::application::paint::DEFAULT_FONT;
use crate::application::pipeline::{LinkTarget, render_dom_with_links};
use crate::application::subresource::SubresourceDiscoverer;
use crate::error::AlloyError;

/// Everything one pump cycle can mutate, bundled so `pump_once` stays under
/// the arity limit: the accumulated document state, the current viewport, and
/// the running relayout count.
///
/// Rebuilding a fresh display list from `dom_tree`/`extra_sheets`/`images` on
/// every relayout is the same "immutable snapshot in, immutable snapshot out"
/// discipline the render pipeline itself uses (`ADR-0010:114-117`), applied
/// to the state a live session must keep between frames.
///
/// Every field is private: `dirty` → [`Session::relayout`] → clean, and
/// `links` always describing `last_frame`, are invariants only this type's
/// methods may move.
pub struct Session<F, T, P, D> {
    services: BrowserServices<F, T, P, D>,
    dom_tree: Option<DomTree>,
    /// The document's effective base URL — `<base href>` already applied —
    /// that both subresource discovery and link clicks resolve against.
    base_url: Option<Url>,
    extra_sheets: StyleSheetSet,
    images: ImageStore,
    /// The link areas of `last_frame`, in paint order.
    links: Vec<LinkTarget>,
    pointer_position: Option<PhysicalPosition>,
    dirty: bool,
    viewport: SurfaceSize,
    last_frame: Option<CachedFrame>,
    stats: LoopStats,
    /// Where the workers of the current navigation report. A worker cannot be
    /// cancelled, so [`Session::open_channel`] replaces both ends on every
    /// navigation: dropping the old `inbox` discards what its workers already
    /// queued and makes their later `send` fail, so a result from a page the
    /// user left has no way back in. That includes an `ImageId` — a DOM node
    /// index reused by every document — naming an `<img>` of the new page.
    inbox: Receiver<LoopMessage>,
    outbox: Sender<LoopMessage>,
    omnibox: OmniboxState,
    ui_tree: UiRenderTree,
}

impl<F, T, P, D> Session<F, T, P, D>
where
    F: FontProvider + 'static,
    T: HttpTransport + 'static,
    P: RequestPolicy + 'static,
    D: SubresourceDiscoverer,
{
    pub fn new(viewport: SurfaceSize, services: BrowserServices<F, T, P, D>) -> Self {
        let (outbox, inbox) = mpsc::channel();
        let ui_tree = UiRenderTree::build(viewport, ChromeHeight::DEFAULT, &[])
            .unwrap_or_else(|_| UiRenderTree::fallback(viewport));
        Self {
            services,
            dom_tree: None,
            base_url: None,
            extra_sheets: StyleSheetSet::new(),
            images: ImageStore::new(),
            links: Vec::new(),
            pointer_position: None,
            dirty: false,
            viewport,
            last_frame: None,
            stats: LoopStats::default(),
            inbox,
            outbox,
            omnibox: OmniboxState::new(),
            ui_tree,
        }
    }

    pub const fn stats(&self) -> LoopStats {
        self.stats
    }

    pub const fn pointer_position(&self) -> Option<PhysicalPosition> {
        self.pointer_position
    }

    /// Read-only access to the session's address bar state.
    #[must_use]
    #[cfg(test)]
    pub const fn omnibox(&self) -> &OmniboxState {
        &self.omnibox
    }

    /// Read-only access to the session's UI render tree.
    #[must_use]
    #[cfg(test)]
    pub const fn ui_tree(&self) -> &UiRenderTree {
        &self.ui_tree
    }

    /// Where the pointer was last seen — `None` until it first enters.
    pub const fn track_pointer(&mut self, position: Option<PhysicalPosition>) {
        self.pointer_position = position;
    }

    /// Adopts the new viewport; the next [`Session::relayout`] lays out at it.
    pub fn resize(&mut self, viewport: SurfaceSize) {
        self.viewport = viewport;
        if let Ok(tree) = UiRenderTree::build(viewport, ChromeHeight::DEFAULT, &self.links) {
            self.ui_tree = tree;
        }
        self.dirty = true;
    }

    pub const fn needs_relayout(&self) -> bool {
        self.dirty
    }

    /// Starts fetching `url` on a worker thread; the result arrives as a
    /// [`LoopMessage::Navigation`]. Supersedes every navigation and
    /// subresource fetch still in flight: they report on a channel nobody
    /// reads any more.
    pub fn navigate(&mut self, url: Url) {
        self.open_channel();
        self.omnibox.reset_to_url(&url);
        let transport = Arc::clone(self.services.transport());
        let policy = Arc::clone(self.services.policy());
        spawn_navigation(url, transport, policy, self.outbox.clone());
    }

    /// Dispatches a pointer click at `position` through the UI render tree:
    /// - Clicks on the omnibox or chrome top bar focus the omnibox.
    /// - Clicks on a hyperlink navigate to the resolved destination.
    /// - Clicks on empty page content do nothing.
    pub fn follow_link_at(&mut self, position: PhysicalPosition) {
        let Some(target) = self.ui_tree.hit_test(position) else {
            return;
        };
        match target {
            UiHitTarget::Omnibox | UiHitTarget::Chrome => {
                self.omnibox.focus();
            }
            UiHitTarget::PageLink(href) => {
                let href = href.to_owned();
                self.follow_link(&href);
            }
            UiHitTarget::PageContent => {}
        }
    }

    fn follow_link(&mut self, href: &str) {
        let Some(base_url) = self.base_url.as_ref() else {
            return;
        };
        if href.starts_with('#') {
            tracing::info!(anchor = href, "in-page anchor clicked (no-op in v0.5)");
            return;
        }
        match base_url.join(href) {
            Ok(target_url) => {
                tracing::info!(url = %target_url, "link clicked, navigating");
                self.navigate(target_url);
            }
            Err(error) => tracing::warn!(href, %error, "failed to resolve link target"),
        }
    }

    /// Rebuilds the display list from the current document, viewport and
    /// subresources, presents it, and caches the pixels for a later cheap
    /// [`Session::repaint`]. The only path that bumps `stats.relayouts`, and
    /// only once a frame was actually presented — the I4 coalescing proof
    /// and `run_browser_until_first_frame` both rest on that. With no
    /// document yet there is nothing to lay out, and the request is dropped.
    pub fn relayout<R: Presenter>(&mut self, presenter: &mut R) -> Result<(), AlloyError> {
        self.dirty = false;
        let Some(dom_tree) = self.dom_tree.as_ref() else {
            return Ok(());
        };
        let graphics_size = self.ui_tree.content_size()?;
        let (content_framebuffer, links) = render_dom_with_links(
            dom_tree,
            self.extra_sheets.clone(),
            &self.images,
            graphics_size,
            Arc::clone(self.services.font_provider()),
        )?;
        let chrome_framebuffer = render_chrome(
            &self.omnibox,
            self.viewport.width(),
            Arc::clone(self.services.font_provider()),
        )?;
        let frame = CachedFrame::compose(&chrome_framebuffer, &content_framebuffer, self.viewport)?;
        frame.present(presenter)?;
        self.ui_tree = UiRenderTree::build(self.viewport, ChromeHeight::DEFAULT, &links)?;
        self.links = links;
        self.last_frame = Some(frame);
        self.stats.relayouts = self.stats.relayouts.saturating_add(1);
        Ok(())
    }

    /// Re-blits the frame [`Session::relayout`] last produced, with no
    /// pipeline work and without touching `stats.relayouts`. A no-op before
    /// the first frame. Serves `RedrawRequested`: a compositor
    /// expose/occlusion, or the redraw winit re-arms once a Wayland surface
    /// that dropped the first present is finally configured.
    pub fn repaint<R: Presenter>(&self, presenter: &mut R) -> Result<(), AlloyError> {
        let Some(frame) = self.last_frame.as_ref() else {
            return Ok(());
        };
        frame.present(presenter)
    }
}

impl<F, T, P, D> Session<F, T, P, D> {
    fn open_channel(&mut self) {
        (self.outbox, self.inbox) = mpsc::channel();
    }
}

const CHROME_BG: graphics::Color = graphics::Color::rgb(0xF1, 0xF3, 0xF4);
const INPUT_BG: graphics::Color = graphics::Color::WHITE;
const TEXT_COLOR: graphics::Color = graphics::Color::rgb(0x20, 0x21, 0x24);
const FOCUS_OUTLINE_COLOR: graphics::Color = graphics::Color::rgb(0x1A, 0x73, 0xE8);

fn render_chrome<F: FontProvider + 'static>(
    omnibox: &OmniboxState,
    viewport_width: u32,
    font_provider: Arc<F>,
) -> Result<graphics::Framebuffer, AlloyError> {
    let chrome_size = graphics::SurfaceSize::new(viewport_width, CHROME_HEIGHT)
        .ok_or(AlloyError::InvalidDimensions)?;
    let mut builder = DisplayListBuilder::new();
    paint_chrome(
        omnibox,
        viewport_width,
        font_provider.as_ref(),
        &mut builder,
    )?;
    let display_list = builder.build()?;

    let mut backend = SoftwareCpuBackend::with_font_provider(font_provider);
    backend.begin_frame(chrome_size)?;
    backend.submit(&display_list)?;
    backend.end_frame()?;
    let framebuffer = backend.read_back()?;
    Ok(framebuffer)
}

const CHROME_HEIGHT_F32: f32 = 42.0;

#[allow(clippy::as_conversions, clippy::cast_precision_loss)]
const fn u32_to_f32(value: u32) -> f32 {
    value as f32
}

fn paint_chrome<F: FontProvider>(
    omnibox: &OmniboxState,
    viewport_width: u32,
    font_provider: &F,
    builder: &mut DisplayListBuilder,
) -> Result<(), graphics::GraphicsError> {
    let width_f32 = u32_to_f32(viewport_width);
    let height_f32 = CHROME_HEIGHT_F32;

    let chrome_area = graphics::PxRect::from_px(0.0, 0.0, width_f32, height_f32);
    builder.draw_rect(chrome_area, CHROME_BG)?;

    let margin_x = 8.0f32;
    let box_height = 30.0f32;
    let margin_y = ((height_f32 - box_height) / 2.0).max(0.0);
    let box_width = (width_f32 - (margin_x + margin_x)).max(0.0);

    paint_focus_ring(omnibox, margin_x, margin_y, box_width, box_height, builder)?;

    let input_area = graphics::PxRect::from_px(margin_x, margin_y, box_width, box_height);
    builder.draw_rounded_rect(input_area, INPUT_BG, graphics::Px::new(6.0))?;

    paint_chrome_text(omnibox, font_provider, builder)?;
    Ok(())
}

fn paint_focus_ring(
    omnibox: &OmniboxState,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    builder: &mut DisplayListBuilder,
) -> Result<(), graphics::GraphicsError> {
    if !omnibox.is_focused() {
        return Ok(());
    }
    let border_w = 1.0f32;
    let outline_area = graphics::PxRect::from_px(
        x - border_w,
        y - border_w,
        width + border_w + border_w,
        height + border_w + border_w,
    );
    builder.draw_rounded_rect(outline_area, FOCUS_OUTLINE_COLOR, graphics::Px::new(7.0))
}

fn paint_chrome_text<F: FontProvider>(
    omnibox: &OmniboxState,
    font_provider: &F,
    builder: &mut DisplayListBuilder,
) -> Result<(), graphics::GraphicsError> {
    let text_cow = omnibox.text();
    let active_url_text = omnibox.active_url().map(network::Url::to_text);
    let display_text = select_display_text(&text_cow, active_url_text.as_deref());
    if display_text.is_empty() {
        return Ok(());
    }

    let metrics = font_provider.metrics(DEFAULT_FONT)?;
    let start_x = graphics::Au::from_whole_px(18).unwrap_or(graphics::Au::ZERO);
    let middle_y = graphics::Au::from_whole_px(21).unwrap_or(graphics::Au::ZERO);
    let half_line = metrics.line_height().raw().checked_div(2).unwrap_or(0);
    let top_y = graphics::Au::from_raw(middle_y.raw().saturating_sub(half_line));
    let baseline_y = top_y.saturating_add(metrics.ascent());

    let mut run = graphics::GlyphRun::new();
    let mut pen_x = start_x;
    for ch in display_text.chars() {
        let glyph = font_provider.glyph_for_char(DEFAULT_FONT, ch)?;
        if glyph != graphics::GlyphId::NOTDEF {
            run.push(graphics::GlyphInstance::new(
                glyph,
                graphics::Point::new(pen_x, baseline_y),
            ));
        }
        let advance = font_provider.advance(DEFAULT_FONT, glyph)?;
        pen_x = pen_x.saturating_add(advance);
    }

    if !run.is_empty() {
        builder.draw_text(run, TEXT_COLOR, DEFAULT_FONT)?;
    }
    Ok(())
}

const fn select_display_text<'a>(buffer_text: &'a str, active_url: Option<&'a str>) -> &'a str {
    if !buffer_text.is_empty() {
        return buffer_text;
    }
    let Some(url) = active_url else {
        return "";
    };
    url
}

/// Read-only views the `pump_once` tests assert on.
#[cfg(test)]
impl<F, T, P, D> Session<F, T, P, D> {
    /// A clone of the sender the current page's workers hold.
    pub fn outbox_for_test(&self) -> Sender<LoopMessage> {
        self.outbox.clone()
    }

    /// Supersedes the current page the way [`Session::navigate`] does, without
    /// spawning a fetch.
    pub fn begin_navigation_for_test(&mut self) {
        self.open_channel();
    }

    pub const fn viewport(&self) -> SurfaceSize {
        self.viewport
    }

    pub const fn base_url(&self) -> Option<&Url> {
        self.base_url.as_ref()
    }

    pub const fn has_frame(&self) -> bool {
        self.last_frame.is_some()
    }

    pub const fn has_links(&self) -> bool {
        !self.links.is_empty()
    }
}

//! The v0.5 Phase I4 event loop (`ADR-0019`).
//!
//! A single [`WindowSystem`] owns the main thread. Every blocking fetch —
//! navigation, a stylesheet, an image — runs on its own `std::thread`; its
//! result comes back over `std::sync::mpsc` as one more event this loop
//! drains. **No async runtime.**
//!
//! Coalescing is the same mechanism for both resize and subresource arrival:
//! one pump cycle drains *every* window event and *every* queued background
//! result before deciding whether to relay out, and relays out **at most
//! once** per cycle. Ten resizes or fifty image arrivals queued between two
//! pump cycles cost one relayout, not ten or fifty.

use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use css::{Origin, StyleSheetSet};
use dom::DomTree;
use graphics::{Au, FontProvider, Framebuffer, ImageId};
use network::{HttpRequest, HttpTransport, RequestPolicy, Url};
use window::{
    FrameView, PhysicalPosition, PointerButton, Presenter, PumpStatus, WindowAttributes,
    WindowEvent, WindowSystem, WindowTitle,
};

use crate::application::browser_services::BrowserServices;
use crate::application::image_store::ImageStore;
use crate::application::navigation;
use crate::application::pipeline::{LinkTarget, RenderOptions, render_dom_with_links};
use crate::application::subresource::{SubresourceDiscoverer, SubresourceRequest};
use crate::error::AlloyError;

/// What a background fetch produced, drained by the loop's own thread.
enum LoopMessage {
    Navigation(Result<(DomTree, Url), AlloyError>),
    Stylesheet(Result<String, AlloyError>),
    Image(ImageId, Result<Framebuffer, AlloyError>),
}

/// What a run of the loop did so far.
///
/// Instrumented for I4's coalescing proofs (resize, subresource bursts) and
/// for a caller (or a test) that needs to wait for a specific piece of
/// background work to land before looking at the presented frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LoopStats {
    /// How many times this run actually re-laid-out and presented a frame.
    pub relayouts: usize,
    /// How many navigations have successfully parsed into the document tree.
    pub navigations: usize,
    /// How many external `<link rel=stylesheet>` sheets were absorbed into the
    /// cascade.
    pub stylesheets_loaded: usize,
    /// How many `<img>` fetches have decoded and replaced their placeholder.
    pub images_loaded: usize,
}

/// Everything one pump cycle can mutate, bundled so `pump_once` stays under
/// the arity limit: the accumulated document state, the current viewport, and
/// the running relayout count.
///
/// Rebuilding a fresh display list from `dom_tree`/`extra_sheets`/`images` on
/// every relayout is the same "immutable snapshot in, immutable snapshot out"
/// discipline the render pipeline itself uses (`ADR-0010:114-117`), applied
/// to the state a live session must keep between frames.
struct Session<F, T, P, D> {
    services: BrowserServices<F, T, P, D>,
    dom_tree: Option<DomTree>,
    base_url: Option<Url>,
    extra_sheets: StyleSheetSet,
    images: ImageStore,
    links: Vec<LinkTarget>,
    pointer_pos: Option<PhysicalPosition>,
    dirty: bool,
    viewport: window::SurfaceSize,
    stats: LoopStats,
}

impl<F, T, P, D> Session<F, T, P, D>
where
    F: FontProvider + 'static,
    T: HttpTransport + 'static,
    P: RequestPolicy + 'static,
    D: SubresourceDiscoverer,
{
    fn new(viewport: window::SurfaceSize, services: BrowserServices<F, T, P, D>) -> Self {
        Self {
            services,
            dom_tree: None,
            base_url: None,
            extra_sheets: StyleSheetSet::new(),
            images: ImageStore::new(),
            links: Vec::new(),
            pointer_pos: None,
            dirty: false,
            viewport,
            stats: LoopStats::default(),
        }
    }

    /// Applies one drained background-fetch result, spawning whatever
    /// follow-up fetches it reveals (a fresh document's subresources).
    fn apply(&mut self, message: LoopMessage, sender: &Sender<LoopMessage>) {
        match message {
            LoopMessage::Navigation(Ok((dom_tree, base_url))) => {
                tracing::info!(url = %base_url, "navigation complete");
                self.reset_document_state();
                self.base_url = Some(base_url.clone());
                self.spawn_subresources(&dom_tree, &base_url, sender);
                self.dom_tree = Some(dom_tree);
                self.dirty = true;
                self.stats.navigations = self.stats.navigations.saturating_add(1);
            }
            LoopMessage::Navigation(Err(error)) => {
                tracing::error!(%error, "navigation failed");
                self.show_navigation_error(&error);
            }
            LoopMessage::Stylesheet(Ok(text)) => self.absorb_stylesheet(&text),
            LoopMessage::Stylesheet(Err(error)) => {
                tracing::warn!(%error, "stylesheet fetch failed");
            }
            LoopMessage::Image(id, Ok(framebuffer)) => {
                self.images.insert(id, framebuffer);
                self.dirty = true;
                self.stats.images_loaded = self.stats.images_loaded.saturating_add(1);
            }
            LoopMessage::Image(id, Err(error)) => {
                tracing::warn!(%error, %id, "image fetch failed");
            }
        }
    }

    /// A new document starts from no sheets, images or links.
    fn reset_document_state(&mut self) {
        self.extra_sheets = StyleSheetSet::new();
        self.images.clear();
        self.links.clear();
    }

    /// Replaces the page with a visible error document — a failed navigation
    /// must be seen, not just logged.
    fn show_navigation_error(&mut self, error: &AlloyError) {
        let escaped_error = error.to_string().replace('&', "&amp;").replace('<', "&lt;");
        let error_html = format!(
            "<!DOCTYPE html><html><head><title>Navigation Error</title><style>body {{ margin: 32px; background-color: #fdf2e9; color: #78281f; }} h1 {{ color: #c0392b; }} .error-box {{ background-color: #ffffff; padding: 16px; border-width: 2px; }}</style></head><body><h1>Navigation Error</h1><div class=\"error-box\"><p><strong>Failed to load:</strong> {escaped_error}</p></div></body></html>"
        );
        let Ok(error_tree) = html::parse(&error_html) else {
            return;
        };
        self.reset_document_state();
        self.dom_tree = Some(error_tree);
        self.dirty = true;
    }

    fn absorb_stylesheet(&mut self, text: &str) {
        if let Ok(sheet) = css::parse_stylesheet(text, Origin::Author) {
            self.extra_sheets.absorb(sheet);
            self.dirty = true;
            self.stats.stylesheets_loaded = self.stats.stylesheets_loaded.saturating_add(1);
        }
    }

    /// Asks the discoverer what `dom_tree` references, registers a
    /// placeholder for every image found (see
    /// `subresource::placeholder_framebuffer`), and spawns one worker thread
    /// per subresource.
    fn spawn_subresources(
        &mut self,
        dom_tree: &DomTree,
        base_url: &Url,
        sender: &Sender<LoopMessage>,
    ) {
        let snapshot = css::snapshot(dom_tree, dom_tree.document());
        let found = self.services.discoverer().discover(&snapshot, base_url);
        for request in found {
            if let SubresourceRequest::Image(image) = &request {
                self.images.reserve_placeholder(image.id());
            }
            spawn_subresource_fetch(request, Arc::clone(self.services.transport()), sender);
        }
    }

    const fn record_relayout(&mut self) {
        self.stats.relayouts = self.stats.relayouts.saturating_add(1);
    }
}

fn spawn_navigation<T, P>(url: Url, transport: Arc<T>, policy: Arc<P>, sender: Sender<LoopMessage>)
where
    T: HttpTransport + 'static,
    P: RequestPolicy + 'static,
{
    thread::spawn(move || {
        let result = navigation::navigate(&url, transport.as_ref(), policy.as_ref())
            .map(|dom_tree| (dom_tree, url));
        let _ = sender.send(LoopMessage::Navigation(result));
    });
}

fn spawn_subresource_fetch<T: HttpTransport + 'static>(
    request: SubresourceRequest,
    transport: Arc<T>,
    sender: &Sender<LoopMessage>,
) {
    let sender = sender.clone();
    thread::spawn(move || {
        let _ = sender.send(fetch_subresource(request, transport.as_ref()));
    });
}

/// The one place a [`SubresourceRequest`] variant maps to how it is fetched.
fn fetch_subresource<T: HttpTransport>(request: SubresourceRequest, transport: &T) -> LoopMessage {
    match request {
        SubresourceRequest::Stylesheet(stylesheet) => {
            LoopMessage::Stylesheet(fetch_text(stylesheet.url(), transport))
        }
        SubresourceRequest::Image(image) => {
            LoopMessage::Image(image.id(), fetch_image(image.url(), transport))
        }
    }
}

fn fetch_text<T: HttpTransport>(url: &Url, transport: &T) -> Result<String, AlloyError> {
    let response = transport.execute(&HttpRequest::get(url.clone()))?;
    Ok(response.body().as_str().unwrap_or_default().to_owned())
}

fn fetch_image<T: HttpTransport>(url: &Url, transport: &T) -> Result<Framebuffer, AlloyError> {
    let response = transport.execute(&HttpRequest::get(url.clone()))?;
    Ok(graphics::png::decode(response.body().as_bytes())?)
}

/// The window `alloy <url>` and the e2e golden test both open.
///
/// One shared definition so `run_browser`'s caller and its viewport-tracking
/// agree on the starting size.
///
/// # Errors
///
/// [`AlloyError::InvalidDimensions`] only if [`RenderOptions`]'s own defaults
/// were ever changed to zero — not reachable with the values in this crate.
pub fn initial_window_attributes() -> Result<WindowAttributes, AlloyError> {
    let size =
        window::SurfaceSize::new(RenderOptions::DEFAULT_WIDTH, RenderOptions::DEFAULT_HEIGHT)
            .ok_or(AlloyError::InvalidDimensions)?;
    Ok(WindowAttributes::new(WindowTitle::from("alloy"), size))
}

/// Runs a full browser session against `url`: navigates and pumps until the
/// window closes.
///
/// `system` and `presenter` must already have a live window —
/// [`WindowSystem::create_window`] with [`initial_window_attributes`] is the
/// caller's job, because the real `winit` [`Presenter`] adapter
/// (`SoftbufferPresenter`) needs the window handle `create_window` produces
/// to construct itself, and that handle is not part of the [`WindowSystem`]
/// trait this function is generic over.
///
/// Generic over [`WindowSystem`]/[`Presenter`] on purpose — the real `winit`
/// backend and the headless reference (`window::HeadlessWindowSystem` /
/// `RecordingPresenter`) drive the exact same loop, which is what lets the
/// e2e golden test exercise this function directly rather than a parallel
/// test-only copy of it.
pub fn run_browser<S, R, F, T, P, D>(
    url: &Url,
    services: BrowserServices<F, T, P, D>,
    system: &mut S,
    presenter: &mut R,
    initial_size: window::SurfaceSize,
) -> Result<LoopStats, AlloyError>
where
    S: WindowSystem,
    R: Presenter,
    F: FontProvider + 'static,
    T: HttpTransport + 'static,
    P: RequestPolicy + 'static,
    D: SubresourceDiscoverer,
{
    run_browser_until(url, services, system, presenter, initial_size, |_| false)
}

/// The same session as [`run_browser`], but returns as soon as `should_stop`
/// answers `true` for the accumulated [`LoopStats`], instead of waiting for
/// the window to close.
///
/// Useful for a test that needs to wait for a specific piece of background
/// work (a navigation, a stylesheet, an image) to land before inspecting the
/// presented frame, without racing the background fetch threads against a
/// scripted close event — and for a one-shot render, via
/// [`run_browser_until_first_frame`].
pub fn run_browser_until<S, R, F, T, P, D>(
    url: &Url,
    services: BrowserServices<F, T, P, D>,
    system: &mut S,
    presenter: &mut R,
    initial_size: window::SurfaceSize,
    should_stop: impl FnMut(&LoopStats) -> bool,
) -> Result<LoopStats, AlloyError>
where
    S: WindowSystem,
    R: Presenter,
    F: FontProvider + 'static,
    T: HttpTransport + 'static,
    P: RequestPolicy + 'static,
    D: SubresourceDiscoverer,
{
    let mut session = Session::new(initial_size, services);
    run_loop(url, system, presenter, &mut session, should_stop)
}

/// [`run_browser_until`], stopping as soon as the first frame has been laid
/// out and presented.
pub fn run_browser_until_first_frame<S, R, F, T, P, D>(
    url: &Url,
    services: BrowserServices<F, T, P, D>,
    system: &mut S,
    presenter: &mut R,
    initial_size: window::SurfaceSize,
) -> Result<LoopStats, AlloyError>
where
    S: WindowSystem,
    R: Presenter,
    F: FontProvider + 'static,
    T: HttpTransport + 'static,
    P: RequestPolicy + 'static,
    D: SubresourceDiscoverer,
{
    run_browser_until(url, services, system, presenter, initial_size, |stats| {
        stats.relayouts >= 1
    })
}

/// How long a pump cycle sleeps when neither a window event nor a
/// background-fetch result was waiting — keeps the loop from busy-spinning a
/// CPU core while a fetch is in flight, with no async runtime and no OS
/// blocking primitive spanning both the window and the `mpsc` channel.
const IDLE_POLL: std::time::Duration = std::time::Duration::from_millis(4);

fn run_loop<S, R, F, T, P, D>(
    url: &Url,
    system: &mut S,
    presenter: &mut R,
    session: &mut Session<F, T, P, D>,
    mut should_stop: impl FnMut(&LoopStats) -> bool,
) -> Result<LoopStats, AlloyError>
where
    S: WindowSystem,
    R: Presenter,
    F: FontProvider + 'static,
    T: HttpTransport + 'static,
    P: RequestPolicy + 'static,
    D: SubresourceDiscoverer,
{
    let (sender, receiver) = mpsc::channel();
    spawn_navigation(
        url.clone(),
        Arc::clone(session.services.transport()),
        Arc::clone(session.services.policy()),
        sender.clone(),
    );

    loop {
        let (outcome, did_work) = pump_once(system, presenter, &receiver, &sender, session)?;
        if outcome == PumpStatus::Exit || should_stop(&session.stats) {
            return Ok(session.stats);
        }
        if !did_work {
            thread::sleep(IDLE_POLL);
        }
    }
}

/// One iteration: drain every window event, drain every background-fetch
/// result waiting right now, and — only if something actually changed —
/// relay out and present exactly once.
///
/// The returned `bool` says whether this cycle observed *anything* (a window
/// event, a background message) — the caller uses it to decide whether to
/// idle-sleep before the next cycle, purely to avoid busy-spinning a CPU core;
/// it plays no part in the coalescing invariant itself, which is entirely
/// "drain everything currently available, then relay out at most once".
fn pump_once<S, R, F, T, P, D>(
    system: &mut S,
    presenter: &mut R,
    receiver: &Receiver<LoopMessage>,
    sender: &Sender<LoopMessage>,
    session: &mut Session<F, T, P, D>,
) -> Result<(PumpStatus, bool), AlloyError>
where
    S: WindowSystem,
    R: Presenter,
    F: FontProvider + 'static,
    T: HttpTransport + 'static,
    P: RequestPolicy + 'static,
    D: SubresourceDiscoverer,
{
    let mut close_requested = false;
    let mut latest_resize = None;
    let mut saw_window_event = false;
    let mut clicked_pos = None;
    let window_status = system.pump_events(&mut |event| {
        saw_window_event = true;
        match event {
            WindowEvent::CloseRequested => close_requested = true,
            WindowEvent::Resized(size) => latest_resize = Some(size),
            WindowEvent::PointerMoved { position } => session.pointer_pos = Some(position),
            WindowEvent::PointerButton {
                button: PointerButton::Left,
                pressed: true,
            } => {
                if let Some(pos) = session.pointer_pos {
                    clicked_pos = Some(pos);
                }
            }
            _ => {}
        }
    })?;

    if let Some(size) = latest_resize {
        session.viewport = size;
        session.dirty = true;
    }

    if let Some(pos) = clicked_pos
        && let Some(href) = hit_test(&session.links, pos)
        && let Some(base) = session.base_url.as_ref()
    {
        if href.starts_with('#') {
            tracing::info!(anchor = href, "in-page anchor clicked (no-op in v0.5)");
        } else if let Ok(target_url) = base.join(href) {
            tracing::info!(url = %target_url, "link clicked, navigating");
            spawn_navigation(
                target_url,
                Arc::clone(session.services.transport()),
                Arc::clone(session.services.policy()),
                sender.clone(),
            );
        } else {
            tracing::warn!(href, "failed to resolve link target against base URL");
        }
    }

    let mut saw_message = false;
    while let Ok(message) = receiver.try_recv() {
        saw_message = true;
        session.apply(message, sender);
    }

    if session.dirty {
        present_if_ready(presenter, session)?;
        session.record_relayout();
        session.dirty = false;
    }

    let did_work = saw_window_event || saw_message;

    if close_requested || window_status == PumpStatus::Exit {
        return Ok((PumpStatus::Exit, did_work));
    }
    Ok((PumpStatus::Continue, did_work))
}

#[allow(clippy::as_conversions, clippy::cast_possible_truncation)]
fn hit_test(links: &[LinkTarget], position: PhysicalPosition) -> Option<&str> {
    if !position.x().is_finite() || !position.y().is_finite() {
        return None;
    }
    let x_px = position.x().round() as i64;
    let y_px = position.y().round() as i64;
    let x_i32 = i32::try_from(x_px).ok()?;
    let y_i32 = i32::try_from(y_px).ok()?;
    let x_au = Au::from_whole_px(x_i32)?;
    let y_au = Au::from_whole_px(y_i32)?;
    for target in links.iter().rev() {
        let area = target.area;
        if x_au >= area.min_x()
            && x_au < area.max_x()
            && y_au >= area.min_y()
            && y_au < area.max_y()
        {
            return Some(&target.href);
        }
    }
    None
}

fn present_if_ready<R, F, T, P, D>(
    presenter: &mut R,
    session: &mut Session<F, T, P, D>,
) -> Result<(), AlloyError>
where
    R: Presenter,
    F: FontProvider + 'static,
{
    let Some(dom_tree) = session.dom_tree.as_ref() else {
        return Ok(());
    };
    let viewport = session.viewport;
    let graphics_size = graphics::SurfaceSize::new(viewport.width(), viewport.height())
        .ok_or(AlloyError::InvalidDimensions)?;
    let (framebuffer, links) = render_dom_with_links(
        dom_tree,
        session.extra_sheets.clone(),
        &session.images,
        graphics_size,
        Arc::clone(session.services.font_provider()),
    )?;
    session.links = links;
    let pixels = frame_pixels(&framebuffer);
    let view = FrameView::new(viewport.width(), viewport.height(), &pixels)
        .ok_or(AlloyError::InvalidDimensions)?;
    presenter.present(view)?;
    Ok(())
}

/// Straight-alpha `RGBA8` (`core/graphics`'s wire format) to premultiplied
/// `0xAARRGGBB` (`window::FrameView`'s — see its own doc comment).
fn frame_pixels(framebuffer: &Framebuffer) -> Vec<u32> {
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

/// Direct, thread-free proofs of the one invariant `06-i4-alloy-url.md` names
/// twice.
///
/// Resize and subresource-arrival coalescing collapse an arbitrary burst of
/// events into **at most one** relayout per pump cycle. Exercising
/// `pump_once` straight (rather than the threaded `run_browser`) is
/// deliberate: the property under test is what one drain-and-decide cycle
/// does with whatever is already queued, which has nothing to do with how
/// fast a background fetch thread happens to run — testing it through real
/// threads would make the proof racy for no added coverage.
#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::sync::mpsc;

    use graphics::SyntheticFontProvider;
    use network::{AllowAllPolicy, MockTransport};
    use window::{HeadlessWindowSystem, RecordingPresenter, SurfaceSize, WindowSystem as _};

    use super::{Arc, BrowserServices, ImageId, LoopMessage, Session, WindowEvent, pump_once};
    use crate::application::event_loop::initial_window_attributes;
    use crate::application::paint::DEFAULT_FONT;
    use crate::application::pipeline::DEFAULT_FONT_SIZE;
    use crate::application::subresource::{MarkupDiscoverer, placeholder_framebuffer};

    type TestSession =
        Session<SyntheticFontProvider, MockTransport, AllowAllPolicy, MarkupDiscoverer>;

    fn session_over(transport: MockTransport, viewport: SurfaceSize) -> TestSession {
        let font_provider =
            Arc::new(SyntheticFontProvider::new().with_size(DEFAULT_FONT, DEFAULT_FONT_SIZE));
        let services = BrowserServices::new(
            font_provider,
            Arc::new(transport),
            Arc::new(AllowAllPolicy::new()),
        );
        Session::new(viewport, services)
    }

    fn loaded_session(viewport: SurfaceSize) -> TestSession {
        let mut session = session_over(MockTransport::new(), viewport);
        session.dom_tree = Some(html::parse("<html><body>hi</body></html>").unwrap());
        session
    }

    #[test]
    fn multiple_resizes_in_one_pump_coalesce_to_one_relayout() {
        let attributes = initial_window_attributes().unwrap();
        let mut system = HeadlessWindowSystem::new();
        system.create_window(&attributes).unwrap();
        let bigger = SurfaceSize::new(1024, 768).unwrap();
        let smaller = SurfaceSize::new(640, 480).unwrap();
        system.schedule(WindowEvent::Resized(bigger));
        system.schedule(WindowEvent::Resized(smaller));

        let mut presenter = RecordingPresenter::new();
        let (sender, receiver) = mpsc::channel();
        let mut session = loaded_session(attributes.initial_size());

        pump_once(
            &mut system,
            &mut presenter,
            &receiver,
            &sender,
            &mut session,
        )
        .unwrap();

        assert_eq!(
            session.stats.relayouts, 1,
            "three coalesced Resized events (the initial one plus two scheduled) must cost exactly one relayout"
        );
        assert_eq!(
            session.viewport, smaller,
            "the viewport must reflect the LAST resize in the coalesced batch"
        );
    }

    #[test]
    fn fifty_image_arrivals_in_one_pump_coalesce_to_one_relayout() {
        let attributes = initial_window_attributes().unwrap();
        let mut system = HeadlessWindowSystem::new();
        system.create_window(&attributes).unwrap();

        let mut presenter = RecordingPresenter::new();
        let (sender, receiver) = mpsc::channel();
        let mut session = loaded_session(attributes.initial_size());
        for index in 0..50u32 {
            sender
                .send(LoopMessage::Image(
                    ImageId::new(index),
                    Ok(placeholder_framebuffer()),
                ))
                .unwrap();
        }

        pump_once(
            &mut system,
            &mut presenter,
            &receiver,
            &sender,
            &mut session,
        )
        .unwrap();

        assert_eq!(
            session.stats.relayouts, 1,
            "fifty coalesced image arrivals must cost exactly one relayout, not fifty"
        );
    }

    #[test]
    fn clicking_a_link_triggers_navigation_to_resolved_url() {
        let attributes = initial_window_attributes().unwrap();
        let mut system = HeadlessWindowSystem::new();
        system.create_window(&attributes).unwrap();

        let mut presenter = RecordingPresenter::new();
        let (sender, receiver) = mpsc::channel();
        let target_url = network::Url::parse("http://example.com/target.html").unwrap();
        let response = network::HttpResponse::new(
            network::StatusCode::OK,
            network::HeaderMap::new(),
            network::Body::from_text("<html><body>target</body></html>"),
        );
        let mut session = session_over(
            MockTransport::new().with_response(target_url, response),
            attributes.initial_size(),
        );
        session.base_url = Some(network::Url::parse("http://example.com/index.html").unwrap());
        session.dom_tree = Some(
            html::parse("<html><body><a href=\"target.html\" style=\"display: block; width: 100px; height: 50px;\">Click me</a></body></html>").unwrap(),
        );
        session.dirty = true;

        // First pump: renders document and populates session.links
        pump_once(
            &mut system,
            &mut presenter,
            &receiver,
            &sender,
            &mut session,
        )
        .unwrap();

        assert!(!session.links.is_empty(), "link target must be collected");

        // Move pointer over the link and click
        system.schedule(WindowEvent::PointerMoved {
            position: window::PhysicalPosition::new(20.0, 20.0),
        });
        system.schedule(WindowEvent::PointerButton {
            button: window::PointerButton::Left,
            pressed: true,
        });

        pump_once(
            &mut system,
            &mut presenter,
            &receiver,
            &sender,
            &mut session,
        )
        .unwrap();

        // The click should have spawned a navigation message to receiver
        let message = receiver
            .recv_timeout(std::time::Duration::from_millis(500))
            .expect("navigation message received");
        match message {
            LoopMessage::Navigation(Ok((_, target_url))) => {
                let expected = network::Url::parse("http://example.com/target.html").unwrap();
                assert_eq!(target_url, expected);
            }
            _ => panic!("expected successful navigation to target.html"),
        }
    }
}

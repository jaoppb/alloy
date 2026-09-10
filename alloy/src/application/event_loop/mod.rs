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

mod hit_test;
mod pixel;
mod session;
mod worker;

#[cfg(test)]
mod tests;

use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use graphics::SyntheticFontProvider;
use network::{HttpTransport, RequestPolicy, Url};
use window::{
    FrameView, PointerButton, Presenter, PumpStatus, WindowAttributes, WindowEvent, WindowSystem,
    WindowTitle,
};

use self::hit_test::hit_test;
use self::pixel::frame_pixels;
pub use self::session::LoopStats;
use self::session::{CachedFrame, LoopMessage, Session};
use self::worker::spawn_navigation;
use crate::application::paint::DEFAULT_FONT;
use crate::application::pipeline::{
    DEFAULT_FONT_SIZE, RenderOptions, default_runtime_font_provider, render_dom_with_links,
};
use crate::error::AlloyError;

/// How long a pump cycle sleeps when neither a window event nor a
/// background-fetch result was waiting — keeps the loop from busy-spinning a
/// CPU core while a fetch is in flight, with no async runtime and no OS
/// blocking primitive spanning both the window and the `mpsc` channel.
const IDLE_POLL: std::time::Duration = std::time::Duration::from_millis(4);

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
/// to construct itself, and that handle is not part of the object-safe
/// [`WindowSystem`] trait this function is generic over.
///
/// Generic over [`WindowSystem`]/[`Presenter`] on purpose — the real `winit`
/// backend and the headless reference (`window::HeadlessWindowSystem` /
/// `RecordingPresenter`) drive the exact same loop, which is what lets the
/// e2e golden test exercise this function directly rather than a parallel
/// test-only copy of it.
// `Arc<dyn HttpTransport>` by value is the intended public-API shape (the
// caller hands over shared ownership once, cleanly, instead of managing a
// local binding); `run_loop` only ever needs to borrow it, which is why it
// takes `&Arc<_>` instead.
#[allow(clippy::needless_pass_by_value)]
pub fn run_browser(
    url: &Url,
    transport: Arc<dyn HttpTransport>,
    policy: Arc<dyn RequestPolicy>,
    system: &mut dyn WindowSystem,
    presenter: &mut dyn Presenter,
    initial_size: window::SurfaceSize,
) -> Result<LoopStats, AlloyError> {
    let mut session = Session::new(
        initial_size,
        default_runtime_font_provider(),
        Arc::clone(&policy),
    );
    run_loop(url, &transport, system, presenter, &mut session, |_| false)
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
#[allow(clippy::needless_pass_by_value)]
pub fn run_browser_until(
    url: &Url,
    transport: Arc<dyn HttpTransport>,
    policy: Arc<dyn RequestPolicy>,
    system: &mut dyn WindowSystem,
    presenter: &mut dyn Presenter,
    initial_size: window::SurfaceSize,
    should_stop: impl FnMut(&LoopStats) -> bool,
) -> Result<LoopStats, AlloyError> {
    let font_provider =
        Arc::new(SyntheticFontProvider::new().with_size(DEFAULT_FONT, DEFAULT_FONT_SIZE));
    let mut session = Session::new(initial_size, font_provider, Arc::clone(&policy));
    run_loop(
        url,
        &transport,
        system,
        presenter,
        &mut session,
        should_stop,
    )
}

/// [`run_browser_until`], stopping as soon as the first frame has been laid
/// out and presented.
pub fn run_browser_until_first_frame(
    url: &Url,
    transport: Arc<dyn HttpTransport>,
    policy: Arc<dyn RequestPolicy>,
    system: &mut dyn WindowSystem,
    presenter: &mut dyn Presenter,
    initial_size: window::SurfaceSize,
) -> Result<LoopStats, AlloyError> {
    run_browser_until(
        url,
        transport,
        policy,
        system,
        presenter,
        initial_size,
        |stats| stats.relayouts >= 1,
    )
}

fn run_loop(
    url: &Url,
    transport: &Arc<dyn HttpTransport>,
    system: &mut dyn WindowSystem,
    presenter: &mut dyn Presenter,
    session: &mut Session,
    mut should_stop: impl FnMut(&LoopStats) -> bool,
) -> Result<LoopStats, AlloyError> {
    let (sender, receiver) = mpsc::channel();
    spawn_navigation(
        url.clone(),
        Arc::clone(transport),
        Arc::clone(&session.policy),
        sender.clone(),
    );

    loop {
        let (outcome, did_work) =
            pump_once(system, presenter, &receiver, transport, &sender, session)?;
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
pub(crate) fn pump_once(
    system: &mut dyn WindowSystem,
    presenter: &mut dyn Presenter,
    receiver: &Receiver<LoopMessage>,
    transport: &Arc<dyn HttpTransport>,
    sender: &Sender<LoopMessage>,
    session: &mut Session,
) -> Result<(PumpStatus, bool), AlloyError> {
    let mut close_requested = false;
    let mut latest_resize = None;
    let mut saw_window_event = false;
    let mut needs_repaint = false;
    let mut clicked_pos = None;
    let window_status = system.pump_events(&mut |event| {
        saw_window_event = true;
        match event {
            WindowEvent::CloseRequested => close_requested = true,
            WindowEvent::Resized(size) => latest_resize = Some(size),
            WindowEvent::RedrawRequested => needs_repaint = true,
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
                Arc::clone(transport),
                Arc::clone(&session.policy),
                sender.clone(),
            );
        } else {
            tracing::warn!(href, "failed to resolve link target against base URL");
        }
    }

    let mut saw_message = false;
    while let Ok(message) = receiver.try_recv() {
        saw_message = true;
        session.apply(message, transport, sender);
    }

    let relaid_out = session.dirty;
    if session.dirty {
        relayout_and_present(presenter, session)?;
        session.record_relayout();
        session.dirty = false;
        // Re-arm the platform redraw: on Wayland the present just made can be
        // dropped by a not-yet-configured surface, and the RedrawRequested
        // this schedules re-blits the cached frame once it is live.
        system.request_redraw();
    }
    if needs_repaint && !relaid_out {
        repaint(presenter, session)?;
    }

    let did_work = saw_window_event || saw_message;

    if close_requested || window_status == PumpStatus::Exit {
        return Ok((PumpStatus::Exit, did_work));
    }
    Ok((PumpStatus::Continue, did_work))
}

/// Rebuilds the display list from the current document, viewport and
/// subresources, presents it, and caches the pixels for a later cheap
/// [`repaint`]. The only path that bumps `stats.relayouts` — the I4 coalescing
/// proof rests on that staying true.
fn relayout_and_present(
    presenter: &mut dyn Presenter,
    session: &mut Session,
) -> Result<(), AlloyError> {
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
        Arc::clone(&session.font_provider),
    )?;
    session.links = links;
    let cached = CachedFrame {
        width: viewport.width(),
        height: viewport.height(),
        pixels: frame_pixels(&framebuffer),
    };
    let view = FrameView::new(cached.width, cached.height, &cached.pixels)
        .ok_or(AlloyError::InvalidDimensions)?;
    presenter.present(view)?;
    session.last_frame = Some(cached);
    Ok(())
}

/// Re-blits the frame [`relayout_and_present`] last produced, with no pipeline
/// work and without touching `stats.relayouts`. A no-op before the first
/// frame. Serves `RedrawRequested`: a compositor expose/occlusion, or the
/// redraw winit re-arms once a Wayland surface that dropped the first present
/// is finally configured.
fn repaint(presenter: &mut dyn Presenter, session: &Session) -> Result<(), AlloyError> {
    let Some(frame) = session.last_frame.as_ref() else {
        return Ok(());
    };
    let view = FrameView::new(frame.width, frame.height, &frame.pixels)
        .ok_or(AlloyError::InvalidDimensions)?;
    presenter.present(view)?;
    Ok(())
}

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

mod frame;
mod hit_test;
mod pixel;
mod session;
mod stats;
mod worker;

#[cfg(test)]
mod tests;

use std::thread;

use graphics::FontProvider;
use network::{HttpTransport, RequestPolicy, Url};
use window::{
    PhysicalPosition, PointerButton, Presenter, PumpStatus, SurfaceSize, WindowAttributes,
    WindowEvent, WindowSystem, WindowTitle,
};

use self::session::Session;
pub use self::stats::LoopStats;
use crate::application::browser_services::BrowserServices;
use crate::application::pipeline::RenderOptions;
use crate::application::subresource::SubresourceDiscoverer;
use crate::error::AlloyError;

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
    session.navigate(url.clone());

    loop {
        let (outcome, did_work) = pump_once(system, presenter, session)?;
        if outcome == PumpStatus::Exit || should_stop(&session.stats()) {
            return Ok(session.stats());
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
    let mut events = PumpEvents::starting_at(session.pointer_position());
    let window_status = system.pump_events(&mut |event| events.observe(&event))?;
    session.track_pointer(events.pointer_position);

    if let Some(viewport) = events.latest_resize {
        session.resize(viewport);
    }
    if let Some(position) = events.clicked_at {
        session.follow_link_at(position);
    }

    let saw_message = session.drain_messages();

    let relaid_out = session.needs_relayout();
    if relaid_out {
        session.relayout(presenter)?;
        // Re-arm the platform redraw: on Wayland the present just made can be
        // dropped by a not-yet-configured surface, and the RedrawRequested
        // this schedules re-blits the cached frame once it is live.
        system.request_redraw();
    }
    if events.needs_repaint && !relaid_out {
        session.repaint(presenter)?;
    }

    let did_work = events.saw_window_event || saw_message;
    Ok((events.status(window_status), did_work))
}

/// What one `pump_events` drain observed, folded so `pump_once` decides once
/// per cycle: only the *last* resize counts, any number of redraw requests
/// cost one repaint, and a click lands wherever the pointer was at that
/// moment in the batch.
#[derive(Default)]
struct PumpEvents {
    saw_window_event: bool,
    close_requested: bool,
    latest_resize: Option<SurfaceSize>,
    needs_repaint: bool,
    pointer_position: Option<PhysicalPosition>,
    clicked_at: Option<PhysicalPosition>,
}

impl PumpEvents {
    fn starting_at(pointer_position: Option<PhysicalPosition>) -> Self {
        Self {
            pointer_position,
            ..Self::default()
        }
    }

    fn observe(&mut self, event: &WindowEvent) {
        self.saw_window_event = true;
        match event {
            WindowEvent::CloseRequested => self.close_requested = true,
            WindowEvent::Resized(viewport) => self.latest_resize = Some(*viewport),
            WindowEvent::RedrawRequested => self.needs_repaint = true,
            WindowEvent::PointerMoved { position } => self.pointer_position = Some(*position),
            WindowEvent::PointerButton {
                button: PointerButton::Left,
                pressed: true,
            } => self.clicked_at = self.pointer_position.or(self.clicked_at),
            _ => {}
        }
    }

    fn status(&self, window_status: PumpStatus) -> PumpStatus {
        if self.close_requested || window_status == PumpStatus::Exit {
            return PumpStatus::Exit;
        }
        PumpStatus::Continue
    }
}

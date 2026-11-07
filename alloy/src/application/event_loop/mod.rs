//! The v0.5 Phase I4 event loop (`ADR-0019`): pump, fold, dispatch, present.
//!
//! A single [`WindowSystem`] owns the main thread. Every blocking fetch is
//! handed to the [`FetchExecutor`] port (`infrastructure` runs it on a worker
//! thread of its own); its result comes back over `std::sync::mpsc` as one more
//! event this loop drains. **No async runtime.** The loop only folds what it
//! observed and hands it to [`BrowserApp`], whose `EffectExecutor` is the one
//! place effects are carried out.
//!
//! Coalescing is the same mechanism for both resize and subresource arrival:
//! one pump cycle drains *every* window event and *every* queued background
//! result before deciding whether to relay out, and relays out **at most
//! once** per cycle. Ten resizes or fifty image arrivals queued between two
//! pump cycles cost one relayout, not ten or fifty.
//!
//! [`FetchExecutor`]: crate::application::ports::FetchExecutor

mod keyboard_input;
mod pump_events;

#[cfg(test)]
mod click_tests;
#[cfg(test)]
mod coalescing_tests;
#[cfg(test)]
mod damage_tests;
#[cfg(test)]
mod keyboard_tests;

use graphics::{FontProvider, RenderBackend};
use window::{Presenter, PumpStatus, WindowSystem};

use self::keyboard_input::KeyboardInput;
use self::pump_events::PumpEvents;
use crate::application::browser_app::BrowserApp;
use crate::application::loop_stats::LoopStats;
use crate::application::ports::{FetchExecutor, IdlePause};
use crate::error::AlloyError;

/// Pumps `app` until the window closes or `should_stop` answers `true`.
pub(crate) fn run_loop<S, R, F, X, W, B>(
    system: &mut S,
    presenter: &mut R,
    app: &mut BrowserApp<F, X, W, B>,
    mut should_stop: impl FnMut(&LoopStats) -> bool,
) -> Result<LoopStats, AlloyError>
where
    S: WindowSystem,
    R: Presenter,
    F: FontProvider + 'static,
    X: FetchExecutor,
    W: IdlePause,
    B: RenderBackend,
{
    loop {
        let (outcome, did_work) = pump_once(system, presenter, app)?;
        if outcome == PumpStatus::Exit || should_stop(&app.stats()) {
            return Ok(app.stats());
        }
        if !did_work {
            app.pause_idle();
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
pub(crate) fn pump_once<S, R, F, X, W, B>(
    system: &mut S,
    presenter: &mut R,
    app: &mut BrowserApp<F, X, W, B>,
) -> Result<(PumpStatus, bool), AlloyError>
where
    S: WindowSystem,
    R: Presenter,
    F: FontProvider + 'static,
    X: FetchExecutor,
    W: IdlePause,
    B: RenderBackend,
{
    let mut events = PumpEvents::starting_at(app.pointer_position());
    let window_status = system.pump_events(&mut |event| events.observe(&event))?;
    app.track_pointer(events.pointer_position());

    dispatch_window_events(app, &mut events);
    let saw_message = drain_fetches(app);

    let shown = app.render(presenter)?;
    if shown.is_shown() {
        // Re-arm the platform redraw: on Wayland the present just made can be
        // dropped by a not-yet-configured surface, and the RedrawRequested
        // this schedules re-blits the composed frame once it is live.
        system.request_redraw();
    }
    if events.needs_repaint() && !shown.is_shown() {
        app.repaint(presenter)?;
    }

    let did_work = events.saw_window_event() || saw_message;
    Ok((events.status(window_status), did_work))
}

/// Hands the folded window events to `app`: the last resize, the click, then
/// every keyboard input in arrival order.
fn dispatch_window_events<F, X, W, B>(app: &mut BrowserApp<F, X, W, B>, events: &mut PumpEvents)
where
    F: FontProvider + 'static,
    X: FetchExecutor,
    W: IdlePause,
    B: RenderBackend,
{
    if let Some(viewport) = events.latest_resize() {
        app.resize(viewport);
    }
    if let Some(position) = events.clicked_at() {
        app.handle_click(position);
    }
    for input in events.take_keyboard() {
        match input {
            KeyboardInput::Key(key) => app.handle_key(&key),
            KeyboardInput::Committed(text) => app.handle_text_committed(text),
        }
    }
}

/// Applies every fetch result waiting now; tells the caller whether there were
/// any.
fn drain_fetches<F, X, W, B>(app: &mut BrowserApp<F, X, W, B>) -> bool
where
    F: FontProvider + 'static,
    X: FetchExecutor,
    W: IdlePause,
    B: RenderBackend,
{
    let mut saw_message = false;
    while let Some(result) = app.next_ready_fetch() {
        saw_message = true;
        app.complete_fetch(result);
    }
    saw_message
}

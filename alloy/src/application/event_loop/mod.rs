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

mod dispatch;
mod keyboard_input;
mod pump_events;

use graphics::{FontProvider, RenderBackend};
use window::{Presenter, PumpStatus, WindowSystem};

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
        let cycle = pump_once(system, presenter, app)?;
        if cycle.status == PumpStatus::Exit || should_stop(&app.stats()) {
            return Ok(app.stats());
        }
        if !cycle.did_work {
            app.pause_idle();
        }
    }
}

/// What one pump cycle ended with: whether to keep going, and whether it
/// observed *anything* (a window event, a background message). The caller idle-sleeps
/// on a cycle that did no work, purely to avoid busy-spinning a CPU core; it plays no
/// part in the coalescing invariant.
pub(crate) struct Cycle {
    pub(crate) status: PumpStatus,
    pub(crate) did_work: bool,
}

/// One iteration: drain every window event, drain every background-fetch
/// result waiting right now, and — only if something actually changed —
/// relay out and present exactly once ("drain everything currently available,
/// then relay out at most once").
pub(crate) fn pump_once<S, R, F, X, W, B>(
    system: &mut S,
    presenter: &mut R,
    app: &mut BrowserApp<F, X, W, B>,
) -> Result<Cycle, AlloyError>
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

    app.dispatch_window_events(&mut events);
    app.apply_ready_fetches(&mut events);

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

    Ok(Cycle {
        status: events.status(window_status),
        did_work: events.did_work(),
    })
}

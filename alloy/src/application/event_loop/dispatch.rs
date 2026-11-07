//! What the loop hands to [`BrowserApp`] once a drain is folded.

use graphics::{FontProvider, RenderBackend};

use super::keyboard_input::KeyboardInput;
use super::pump_events::PumpEvents;
use crate::application::browser_app::BrowserApp;
use crate::application::ports::{FetchExecutor, IdlePause};

impl<F, X, W, B> BrowserApp<F, X, W, B>
where
    F: FontProvider + 'static,
    X: FetchExecutor,
    W: IdlePause,
    B: RenderBackend,
{
    /// Hands the folded window events over: the last resize, the click, then
    /// every keyboard input in arrival order.
    pub(super) fn dispatch_window_events(&mut self, events: &mut PumpEvents) {
        if let Some(viewport) = events.latest_resize() {
            self.resize(viewport);
        }
        if let Some(position) = events.clicked_at() {
            self.handle_click(position);
        }
        for input in events.take_keyboard() {
            match input {
                KeyboardInput::Key(key) => self.handle_key(&key),
                KeyboardInput::Committed(text) => self.handle_text_committed(text),
            }
        }
    }

    /// Applies every fetch result waiting now, noting on `events` that the
    /// cycle saw any.
    pub(super) fn apply_ready_fetches(&mut self, events: &mut PumpEvents) {
        while let Some(result) = self.next_ready_fetch() {
            events.observe_fetch();
            self.complete_fetch(result);
        }
    }
}

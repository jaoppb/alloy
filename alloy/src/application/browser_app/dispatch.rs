//! How [`BrowserApp`] applies the window input one loop cycle folded
//! (`ADR-0019`, #115): the loop pumps, folds and dispatches here.

use graphics::{FontProvider, RenderBackend};

use super::BrowserApp;
use crate::application::event_loop::{KeyboardInput, WindowInput};
use crate::application::ports::{FetchExecutor, IdlePause};

impl<F, X, W, B> BrowserApp<F, X, W, B>
where
    F: FontProvider + 'static,
    X: FetchExecutor,
    W: IdlePause,
    B: RenderBackend,
{
    /// Applies the folded window input: the last resize, the click, then
    /// every keyboard input in arrival order.
    pub(crate) fn apply_input(&mut self, input: WindowInput) {
        if let Some(viewport) = input.latest_resize() {
            self.resize(viewport);
        }
        if let Some(position) = input.clicked_at() {
            self.handle_click(position);
        }
        for keyboard in input.into_keyboard() {
            match keyboard {
                KeyboardInput::Key(key) => self.handle_key(&key),
                KeyboardInput::Committed(text) => self.handle_text_committed(text),
            }
        }
    }
}

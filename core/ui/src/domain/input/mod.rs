//! What the chrome is told: resolved input, the commands a keymap binds, and
//! the effects handling it asks for (`ADR-0027`).

mod browser_command;
mod focus_target;
mod key_chord;
mod keymap;
mod keymap_binding;
mod keymap_error;
mod ui_effect;
mod ui_effects;
mod ui_input;

#[cfg(test)]
mod tests;

pub use browser_command::BrowserCommand;
pub use focus_target::FocusTarget;
pub use key_chord::{ChordKey, KeyChord};
pub use keymap::Keymap;
pub use keymap_binding::{ActionName, ChordText, ChordWord, KeymapBinding};
pub use keymap_error::KeymapError;
pub use ui_effect::UiEffect;
pub use ui_effects::UiEffects;
pub use ui_input::UiInput;

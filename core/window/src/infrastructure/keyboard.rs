//! Stateful keyboard translation: `winit` key input → [`KeyEvent`].
//!
//! The adapter's one piece of memory (`ADR-0027`). A `winit` key event does
//! not carry the modifiers held — they arrive as separate `ModifiersChanged`
//! events — and `winit`'s `ModifiersState` has no `AltGr` bit, so
//! [`KeyboardState`] folds both into the [`Modifiers`] stamped on every
//! [`KeyEvent`]. The adapter reports; it never decides whether a press is a
//! shortcut or typing — that belongs to the consumer's keymap.
//!
//! Command–Query Separation: [`KeyboardState::observe`] and
//! [`KeyboardState::observe_key`] update the state, [`KeyboardState::translate`]
//! is a pure query over it. The order — observe, then translate — is owned in
//! one place, `event_map::map_key_input`, so no caller can get it wrong.
//!
//! `winit::event::KeyEvent` has no public constructor, so the translation
//! takes a [`KeyInput`] — the fields of it this adapter reads — which a test
//! can build.

use winit::event::{ElementState, KeyEvent as WinitKeyEvent, WindowEvent};
use winit::keyboard::{Key, ModifiersState, NamedKey, PhysicalKey};

use crate::domain::function_key::FunctionKey;
use crate::domain::key_event::{KeyEvent, KeyState};
use crate::domain::logical_key::LogicalKey;
use crate::domain::modifiers::Modifiers;
use crate::domain::text::InputText;
use crate::infrastructure::modifier_keys::{HeldModifierKeys, ModifierKey};
use crate::infrastructure::physical_key::map_physical_key;

/// Whether a key event came from the user or from `winit` itself.
///
/// On X11 and Windows `winit` synthesises a press for every key already held
/// when the window gains focus and a release for every key still down when it
/// loses it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    Real,
    Synthetic,
}

impl From<bool> for Origin {
    fn from(is_synthetic: bool) -> Self {
        if is_synthetic {
            Self::Synthetic
        } else {
            Self::Real
        }
    }
}

/// The fields of a `winit` key event this adapter reads.
#[derive(Clone, Debug)]
pub struct KeyInput {
    physical_key: PhysicalKey,
    logical_key: Key,
    text: Option<String>,
    state: KeyState,
    origin: Origin,
}

impl KeyInput {
    /// A real (non-synthetic) key event; see [`Self::synthetic`].
    #[must_use]
    pub fn new(
        physical_key: PhysicalKey,
        logical_key: Key,
        text: Option<&str>,
        state: KeyState,
    ) -> Self {
        Self {
            physical_key,
            logical_key,
            text: text.map(str::to_owned),
            state,
            origin: Origin::Real,
        }
    }

    /// This input, marked as one `winit` synthesised.
    #[must_use]
    pub const fn synthetic(mut self) -> Self {
        self.origin = Origin::Synthetic;
        self
    }

    #[must_use]
    pub const fn is_synthetic(&self) -> bool {
        matches!(self.origin, Origin::Synthetic)
    }

    /// Copies out the fields of `event`. `text` is `winit`'s
    /// `KeyEvent::text`, **not** `text_with_all_modifiers`: it ignores Ctrl on
    /// xkb, which the keymap compensates for by reading the modifiers
    /// (`ADR-0027`).
    #[must_use]
    pub fn from_winit(event: &WinitKeyEvent, origin: Origin) -> Self {
        let input = Self::new(
            event.physical_key,
            event.logical_key.clone(),
            event.text.as_deref(),
            map_key_state(event.state, Repetition::from(event.repeat)),
        );
        match origin {
            Origin::Real => input,
            Origin::Synthetic => input.synthetic(),
        }
    }
}

/// Whether `winit` flagged a press as the platform's key-repeat firing again.
/// A two-case enum so [`map_key_state`] takes no bare flag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Repetition {
    First,
    AutoRepeat,
}

impl From<bool> for Repetition {
    fn from(repeat: bool) -> Self {
        if repeat {
            Self::AutoRepeat
        } else {
            Self::First
        }
    }
}

/// A held key is `Pressed` once, then `Repeated`, then `Released` — never
/// repeated `Pressed`.
#[must_use]
pub const fn map_key_state(state: ElementState, repetition: Repetition) -> KeyState {
    match (state, repetition) {
        (ElementState::Pressed, Repetition::First) => KeyState::Pressed,
        (ElementState::Pressed, Repetition::AutoRepeat) => KeyState::Repeated,
        (ElementState::Released, _) => KeyState::Released,
    }
}

/// Folds `winit`'s modifier reports and the modifier keys' own presses into
/// the [`Modifiers`] stamped on each [`KeyEvent`].
#[derive(Clone, Debug, Default)]
pub struct KeyboardState {
    reported: ModifiersState,
    keys: HeldModifierKeys,
}

impl KeyboardState {
    /// Updates the state from a window-level event: the modifiers `winit`
    /// reports (`ModifiersChanged`) or the window losing focus. Every other
    /// event leaves it alone; key presses go through [`Self::observe_key`].
    pub fn observe(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::ModifiersChanged(modifiers) => self.reported = modifiers.state(),
            WindowEvent::Focused(false) => self.focus_lost(),
            _ => {}
        }
    }

    /// Updates the state from one key input, synthetic or not: which physical
    /// modifier keys are down is true either way. `winit` has no `AltGr`
    /// modifier bit, so for `AltGr` this is the only signal (Linux, and
    /// Windows layouts with an `AltGr` key, where `winit` also clears Ctrl and
    /// Alt from its state while it is held: read from the 0.30.13 source, not
    /// run on Windows). macOS reports Option as Alt, so there `alt()` is set
    /// instead and the consumer owns the rule (`ADR-0027`).
    pub(super) fn observe_key(&mut self, input: &KeyInput) {
        let Some(modifier) = ModifierKey::of(&input.logical_key) else {
            return;
        };
        let key = map_physical_key(input.physical_key);
        if input.state.is_released() {
            self.keys.release(modifier, key);
            return;
        }
        self.keys.press(modifier, key);
    }

    /// The window lost focus: a modifier released elsewhere may never be
    /// reported, so forget them all rather than stamp them forever. `winit`
    /// re-sends the held modifiers when focus returns.
    fn focus_lost(&mut self) {
        *self = Self::default();
    }

    /// The [`KeyEvent`] for `input`, stamped with the modifiers held now.
    #[must_use]
    pub(super) fn translate(&self, input: &KeyInput) -> KeyEvent {
        let event = KeyEvent::new(
            map_logical_key(&input.logical_key),
            map_physical_key(input.physical_key),
            input.state,
        )
        .with_modifiers(self.modifiers(ModifierKey::of(&input.logical_key)));
        match key_text(input) {
            Some(text) => event.with_text(text),
            None => event,
        }
    }

    /// The modifiers held. For the modifier key `own` that this very event is,
    /// our own record decides (its press reports it held, its release reports
    /// it released unless the twin is still down) so the answer does not
    /// depend on whether `winit` sent `ModifiersChanged` first.
    fn modifiers(&self, own: Option<ModifierKey>) -> Modifiers {
        ModifierKey::ALL
            .into_iter()
            .filter(|modifier| self.is_held(*modifier, own))
            .fold(Modifiers::NONE, |held, modifier| modifier.add_to(held))
    }

    fn is_held(&self, modifier: ModifierKey, own: Option<ModifierKey>) -> bool {
        if own == Some(modifier) || modifier == ModifierKey::AltGraph {
            return self.keys.is_down(modifier);
        }
        modifier.is_reported_in(self.reported)
    }
}

/// A press's text, if it has any printable: a release inserts nothing, and a
/// control character (`\r` for Enter, `\x08` for Backspace) is a command the
/// logical key already names.
fn key_text(input: &KeyInput) -> Option<InputText> {
    if input.state.is_released() {
        return None;
    }
    input.text.as_deref().and_then(InputText::new)
}

fn map_logical_key(key: &Key) -> LogicalKey {
    match key {
        Key::Named(named) => map_named_key(*named),
        Key::Character(characters) => InputText::new(characters.as_str())
            .map_or(LogicalKey::Unidentified, LogicalKey::Character),
        Key::Dead(_) => LogicalKey::Dead,
        Key::Unidentified(_) => LogicalKey::Unidentified,
    }
}

/// `winit::keyboard::NamedKey` is `#[non_exhaustive]` and ~200 strong: the keys
/// this port names are listed, every other becomes [`LogicalKey::Other`].
const fn map_named_key(key: NamedKey) -> LogicalKey {
    match key {
        NamedKey::Enter => LogicalKey::Enter,
        NamedKey::Tab => LogicalKey::Tab,
        NamedKey::Space => LogicalKey::Space,
        NamedKey::Backspace => LogicalKey::Backspace,
        NamedKey::Delete => LogicalKey::Delete,
        NamedKey::Escape => LogicalKey::Escape,
        NamedKey::Insert => LogicalKey::Insert,
        NamedKey::ArrowUp => LogicalKey::ArrowUp,
        NamedKey::ArrowDown => LogicalKey::ArrowDown,
        NamedKey::ArrowLeft => LogicalKey::ArrowLeft,
        NamedKey::ArrowRight => LogicalKey::ArrowRight,
        NamedKey::Home => LogicalKey::Home,
        NamedKey::End => LogicalKey::End,
        NamedKey::PageUp => LogicalKey::PageUp,
        NamedKey::PageDown => LogicalKey::PageDown,
        NamedKey::Shift => LogicalKey::Shift,
        NamedKey::Control => LogicalKey::Control,
        NamedKey::Alt => LogicalKey::Alt,
        NamedKey::AltGraph => LogicalKey::AltGraph,
        NamedKey::Super => LogicalKey::Super,
        NamedKey::CapsLock => LogicalKey::CapsLock,
        other => map_function_key(other),
    }
}

const fn map_function_key(key: NamedKey) -> LogicalKey {
    let number = match key {
        NamedKey::F1 => 1,
        NamedKey::F2 => 2,
        NamedKey::F3 => 3,
        NamedKey::F4 => 4,
        NamedKey::F5 => 5,
        NamedKey::F6 => 6,
        NamedKey::F7 => 7,
        NamedKey::F8 => 8,
        NamedKey::F9 => 9,
        NamedKey::F10 => 10,
        NamedKey::F11 => 11,
        NamedKey::F12 => 12,
        _ => return LogicalKey::Other,
    };
    match FunctionKey::new(number) {
        Some(function_key) => LogicalKey::Function(function_key),
        None => LogicalKey::Other,
    }
}

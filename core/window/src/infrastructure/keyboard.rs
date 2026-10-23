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
use crate::infrastructure::physical_key::map_physical_key;

/// The fields of a `winit` key event this adapter reads.
#[derive(Clone, Debug)]
pub struct KeyInput {
    physical_key: PhysicalKey,
    logical_key: Key,
    text: Option<String>,
    state: KeyState,
}

impl KeyInput {
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
        }
    }

    /// Copies out the fields of `event`. `text` is `winit`'s
    /// `KeyEvent::text`, **not** `text_with_all_modifiers`: it ignores Ctrl on
    /// xkb, which the keymap compensates for by reading the modifiers
    /// (`ADR-0027`).
    #[must_use]
    pub fn from_winit(event: &WinitKeyEvent) -> Self {
        Self::new(
            event.physical_key,
            event.logical_key.clone(),
            event.text.as_deref(),
            map_key_state(event.state, Repetition::from(event.repeat)),
        )
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

/// Folds `winit`'s modifier reports and `AltGr` key presses into the
/// [`Modifiers`] stamped on each [`KeyEvent`].
#[derive(Clone, Copy, Debug, Default)]
pub struct KeyboardState {
    held: ModifiersState,
    alt_graph: bool,
}

impl KeyboardState {
    /// Updates the state from a window-level event: the modifiers `winit`
    /// reports (`ModifiersChanged`) or the window losing focus. Every other
    /// event leaves it alone; key presses go through [`Self::observe_key`].
    pub fn observe(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::ModifiersChanged(modifiers) => self.held = modifiers.state(),
            WindowEvent::Focused(false) => self.focus_lost(),
            _ => {}
        }
    }

    /// Updates the state from one key input. `winit` has no `AltGr` modifier
    /// bit, so the key's own press and release are the only signal (Linux).
    /// macOS reports Option as Alt, so there `alt()` is set instead and the
    /// consumer owns the rule (`ADR-0027`).
    pub(super) const fn observe_key(&mut self, input: &KeyInput) {
        if matches!(input.logical_key, Key::Named(NamedKey::AltGraph)) {
            self.alt_graph = !input.state.is_released();
        }
    }

    /// The window lost focus: a modifier released elsewhere may never be
    /// reported, so forget them all rather than stamp them forever. `winit`
    /// re-sends the held modifiers when focus returns.
    fn focus_lost(&mut self) {
        *self = Self::default();
    }

    /// The [`KeyEvent`] for `input`, stamped with the modifiers held now.
    #[must_use]
    pub(super) fn translate(self, input: &KeyInput) -> KeyEvent {
        let event = KeyEvent::new(
            map_logical_key(&input.logical_key),
            map_physical_key(input.physical_key),
            input.state,
        )
        .with_modifiers(self.modifiers());
        match key_text(input) {
            Some(text) => event.with_text(text),
            None => event,
        }
    }

    fn modifiers(self) -> Modifiers {
        let held = held_modifiers(self.held);
        if self.alt_graph {
            return held.with_alt_graph();
        }
        held
    }
}

/// The modifiers `winit` reports as held — everything but `AltGr`.
fn held_modifiers(held: ModifiersState) -> Modifiers {
    let mut modifiers = Modifiers::NONE;
    if held.shift_key() {
        modifiers = modifiers.with_shift();
    }
    if held.control_key() {
        modifiers = modifiers.with_control();
    }
    if held.alt_key() {
        modifiers = modifiers.with_alt();
    }
    if held.super_key() {
        modifiers = modifiers.with_super_key();
    }
    modifiers
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

//! Stateful keyboard translation: `winit` key input → [`KeyEvent`].
//!
//! The adapter's one piece of memory (`ADR-0027`). A `winit` key event does
//! not carry the modifiers held — they arrive as separate `ModifiersChanged`
//! events — and `winit`'s `ModifiersState` has no `AltGr` bit, so
//! [`KeyboardState`] folds both into the [`Modifiers`] stamped on every
//! [`KeyEvent`]. The adapter reports; it never decides whether a press is a
//! shortcut or typing — that belongs to the consumer's keymap.
//!
//! `winit::event::KeyEvent` has no public constructor, so the translation
//! takes a [`KeyInput`] — the five fields of it this adapter reads — which a
//! test can build.

use winit::event::{ElementState, KeyEvent as WinitKeyEvent};
use winit::keyboard::{Key, ModifiersState, NamedKey, PhysicalKey};

use crate::domain::key_event::{KeyEvent, KeyState};
use crate::domain::logical_key::LogicalKey;
use crate::domain::modifiers::Modifiers;
use crate::domain::text::InputText;
use crate::infrastructure::event_map::map_physical_key;

/// The fields of a `winit` key event this adapter reads.
#[derive(Clone, Debug)]
pub struct KeyInput {
    physical_key: PhysicalKey,
    logical_key: Key,
    text: Option<String>,
    state: ElementState,
    repeat: bool,
}

impl KeyInput {
    #[must_use]
    pub fn new(
        physical_key: PhysicalKey,
        logical_key: Key,
        text: Option<&str>,
        state: ElementState,
        repeat: bool,
    ) -> Self {
        Self {
            physical_key,
            logical_key,
            text: text.map(str::to_owned),
            state,
            repeat,
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
            event.state,
            event.repeat,
        )
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
    /// Records the modifiers `winit` says are held now (`ModifiersChanged`).
    pub const fn modifiers_changed(&mut self, held: ModifiersState) {
        self.held = held;
    }

    /// The window lost focus: an `AltGr` release that happens elsewhere never
    /// arrives, so forget it rather than stamp it forever.
    pub const fn focus_lost(&mut self) {
        self.alt_graph = false;
    }

    /// Translates one key input into a [`KeyEvent`] stamped with the
    /// modifiers held at that moment.
    pub fn translate(&mut self, input: &KeyInput) -> KeyEvent {
        self.track_alt_graph(input);
        let logical = map_logical_key(&input.logical_key);
        let state = map_key_state(input.state, input.repeat);
        let event = KeyEvent::new(logical, map_physical_key(input.physical_key), state)
            .with_modifiers(self.stamp());
        match key_text(input, state) {
            Some(text) => event.with_text(text),
            None => event,
        }
    }

    /// `winit` has no `AltGr` modifier bit, so the key's own press and release
    /// are the only signal (Linux). macOS reports Option as Alt, so there `alt()` is set instead
    /// and the consumer owns the rule (`ADR-0027`).
    const fn track_alt_graph(&mut self, input: &KeyInput) {
        if matches!(input.logical_key, Key::Named(NamedKey::AltGraph)) {
            self.alt_graph = !matches!(input.state, ElementState::Released);
        }
    }

    fn stamp(self) -> Modifiers {
        let mut modifiers = Modifiers::NONE;
        if self.held.shift_key() {
            modifiers = modifiers.with_shift();
        }
        if self.held.control_key() {
            modifiers = modifiers.with_control();
        }
        if self.held.alt_key() {
            modifiers = modifiers.with_alt();
        }
        if self.held.super_key() {
            modifiers = modifiers.with_super_key();
        }
        if self.alt_graph {
            modifiers = modifiers.with_alt_graph();
        }
        modifiers
    }
}

/// A press's text, if it has any printable: a release inserts nothing, and a
/// control character (`\r` for Enter, `\x08` for Backspace) is a command the
/// logical key already names.
fn key_text(input: &KeyInput, state: KeyState) -> Option<InputText> {
    if state.is_released() {
        return None;
    }
    input.text.as_deref().and_then(InputText::new)
}

const fn map_key_state(state: ElementState, repeat: bool) -> KeyState {
    match (state, repeat) {
        (ElementState::Pressed, false) => KeyState::Pressed,
        (ElementState::Pressed, true) => KeyState::Repeated,
        (ElementState::Released, _) => KeyState::Released,
    }
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
#[allow(clippy::too_many_lines)]
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
        NamedKey::F1 => LogicalKey::F1,
        NamedKey::F2 => LogicalKey::F2,
        NamedKey::F3 => LogicalKey::F3,
        NamedKey::F4 => LogicalKey::F4,
        NamedKey::F5 => LogicalKey::F5,
        NamedKey::F6 => LogicalKey::F6,
        NamedKey::F7 => LogicalKey::F7,
        NamedKey::F8 => LogicalKey::F8,
        NamedKey::F9 => LogicalKey::F9,
        NamedKey::F10 => LogicalKey::F10,
        NamedKey::F11 => LogicalKey::F11,
        NamedKey::F12 => LogicalKey::F12,
        NamedKey::Shift => LogicalKey::Shift,
        NamedKey::Control => LogicalKey::Control,
        NamedKey::Alt => LogicalKey::Alt,
        NamedKey::AltGraph => LogicalKey::AltGraph,
        NamedKey::Super => LogicalKey::Super,
        NamedKey::CapsLock => LogicalKey::CapsLock,
        _ => LogicalKey::Other,
    }
}

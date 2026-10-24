//! Which physical modifier keys are down, per modifier.
//!
//! `winit` reports the *state* of Shift, Ctrl, Alt and Super through
//! `ModifiersChanged`, and the order of that event relative to the modifier
//! key's own key event differs by platform (X11 stamps the state from just
//! before the event). [`HeldModifierKeys`] is the adapter's own record, built
//! from the key events themselves, so a modifier key's own press and release
//! report the same thing on every platform (the DOM convention, `ADR-0027`) —
//! including when the left and right twin of a modifier are both down.

use std::collections::HashSet;

use winit::keyboard::{Key, ModifiersState, NamedKey};

use crate::domain::key::KeyCode;
use crate::domain::modifiers::Modifiers;

/// A modifier a key press can be.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum ModifierKey {
    Shift,
    Control,
    Alt,
    AltGraph,
    Super,
}

impl ModifierKey {
    pub(super) const ALL: [Self; 5] = [
        Self::Shift,
        Self::Control,
        Self::Alt,
        Self::AltGraph,
        Self::Super,
    ];

    /// The modifier `key` is, if it is a modifier key.
    pub(super) const fn of(key: &Key) -> Option<Self> {
        match key {
            Key::Named(NamedKey::Shift) => Some(Self::Shift),
            Key::Named(NamedKey::Control) => Some(Self::Control),
            Key::Named(NamedKey::Alt) => Some(Self::Alt),
            Key::Named(NamedKey::AltGraph) => Some(Self::AltGraph),
            Key::Named(NamedKey::Super) => Some(Self::Super),
            _ => None,
        }
    }

    /// Whether `reported` — `winit`'s modifier state — has this modifier held.
    /// `winit` has no `AltGr` bit, so that one is never reported.
    pub(super) fn is_reported_in(self, reported: ModifiersState) -> bool {
        match self {
            Self::Shift => reported.shift_key(),
            Self::Control => reported.control_key(),
            Self::Alt => reported.alt_key(),
            Self::Super => reported.super_key(),
            Self::AltGraph => false,
        }
    }

    pub(super) const fn add_to(self, modifiers: Modifiers) -> Modifiers {
        match self {
            Self::Shift => modifiers.with_shift(),
            Self::Control => modifiers.with_control(),
            Self::Alt => modifiers.with_alt(),
            Self::AltGraph => modifiers.with_alt_graph(),
            Self::Super => modifiers.with_super_key(),
        }
    }
}

/// The physical keys currently down, by modifier.
#[derive(Clone, Debug, Default)]
pub(super) struct HeldModifierKeys(HashSet<(ModifierKey, KeyCode)>);

impl HeldModifierKeys {
    pub(super) fn press(&mut self, modifier: ModifierKey, key: KeyCode) {
        self.0.insert((modifier, key));
    }

    pub(super) fn release(&mut self, modifier: ModifierKey, key: KeyCode) {
        self.0.remove(&(modifier, key));
    }

    /// Whether any physical key of `modifier` is down.
    pub(super) fn is_down(&self, modifier: ModifierKey) -> bool {
        self.0.iter().any(|(held, _)| *held == modifier)
    }
}

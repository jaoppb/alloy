//! Physical keyboard modifier tracking for the event loop.

use window::KeyCode;

/// Tracks physical keyboard modifier key states using a compact bitmask.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ModifiersState(u8);

impl ModifiersState {
    const CTRL: u8 = 1 << 0;
    const ALT: u8 = 1 << 1;
    const SUPER_KEY: u8 = 1 << 2;
    const SHIFT: u8 = 1 << 3;

    /// Creates a new resting modifier state with no active modifiers.
    #[must_use]
    pub const fn new() -> Self {
        Self(0)
    }

    /// Whether a Control modifier is held.
    #[must_use]
    pub const fn is_ctrl_pressed(self) -> bool {
        (self.0 & Self::CTRL) != 0
    }

    /// Whether an Alt modifier is held.
    #[must_use]
    pub const fn is_alt_pressed(self) -> bool {
        (self.0 & Self::ALT) != 0
    }

    /// Whether a Super (Command/Windows) modifier is held.
    #[must_use]
    pub const fn is_super_pressed(self) -> bool {
        (self.0 & Self::SUPER_KEY) != 0
    }

    /// Whether a Shift modifier is held.
    #[must_use]
    pub const fn is_shift_pressed(self) -> bool {
        (self.0 & Self::SHIFT) != 0
    }

    /// Updates modifier key states according to key code and pressed state.
    pub const fn update(&mut self, code: KeyCode, pressed: bool) {
        match code {
            KeyCode::CONTROL_LEFT | KeyCode::CONTROL_RIGHT => self.set_mask(Self::CTRL, pressed),
            KeyCode::ALT_LEFT | KeyCode::ALT_RIGHT => self.set_mask(Self::ALT, pressed),
            KeyCode::SUPER_LEFT | KeyCode::SUPER_RIGHT => self.set_mask(Self::SUPER_KEY, pressed),
            KeyCode::SHIFT_LEFT | KeyCode::SHIFT_RIGHT => self.set_mask(Self::SHIFT, pressed),
            _ => {}
        }
    }

    const fn set_mask(&mut self, mask: u8, enable: bool) {
        if enable {
            self.0 |= mask;
            return;
        }
        self.0 &= !mask;
    }

    /// Checks if a key code combined with active modifiers forms the omnibox focus shortcut.
    #[must_use]
    pub const fn is_focus_omnibox_shortcut(self, code: KeyCode) -> bool {
        let is_ctrl_l = (self.is_ctrl_pressed() || self.is_super_pressed())
            && code.raw() == KeyCode::KEY_L.raw();
        let is_alt_d = self.is_alt_pressed() && code.raw() == KeyCode::KEY_D.raw();
        is_ctrl_l || is_alt_d
    }
}

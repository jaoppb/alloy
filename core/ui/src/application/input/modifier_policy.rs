//! [`ModifierPolicy`] — the per-OS facts of the router, chosen once.

use platform::HostOs;
use window::Modifiers;

/// What Alt means for the key being routed. Option composes text on macOS, but
/// Option+Backspace is still a word-wise edit, which is out of scope (epic #110
/// decision 24).
#[derive(Clone, Copy, Debug)]
pub(super) enum AltMeaning {
    Command,
    Composer,
}

/// How the host reports modifiers (`ADR-0027`). Built from [`HostOs`] at
/// construction so routing itself never branches on the OS.
#[derive(Clone, Copy, Debug)]
pub(super) struct ModifierPolicy {
    /// Windows reports its left Ctrl+Alt, which it also accepts as `AltGr`, as
    /// plain Ctrl and Alt with no `alt_graph` bit.
    ctrl_alt_is_alt_gr: bool,
    /// macOS reports Option as Alt, and Option+letter composes text.
    alt_composes_text: bool,
}

impl ModifierPolicy {
    pub(super) const fn for_host(host: HostOs) -> Self {
        Self {
            ctrl_alt_is_alt_gr: matches!(host, HostOs::Windows),
            alt_composes_text: matches!(host, HostOs::MacOs),
        }
    }

    /// Exactly Ctrl+Alt on a host where that is `AltGr`. Shift never blocks.
    pub(super) const fn is_alt_gr(self, modifiers: Modifiers) -> bool {
        self.ctrl_alt_is_alt_gr && modifiers.control() && modifiers.alt() && !modifiers.super_key()
    }

    /// Whether Ctrl, Super or a command-meaning Alt is held without `AltGr`.
    pub(super) const fn holds_command_modifier(
        self,
        modifiers: Modifiers,
        alt: AltMeaning,
    ) -> bool {
        if modifiers.alt_graph() {
            return false;
        }
        let alt_is_command =
            modifiers.alt() && !(self.alt_composes_text && matches!(alt, AltMeaning::Composer));
        modifiers.control() || modifiers.super_key() || alt_is_command
    }
}

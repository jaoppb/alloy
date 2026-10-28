//! [`BrowserCommand`] — the closed set of actions a key chord can be bound to.

use core::fmt;
use core::str::FromStr;

use super::keymap_binding::ActionName;
use super::keymap_error::KeymapError;

/// An action a [`Keymap`](super::Keymap) binds a chord to.
///
/// A closed enum parsed from action names (epic #110 decision 15): a script
/// (#50) can only name commands Rust defines, and an unknown name is a typed
/// error at keymap load, never a surprise at key-press time.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BrowserCommand {
    /// Move focus to the address bar and select its text.
    FocusOmnibox,
}

impl BrowserCommand {
    /// The action name scripts use for this command.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::FocusOmnibox => "focus_omnibox",
        }
    }
}

impl FromStr for BrowserCommand {
    type Err = KeymapError;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        match name {
            "focus_omnibox" => Ok(Self::FocusOmnibox),
            unknown => Err(KeymapError::UnknownAction(ActionName::from(unknown))),
        }
    }
}

impl fmt::Display for BrowserCommand {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name())
    }
}

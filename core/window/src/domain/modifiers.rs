//! [`Modifiers`] — the modifier keys held at the moment of a key press.

/// A compact set of held modifier keys.
///
/// Stamped onto every [`KeyEvent`](crate::domain::key_event::KeyEvent) by the
/// adapter from the backend's own modifier reports, so a consumer never
/// reconstructs state from raw key events (which goes wrong whenever a
/// modifier was pressed while the window was unfocused, issue #112).
///
/// [`Self::alt_graph`] is distinct from [`Self::alt`]: it is set only where
/// the platform reports `AltGr` as its own key (`ADR-0027`). Windows reports
/// `AltGr` as Ctrl+Alt, so there it never appears and the consumer owns the
/// fallback rule.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Modifiers(u8);

impl Modifiers {
    const SHIFT: u8 = 1 << 0;
    const CONTROL: u8 = 1 << 1;
    const ALT: u8 = 1 << 2;
    const ALT_GRAPH: u8 = 1 << 3;
    const SUPER_KEY: u8 = 1 << 4;

    /// No modifier held.
    pub const NONE: Self = Self(0);

    #[must_use]
    pub const fn with_shift(self) -> Self {
        Self(self.0 | Self::SHIFT)
    }

    #[must_use]
    pub const fn with_control(self) -> Self {
        Self(self.0 | Self::CONTROL)
    }

    #[must_use]
    pub const fn with_alt(self) -> Self {
        Self(self.0 | Self::ALT)
    }

    #[must_use]
    pub const fn with_alt_graph(self) -> Self {
        Self(self.0 | Self::ALT_GRAPH)
    }

    /// The Windows / Command / Meta key.
    #[must_use]
    pub const fn with_super_key(self) -> Self {
        Self(self.0 | Self::SUPER_KEY)
    }

    #[must_use]
    pub const fn shift(self) -> bool {
        self.0 & Self::SHIFT != 0
    }

    #[must_use]
    pub const fn control(self) -> bool {
        self.0 & Self::CONTROL != 0
    }

    #[must_use]
    pub const fn alt(self) -> bool {
        self.0 & Self::ALT != 0
    }

    #[must_use]
    pub const fn alt_graph(self) -> bool {
        self.0 & Self::ALT_GRAPH != 0
    }

    #[must_use]
    pub const fn super_key(self) -> bool {
        self.0 & Self::SUPER_KEY != 0
    }

    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

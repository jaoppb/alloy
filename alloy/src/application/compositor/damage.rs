//! The three damage signals and their per-cycle coalescing (`ADR-0019`
//! addendum, #108).

/// What a change invalidated.
///
/// A fourth case, `RedrawRequested`, is not damage: nothing is re-rastered,
/// the composed frame is presented again ([`super::Compositor::repaint`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Damage {
    /// The chrome strip changed (omnibox text, focus, selection). Re-rasters
    /// the chrome only and never counts as a relayout.
    Chrome,
    /// The document changed (navigation, stylesheet, image). Relayouts the
    /// content — the only damage that does.
    Content,
    /// The viewport changed size: both layers are re-sized and re-rastered.
    Viewport,
}

/// Damage recorded since the last frame, however many signals produced it.
///
/// N signals in one pump cycle are one flag each, which is what keeps the
/// coalescing invariant: at most one chrome raster and one content relayout
/// per cycle.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PendingDamage {
    chrome: bool,
    content: bool,
}

impl PendingDamage {
    pub const fn record(&mut self, damage: Damage) {
        match damage {
            Damage::Chrome => self.chrome = true,
            Damage::Content => self.content = true,
            Damage::Viewport => {
                self.chrome = true;
                self.content = true;
            }
        }
    }

    /// Hands over what accumulated and starts a clean slate.
    #[must_use]
    pub const fn take(&mut self) -> Self {
        let taken = *self;
        *self = Self {
            chrome: false,
            content: false,
        };
        taken
    }

    #[must_use]
    pub const fn needs_chrome(self) -> bool {
        self.chrome
    }

    #[must_use]
    pub const fn needs_content(self) -> bool {
        self.content
    }

    #[must_use]
    pub const fn is_empty(self) -> bool {
        !self.chrome && !self.content
    }
}

#[cfg(test)]
mod tests {
    use super::{Damage, PendingDamage};

    #[test]
    fn chrome_damage_asks_for_the_chrome_only() {
        let mut pending = PendingDamage::default();
        pending.record(Damage::Chrome);

        assert!(pending.needs_chrome());
        assert!(!pending.needs_content());
    }

    #[test]
    fn viewport_damage_asks_for_both_layers() {
        let mut pending = PendingDamage::default();
        pending.record(Damage::Viewport);

        assert!(pending.needs_chrome());
        assert!(pending.needs_content());
    }

    #[test]
    fn taking_hands_over_the_damage_and_leaves_none() {
        let mut pending = PendingDamage::default();
        pending.record(Damage::Content);
        pending.record(Damage::Content);

        let taken = pending.take();

        assert!(taken.needs_content());
        assert!(pending.is_empty());
    }
}

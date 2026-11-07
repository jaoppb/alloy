//! Counters a run of the event loop accumulates.

use browser::ContextStats;

/// A monotonically growing tally of frames.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Relayouts(usize);

impl Relayouts {
    const fn increment(&mut self) {
        self.0 = self.0.saturating_add(1);
    }
}

/// What a run of the loop did so far: the loop's own relayouts plus what the
/// browsing context accepted.
///
/// Instrumented for I4's coalescing proofs (resize, subresource bursts) and
/// for a caller (or a test) that needs to wait for a specific piece of
/// background work to land before looking at the presented frame. Read through
/// accessors; only the loop records.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LoopStats {
    relayouts: Relayouts,
    browsing: ContextStats,
}

impl LoopStats {
    /// Counts one relayout that was laid out and presented. A bare
    /// `RedrawRequested` repaint re-blits the cached frame and is not one — the
    /// I4 coalescing proof is about relayouts only.
    pub(crate) const fn record_relayout(&mut self) {
        self.relayouts.increment();
    }

    /// This tally with the browsing context's counters read at `browsing`.
    #[must_use]
    pub(crate) const fn observing(self, browsing: ContextStats) -> Self {
        Self { browsing, ..self }
    }

    /// How many times this run re-laid-out and presented a frame.
    #[must_use]
    pub const fn relayouts(&self) -> usize {
        self.relayouts.0
    }

    /// How many navigations have successfully parsed into the document tree.
    #[must_use]
    pub const fn navigations(&self) -> usize {
        self.browsing.navigations()
    }

    /// How many navigations failed and fell back to the error card.
    #[must_use]
    pub const fn navigation_errors(&self) -> usize {
        self.browsing.navigation_errors()
    }

    /// How many external `<link rel=stylesheet>` sheets were absorbed into the cascade.
    #[must_use]
    pub const fn stylesheets_loaded(&self) -> usize {
        self.browsing.stylesheets_loaded()
    }

    /// How many `<img>` fetches have decoded and replaced their placeholder.
    #[must_use]
    pub const fn images_loaded(&self) -> usize {
        self.browsing.images_loaded()
    }
}

#[cfg(test)]
mod tests {
    use browser::ContextStats;

    use super::LoopStats;

    #[test]
    fn a_fresh_tally_is_all_zero() {
        let stats = LoopStats::default();

        assert_eq!(stats.relayouts(), 0);
        assert_eq!(stats.navigations(), 0);
    }

    #[test]
    fn recording_a_relayout_counts_one_and_keeps_the_browsing_counters() {
        let mut stats = LoopStats::default().observing(ContextStats::default());

        stats.record_relayout();
        stats.record_relayout();

        assert_eq!(stats.relayouts(), 2);
        assert_eq!(stats, stats.observing(ContextStats::default()));
    }
}

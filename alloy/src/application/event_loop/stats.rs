//! Counters a run of the event loop accumulates.

/// What a run of the loop did so far.
///
/// Instrumented for I4's coalescing proofs (resize, subresource bursts) and
/// for a caller (or a test) that needs to wait for a specific piece of
/// background work to land before looking at the presented frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LoopStats {
    /// How many times this run actually re-laid-out and presented a frame. A
    /// bare `RedrawRequested` repaint re-blits the cached frame and does not
    /// count here — the I4 coalescing proof is about relayouts only.
    pub relayouts: usize,
    /// How many navigations have successfully parsed into the document tree.
    pub navigations: usize,
    /// How many navigations failed and fell back to the error card.
    pub navigation_errors: usize,
    /// How many external `<link rel=stylesheet>` sheets were absorbed into the
    /// cascade.
    pub stylesheets_loaded: usize,
    /// How many `<img>` fetches have decoded and replaced their placeholder.
    pub images_loaded: usize,
}

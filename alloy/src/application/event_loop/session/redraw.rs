//! [`Redraw`] — whether handling input asked the platform for a redraw.

/// The outcome of executing input on the session, folded by the loop so one
/// pump cycle issues at most one platform redraw request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Redraw {
    Skip,
    Request,
}

impl Redraw {
    /// Requested if either side requested.
    #[must_use]
    pub const fn or(self, other: Self) -> Self {
        match (self, other) {
            (Self::Skip, Self::Skip) => Self::Skip,
            _ => Self::Request,
        }
    }

    #[must_use]
    pub const fn is_requested(self) -> bool {
        matches!(self, Self::Request)
    }
}

//! [`IntrinsicSize`] — what a box's size owes to a resource (v0.5 B4, plan
//! `:499-501`; `Natural` since issue #83).
//!
//! A replaced element (`<img>` and friends) has no size of its own until the
//! resource behind it is decoded. Layout still has to produce a box for it, so
//! the box says out loud that its size is provisional instead of pretending a
//! zero is a measurement.
//!
//! The marker lands **before** the `I3` freeze on purpose: Phase X (`<img>`)
//! reads [`crate::LayoutBox::intrinsic_size`] to know which boxes to re-lay-out
//! once a decoded image arrives, and adding the field afterwards would need a
//! migration note in `PRD-007`.
//!
//! Only layout knows: it asks the [`crate::NaturalSizes`] port about a
//! replaced element's resource and reports, per [`crate::LayoutBox`], whether
//! the box took a natural size, still waits on one, or is final. Every other
//! box — and every line fragment, which never stands for a resource — is
//! `Resolved`.

use crate::domain::replaced::NaturalSize;

/// Whether a box's own size is final.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum IntrinsicSize {
    /// The box's size is final — it came from the cascade, from its content, or
    /// from its formatting context.
    #[default]
    Resolved,
    /// The box belongs to a replaced element whose resource has not been loaded,
    /// and neither axis was pinned by the cascade. Its geometry will change once
    /// the resource arrives (Phase X).
    Pending,
    /// The box belongs to a replaced element whose resource is loaded: this is
    /// the resource's natural size, whatever size the box was given.
    Natural(NaturalSize),
}

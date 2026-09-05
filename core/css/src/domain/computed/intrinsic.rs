//! [`IntrinsicSize`] — the "this box is still waiting on a resource" marker
//! (v0.5 B4, plan `:499-501`).
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
//! It is decided by the **tree builder**, not by a
//! [`crate::CascadeResolver`]: only [`crate::StyledTree`]'s construction pass
//! still sees the element's tag, and making it a resolver's job would change
//! the closure signature every adapter passes.

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
}

impl IntrinsicSize {
    /// Whether this box still depends on an unloaded resource.
    #[must_use]
    pub const fn is_pending(self) -> bool {
        matches!(self, Self::Pending)
    }
}

/// The elements whose size comes from a resource rather than from CSS
/// (HTML Living Standard, "replaced elements").
///
/// Deliberately a short, closed list: an element outside it is laid out from
/// its own content, which is the correct answer for every non-replaced tag.
const REPLACED_TAGS: [&str; 7] = ["img", "video", "canvas", "iframe", "object", "embed", "svg"];

/// The marker an element with this tag is born with.
#[must_use]
pub(crate) fn for_tag(tag: Option<&str>) -> IntrinsicSize {
    let Some(name) = tag else {
        return IntrinsicSize::Resolved;
    };
    if REPLACED_TAGS.contains(&name) {
        return IntrinsicSize::Pending;
    }
    IntrinsicSize::Resolved
}

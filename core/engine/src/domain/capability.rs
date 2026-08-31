//! Capability model (PRD-003 §3, ADR-0004).
//!
//! [`Capability`] is the bitflag set transcribed verbatim from PRD-003:35-47.
//! [`CapabilitySet`] is the newtype an [`crate::ExecutionContext`] is built with
//! and carries for its whole life — Object Calisthenics forbids passing the raw
//! flags around (ADR-0010 rule 3). Enforcement *per native binding* is F6/v0.2;
//! v0.1 only stores the set and exposes [`CapabilitySet::require`] for adapters
//! to call once bindings are guarded.

use crate::domain::error::EngineError;

bitflags::bitflags! {
    /// A single privilege a script context may hold. Exactly the nine flags of
    /// PRD-003:37-46.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct Capability: u32 {
        const DOM_READ         = 1 << 0;
        const DOM_MUTATE       = 1 << 1;
        const NETWORK_FETCH    = 1 << 2;
        const NETWORK_LISTEN   = 1 << 3;
        const FS_READ_SCRIPTS  = 1 << 4;
        const FS_WRITE_CACHE   = 1 << 5;
        const GRAPHICS_DRAW    = 1 << 6;
        const WINDOW_MANAGE    = 1 << 7;
        const DEVTOOLS_INSPECT = 1 << 8;
    }
}

/// The immutable grant an [`crate::ExecutionContext`] is created with
/// (PRD-002:40). Once built it never widens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CapabilitySet(Capability);

impl CapabilitySet {
    /// A context that may do nothing privileged — the right default for a bare
    /// evaluation with no host bindings.
    #[must_use]
    pub const fn empty() -> Self {
        Self(Capability::empty())
    }

    /// Grant exactly `capabilities`.
    #[must_use]
    pub const fn new(capabilities: Capability) -> Self {
        Self(capabilities)
    }

    /// Is `needed` (which may itself be several flags) fully granted?
    #[must_use]
    pub const fn contains(&self, needed: Capability) -> bool {
        self.0.contains(needed)
    }

    /// The granted flags, for adapters that must configure their engine from
    /// them. Read-only — there is no setter.
    #[must_use]
    pub const fn granted(&self) -> Capability {
        self.0
    }

    /// `Ok(())` when `needed` is granted, else [`EngineError::PermissionDenied`]
    /// naming the first missing flag. This is the single check every guarded
    /// binding will call in F6.
    pub const fn require(&self, needed: Capability) -> Result<(), EngineError> {
        if self.0.contains(needed) {
            return Ok(());
        }
        Err(EngineError::permission_denied(needed.difference(self.0)))
    }
}

impl Default for CapabilitySet {
    fn default() -> Self {
        Self::empty()
    }
}

/// The subsystem capability profiles of PRD-003:53-58. These are *mechanism*
/// defaults from the specification, not user policy — a script may be given
/// less, never more, than its subsystem's profile.
pub mod profiles {
    use super::{Capability, CapabilitySet};

    /// DOM parser / HTML engine: read and mutate the tree, nothing else.
    #[must_use]
    pub fn dom_parser() -> CapabilitySet {
        CapabilitySet::new(Capability::DOM_READ | Capability::DOM_MUTATE)
    }

    /// CSS cascade / style engine: read the tree and draw; no mutation.
    #[must_use]
    pub fn css_style() -> CapabilitySet {
        CapabilitySet::new(Capability::DOM_READ | Capability::GRAPHICS_DRAW)
    }

    /// Network interceptor: fetch and write the cache.
    #[must_use]
    pub fn network_interceptor() -> CapabilitySet {
        CapabilitySet::new(Capability::NETWORK_FETCH | Capability::FS_WRITE_CACHE)
    }

    /// UI & window manager: manage windows, draw, and read the tree.
    #[must_use]
    pub fn ui_window() -> CapabilitySet {
        CapabilitySet::new(
            Capability::WINDOW_MANAGE | Capability::GRAPHICS_DRAW | Capability::DOM_READ,
        )
    }
}

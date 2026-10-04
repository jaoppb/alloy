//! The ports a host OS adapter implements (`ADR-0010` §1, `ADR-0022`).

use crate::domain::font::{FontCandidates, FontDirectories, GenericFamily};

/// Where the host OS installs fonts.
///
/// Best-effort by contract: an answer names paths that *may* exist. Reading
/// them, and treating a missing one as "unavailable", is the caller's job.
pub trait FontLocator {
    /// The directories to scan for installed fonts, most preferred first.
    fn font_directories(&self) -> FontDirectories;

    /// Well-known install paths of a face for `family`, most preferred first.
    fn generic_family_candidates(&self, family: GenericFamily) -> FontCandidates;
}

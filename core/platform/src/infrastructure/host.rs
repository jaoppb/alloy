//! [`HostPlatform`] — the [`FontLocator`] for the OS this binary was built for.

use crate::application::FontLocator;
use crate::domain::font::{FontCandidates, FontDirectories, GenericFamily};
use crate::domain::host_os::HostOs;
use crate::infrastructure::os;

/// The operating system this binary was built for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HostPlatform;

impl HostPlatform {
    /// The host platform.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// The operating system family this binary was built for.
    #[must_use]
    pub const fn os(&self) -> HostOs {
        os::host_os()
    }
}

impl FontLocator for HostPlatform {
    fn font_directories(&self) -> FontDirectories {
        os::font_directories()
    }

    fn generic_family_candidates(&self, family: GenericFamily) -> FontCandidates {
        FontCandidates::new(os::generic_family_paths(family))
    }
}

//! [`GenericFamily`] and [`FontCatalog`] — best-effort resolution of the CSS
//! generic font families to a real, installed font file.
//!
//! The per-OS knowledge (install paths, font directories, `HOME` / `WINDIR`)
//! lives in `core/platform` behind its `FontLocator` port (`ADR-0022`); this
//! module only adapts that answer to what [`SystemFontProvider`] and the
//! named-family index consume. A machine with none of the candidates installed
//! is a legitimate [`crate::domain::error::GraphicsError::FontUnavailable`],
//! not a panic — golden and conformance tests never depend on this path, only
//! [`crate::infrastructure::font::SyntheticFontProvider`] does.
//!
//! [`SystemFontProvider`]: crate::infrastructure::font::SystemFontProvider

use platform::{FontCandidates, FontDirectories, FontLocator, HostPlatform};

pub use platform::GenericFamily;

/// Where to look for fonts on the OS this binary was built for.
pub struct FontCatalog;

impl FontCatalog {
    /// The directories to scan for installed fonts, most preferred first. A
    /// missing directory is skipped by the scanner, never an error — the same
    /// best-effort discipline as [`Self::candidate_paths`].
    #[must_use]
    pub fn system_font_dirs() -> FontDirectories {
        HostPlatform::new().font_directories()
    }

    /// Candidate absolute file paths for `family`, most preferred first.
    #[must_use]
    pub fn candidate_paths(family: GenericFamily) -> FontCandidates {
        HostPlatform::new().generic_family_candidates(family)
    }
}

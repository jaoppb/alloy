//! Any target without a known font layout: no directories, no candidates.
//! Fonts then resolve to "unavailable" and the caller falls back, exactly as
//! on a font-less Linux CI image.

use crate::domain::font::{FontDirectories, GenericFamily};

pub(super) const fn font_directories() -> FontDirectories {
    FontDirectories::new()
}

pub(super) const fn generic_family_paths(family: GenericFamily) -> &'static [&'static str] {
    let _ = family;
    &[]
}

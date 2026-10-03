//! macOS font install locations.

use std::path::PathBuf;

use crate::domain::font::{FontDirectories, GenericFamily};
use crate::infrastructure::environment::home_directory;

pub(super) fn font_directories() -> FontDirectories {
    let mut directories: FontDirectories = [
        PathBuf::from("/System/Library/Fonts"),
        PathBuf::from("/Library/Fonts"),
    ]
    .into_iter()
    .collect();
    if let Some(home) = home_directory() {
        directories.push(home.join("Library/Fonts"));
    }
    directories
}

pub(super) const fn generic_family_paths(family: GenericFamily) -> &'static [&'static str] {
    match family {
        GenericFamily::SansSerif => &[
            "/System/Library/Fonts/Helvetica.ttc",
            "/System/Library/Fonts/SFNS.ttf",
        ],
        GenericFamily::Serif => &["/System/Library/Fonts/Supplemental/Times New Roman.ttf"],
        GenericFamily::Monospace => &["/System/Library/Fonts/Menlo.ttc"],
    }
}

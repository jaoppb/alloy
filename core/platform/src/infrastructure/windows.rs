//! Windows font install locations.

use std::path::PathBuf;

use crate::domain::font::{FontDirectories, GenericFamily};

pub(super) fn font_directories() -> FontDirectories {
    let windows_directory =
        std::env::var_os("WINDIR").map_or_else(|| PathBuf::from("C:\\Windows"), PathBuf::from);
    let mut directories: FontDirectories =
        std::iter::once(windows_directory.join("Fonts")).collect();
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        directories.push(PathBuf::from(local).join("Microsoft\\Windows\\Fonts"));
    }
    directories
}

pub(super) const fn generic_family_paths(family: GenericFamily) -> &'static [&'static str] {
    match family {
        GenericFamily::SansSerif => &[
            "C:\\Windows\\Fonts\\segoeui.ttf",
            "C:\\Windows\\Fonts\\arial.ttf",
        ],
        GenericFamily::Serif => &["C:\\Windows\\Fonts\\times.ttf"],
        GenericFamily::Monospace => &["C:\\Windows\\Fonts\\consola.ttf"],
    }
}

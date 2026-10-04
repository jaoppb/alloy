//! Linux font install locations (Debian/Ubuntu, Arch, Fedora/GNOME layouts).

use std::path::PathBuf;

use crate::domain::font::{FontDirectories, GenericFamily};
use crate::infrastructure::environment::home_directory;

pub(super) fn font_directories() -> FontDirectories {
    let mut directories: FontDirectories = [
        PathBuf::from("/usr/share/fonts"),
        PathBuf::from("/usr/local/share/fonts"),
    ]
    .into_iter()
    .collect();
    if let Some(home) = home_directory() {
        directories.push(home.join(".local/share/fonts"));
        directories.push(home.join(".fonts"));
    }
    directories
}

pub(super) const fn generic_family_paths(family: GenericFamily) -> &'static [&'static str] {
    match family {
        GenericFamily::SansSerif => &[
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/truetype/ubuntu/Ubuntu-R.ttf",
            "/usr/share/fonts/TTF/DejaVuSans.ttf",
            "/usr/share/fonts/Adwaita/AdwaitaSans-Regular.ttf",
            "/usr/share/fonts/noto/NotoSans-Regular.ttf",
        ],
        GenericFamily::Serif => &[
            "/usr/share/fonts/truetype/dejavu/DejaVuSerif.ttf",
            "/usr/share/fonts/TTF/DejaVuSerif.ttf",
            "/usr/share/fonts/noto/NotoSerif-Regular.ttf",
        ],
        GenericFamily::Monospace => &[
            "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
            "/usr/share/fonts/TTF/DejaVuSansMono.ttf",
            "/usr/share/fonts/Adwaita/AdwaitaMono-Regular.ttf",
        ],
    }
}

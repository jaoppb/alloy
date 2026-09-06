//! [`GenericFamily`] and [`FontCatalog`] — best-effort per-OS resolution of the
//! CSS generic font families to a real, installed font file.
//!
//! This is deliberately not exhaustive: it names a handful of well-known
//! install paths per OS and per family, and [`SystemFontProvider`] tries them
//! in order until one parses. A machine with none of them installed is a
//! legitimate [`crate::domain::error::GraphicsError::FontUnavailable`], not a
//! panic — golden and conformance tests never depend on this path, only
//! [`crate::infrastructure::font::SyntheticFontProvider`] does.
//!
//! [`SystemFontProvider`]: crate::infrastructure::font::SystemFontProvider

use std::path::PathBuf;

/// The CSS generic font families `font-family` can resolve to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum GenericFamily {
    SansSerif,
    Serif,
    Monospace,
}

/// Where to look for a `GenericFamily`, per operating system.
pub struct FontCatalog;

impl FontCatalog {
    /// The directories to scan for installed fonts, most preferred first, for
    /// the operating system this binary was built for. A missing directory is
    /// skipped by the scanner, never an error — the same best-effort discipline
    /// as [`Self::candidate_paths`].
    #[must_use]
    pub fn system_font_dirs() -> Vec<PathBuf> {
        #[cfg(target_os = "linux")]
        {
            linux_font_dirs()
        }
        #[cfg(target_os = "macos")]
        {
            macos_font_dirs()
        }
        #[cfg(target_os = "windows")]
        {
            windows_font_dirs()
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            Vec::new()
        }
    }

    /// Candidate absolute file paths for `family`, most preferred first, for
    /// the operating system this binary was built for.
    #[must_use]
    pub const fn candidate_paths(family: GenericFamily) -> &'static [&'static str] {
        #[cfg(target_os = "linux")]
        {
            linux_paths(family)
        }
        #[cfg(target_os = "macos")]
        {
            macos_paths(family)
        }
        #[cfg(target_os = "windows")]
        {
            windows_paths(family)
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            let _ = family;
            &[]
        }
    }
}

#[cfg(target_os = "linux")]
const fn linux_paths(family: GenericFamily) -> &'static [&'static str] {
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

#[cfg(target_os = "macos")]
const fn macos_paths(family: GenericFamily) -> &'static [&'static str] {
    match family {
        GenericFamily::SansSerif => &[
            "/System/Library/Fonts/Helvetica.ttc",
            "/System/Library/Fonts/SFNS.ttf",
        ],
        GenericFamily::Serif => &["/System/Library/Fonts/Supplemental/Times New Roman.ttf"],
        GenericFamily::Monospace => &["/System/Library/Fonts/Menlo.ttc"],
    }
}

#[cfg(target_os = "windows")]
const fn windows_paths(family: GenericFamily) -> &'static [&'static str] {
    match family {
        GenericFamily::SansSerif => &[
            "C:\\Windows\\Fonts\\segoeui.ttf",
            "C:\\Windows\\Fonts\\arial.ttf",
        ],
        GenericFamily::Serif => &["C:\\Windows\\Fonts\\times.ttf"],
        GenericFamily::Monospace => &["C:\\Windows\\Fonts\\consola.ttf"],
    }
}

/// The user's home directory, if the environment names one. Only the two Unix
/// targets consult it; Windows uses `%WINDIR%` / `%LOCALAPPDATA%` instead.
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
}

#[cfg(target_os = "linux")]
fn linux_font_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![
        PathBuf::from("/usr/share/fonts"),
        PathBuf::from("/usr/local/share/fonts"),
    ];
    if let Some(home) = home_dir() {
        dirs.push(home.join(".local/share/fonts"));
        dirs.push(home.join(".fonts"));
    }
    dirs
}

#[cfg(target_os = "macos")]
fn macos_font_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![
        PathBuf::from("/System/Library/Fonts"),
        PathBuf::from("/Library/Fonts"),
    ];
    if let Some(home) = home_dir() {
        dirs.push(home.join("Library/Fonts"));
    }
    dirs
}

#[cfg(target_os = "windows")]
fn windows_font_dirs() -> Vec<PathBuf> {
    let windir = std::env::var_os("WINDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("C:\\Windows"));
    let mut dirs = vec![windir.join("Fonts")];
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        dirs.push(PathBuf::from(local).join("Microsoft\\Windows\\Fonts"));
    }
    dirs
}

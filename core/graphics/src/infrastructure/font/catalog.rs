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

//! The font vocabulary the host OS answers in: which generic family is
//! wanted, and where installed font files can be found.

use std::path::{Path, PathBuf};

/// The CSS generic font families a host OS maps to an installed face.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum GenericFamily {
    /// `sans-serif`.
    SansSerif,
    /// `serif`.
    Serif,
    /// `monospace`.
    Monospace,
}

/// The directories to scan for installed fonts, most preferred first.
///
/// A directory that does not exist is the scanner's to skip, never an error:
/// a minimal CI image is a legitimate host with no fonts at all.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FontDirectories {
    directories: Vec<PathBuf>,
}

impl FontDirectories {
    /// No directories yet.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            directories: Vec::new(),
        }
    }

    /// Appends `directory`, least preferred so far.
    pub fn push(&mut self, directory: PathBuf) {
        self.directories.push(directory);
    }

    /// The directories, most preferred first.
    pub fn iter(&self) -> impl Iterator<Item = &Path> + '_ {
        self.directories.iter().map(PathBuf::as_path)
    }

    /// Whether this host names no font directory at all.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.directories.is_empty()
    }
}

impl FromIterator<PathBuf> for FontDirectories {
    fn from_iter<I: IntoIterator<Item = PathBuf>>(iter: I) -> Self {
        Self {
            directories: iter.into_iter().collect(),
        }
    }
}

/// Well-known install paths of one generic family's face, most preferred
/// first. Not exhaustive by design — the caller tries each in turn and treats
/// "none parsed" as an unavailable font, not a failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FontCandidates {
    paths: &'static [&'static str],
}

impl FontCandidates {
    /// The candidates `paths`, most preferred first.
    #[must_use]
    pub const fn new(paths: &'static [&'static str]) -> Self {
        Self { paths }
    }

    /// The candidate file paths, most preferred first.
    pub fn iter(&self) -> impl Iterator<Item = &'static Path> + use<> {
        self.paths.iter().map(Path::new)
    }

    /// Whether this host knows no install path for the family.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }
}

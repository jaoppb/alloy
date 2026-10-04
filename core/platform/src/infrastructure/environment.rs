//! Environment lookups shared by the Unix-like targets.

use std::path::PathBuf;

/// The user's home directory, if the environment names one. Windows uses
/// `%WINDIR%` / `%LOCALAPPDATA%` instead, so only Unix targets compile this.
pub(super) fn home_directory() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
}

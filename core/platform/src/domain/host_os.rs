//! [`HostOs`] — the closed vocabulary of operating systems whose behaviour a
//! subsystem needs to tell apart.

/// The operating system family this binary was built for.
///
/// Deliberately coarse: only the distinctions a consumer actually branches on
/// get a variant (`ui`'s input router: Windows' left Ctrl+Alt is `AltGr`,
/// macOS' Option composes text). Everything else is [`Self::Other`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum HostOs {
    /// Microsoft Windows.
    Windows,
    /// Apple macOS.
    MacOs,
    /// Linux and every other target.
    Other,
}

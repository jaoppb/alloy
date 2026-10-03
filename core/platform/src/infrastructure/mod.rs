//! [`HostPlatform`] and the per-OS knowledge it dispatches to.
//!
//! The `#[cfg(target_os)]` selection of the `os` module below is the single
//! place the workspace branches on the build target (`ADR-0022`); every
//! per-OS module exposes the same two functions, so [`HostPlatform`] itself
//! is target-independent.

mod host;

#[cfg(any(target_os = "linux", target_os = "macos"))]
mod environment;
#[cfg(target_os = "linux")]
#[path = "linux.rs"]
mod os;
#[cfg(target_os = "macos")]
#[path = "macos.rs"]
mod os;
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
#[path = "unsupported.rs"]
mod os;
#[cfg(target_os = "windows")]
#[path = "windows.rs"]
mod os;

pub use host::HostPlatform;

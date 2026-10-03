//! # `platform` — the host operating system, behind one port
//!
//! Every `#[cfg(target_os = …)]` branch and every OS environment lookup
//! (`HOME`, `WINDIR`, `LOCALAPPDATA`) the browser needs lives here, so the
//! subsystem crates ask a question ("where are fonts installed?") instead of
//! encoding per-OS answers themselves (`ADR-0022`).
//!
//! This crate names no other workspace crate and has no dependencies; any
//! crate may depend on it.
//!
//! ## Layout (`ADR-0010` §1)
//!
//! - [`domain`] — zero-I/O value objects: [`GenericFamily`],
//!   [`FontDirectories`], [`FontCandidates`].
//! - [`application`] — the [`FontLocator`] port.
//! - [`infrastructure`] — [`HostPlatform`], the adapter for the OS this binary
//!   was built for. Its per-OS modules are the only `target_os` dispatch in
//!   the workspace.

#![forbid(unsafe_code)]

pub mod application;
pub mod domain;
pub mod infrastructure;

pub use application::FontLocator;
pub use domain::font::{FontCandidates, FontDirectories, GenericFamily};
pub use infrastructure::HostPlatform;

//! The adapters behind
//! [`WindowSystem`](crate::application::ports::WindowSystem) and
//! [`Presenter`](crate::application::ports::Presenter) (`ADR-0010` §1).
//!
//! [`headless`] is always compiled — it is the `no-window` build's only
//! adapter, and the one CI exercises. [`winit_system`],
//! [`softbuffer_presenter`], [`event_map`], [`keyboard`] and [`physical_key`]
//! compile only under the `winit-system` feature (the default): they are the
//! only modules in this crate that name `winit` or `softbuffer` (`ADR-0011`
//! item 2).

pub mod headless;

#[cfg(feature = "winit-system")]
pub mod event_map;
#[cfg(feature = "winit-system")]
pub mod keyboard;
#[cfg(feature = "winit-system")]
mod modifier_keys;
#[cfg(feature = "winit-system")]
pub mod physical_key;
#[cfg(feature = "winit-system")]
pub mod softbuffer_presenter;
#[cfg(feature = "winit-system")]
pub mod winit_system;

pub use headless::{HeadlessWindowSystem, RecordingPresenter};

#[cfg(feature = "winit-system")]
pub use softbuffer_presenter::SoftbufferPresenter;
#[cfg(feature = "winit-system")]
pub use winit_system::WinitSystem;

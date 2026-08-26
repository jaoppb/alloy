#![forbid(unsafe_code)]

//! # Core Engine (`core/engine`)
//!
//! Abstract scripting runtime and execution isolate traits for Alloy.
//!
//! Decouples domain crates from concrete interpreter implementations (PRD-002, ADR-0002).
//! Owns the canonical dynamic value types, capability-based security gates, and domain errors.

pub mod application;
pub mod domain;
pub mod infrastructure;

// Public re-exports of the ubiquitous scripting language
pub use application::conversion::{FromEngineValue, IntoEngineValue};
pub use application::hot_reload::{AtomicScriptSlot, HotReloadCoordinator, ScriptWatcher};
pub use application::ports::{ExecutionContext, FileWatchPort, NativeFn, RuntimeEngine};
pub use application::sandbox::{TrappedExecutor, guarded_native_fn};
pub use domain::capability::{Capability, CapabilitySet, SubsystemProfile};
pub use domain::error::EngineError;
pub use domain::hot_reload::{DebounceDuration, HotReloadStatus, ReloadEvent};
pub use domain::identifier::Identifier;
pub use domain::value::EngineValue;
pub use infrastructure::mock::{MockContext, MockEngine};
pub use infrastructure::notify_watcher::NotifyFileWatcher;

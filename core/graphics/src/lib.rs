#![forbid(unsafe_code)]

//! # Core Graphics (`core/graphics`)
//!
//! 2D declarative graphics pipeline, display list commands, and multi-tier rasterization backends.
//! Part of the aggregate rendering pipeline for Alloy (PRD-005, ADR-0009, ADR-0010).

pub mod application;
pub mod domain;
pub mod infrastructure;

pub use application::cpu_backend::SoftwareCpuBackend;
pub use application::factory::GraphicsBackendFactory;
pub use domain::backend::RenderBackend;
pub use domain::command::RenderCommand;
pub use domain::display_list::DisplayList;
pub use domain::error::GraphicsError;
pub use domain::geometry::{Point, Position, Rect, Size};
pub use domain::layout::LayoutEngine;
pub use infrastructure::rhai_bridge::{ScriptDisplayListContainer, register_graphics_bindings};

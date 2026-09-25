//! Adapters implementing this crate's ports (`ADR-0010` §1).

pub mod cascade;
pub mod font;
pub mod golden;
mod opengl;
pub mod png;
#[cfg(feature = "software-backend")]
pub mod software;
mod vulkan;

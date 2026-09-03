//! Adapters implementing this crate's ports (`ADR-0010` §1).

pub mod cascade;
mod opengl;
#[cfg(feature = "software-backend")]
pub mod software;
mod vulkan;

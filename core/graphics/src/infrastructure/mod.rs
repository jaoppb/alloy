//! Adapters implementing this crate's ports (`ADR-0010` §1).

pub mod cascade;
pub mod font;
pub mod golden;
pub mod image;
mod opengl;
pub mod png;
pub mod png_decode;
#[cfg(feature = "software-backend")]
pub mod software;
mod vulkan;

//! Zero-I/O value objects, the event vocabulary and the one typed error of
//! this port (`ADR-0010` §1).
//!
//! Nothing here names `winit`, `softbuffer` or `graphics` — see the crate doc
//! `## Layout`.

pub mod attributes;
pub mod cursor;
pub mod error;
pub mod event;
pub mod frame;
pub mod function_key;
pub mod key;
pub mod key_event;
pub mod logical_key;
pub mod modifiers;
pub mod surface;
pub mod text;

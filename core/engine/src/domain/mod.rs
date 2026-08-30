//! Innermost layer: immutable value objects and typed errors. Zero I/O, zero
//! framework dependencies (ADR-0010 §1). `bitflags` is the one permitted crate
//! and only inside [`capability`].

pub mod capability;
pub mod error;
pub mod limits;
pub mod source;
pub mod value;

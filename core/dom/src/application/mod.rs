//! Middle layer (`ADR-0010` §1): pure read-only services over the
//! [`crate::domain`] aggregate. [`serialize`] is the only one in v0.2.

pub mod serialize;

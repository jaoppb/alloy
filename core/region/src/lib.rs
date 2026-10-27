//! # `region` — what `core/ui` and `core/browser` agree on about a link
//!
//! `core/ui` hit-tests clickable areas and `core/browser` owns what they point
//! at, but neither may depend on the other (`ADR-0026`). This leaf crate holds
//! the one vocabulary they share: a [`ContentRegion`] — where a link is and the
//! [`RegionId`] naming it — and the [`ContextId`] and [`LayoutEpoch`] that id is
//! scoped by. It never carries an `href` (#110 decision 12).
//!
//! The constructors are public because both crates and their tests need to
//! build values, so "only `core/browser` mints" is a convention, not something
//! the compiler checks. What is enforced: only `core/browser` *resolves* an id,
//! and it refuses one from another context or an earlier layout (#110
//! decision 13). An id forged for the current layout would resolve to a real
//! link of that layout — no link outside it.

#![forbid(unsafe_code)]

pub mod domain;

pub use domain::{ContentRegion, ContextId, LayoutEpoch, RegionId};

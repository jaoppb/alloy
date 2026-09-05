//! The layout adapters.
//!
//! B0 shipped one — [`BlockLayout`], a minimal block-flow placeholder. B4
//! replaces it with the real box model, inline formatting context and
//! Flexbox, spread across the private modules below: [`box_model`]
//! (box-sizing and the three edges), [`margin_collapse`] (CSS 2.1 §8.3.1),
//! [`context`] (what a formatting context reads and answers), [`fragment`]
//! (a laid-out box before it knows where it is), [`inline`] (line boxes and
//! text) and [`flex`] (CSS Flexbox L1).

mod box_model;
mod context;
mod flex;
mod fragment;
mod inline;
mod margin_collapse;

pub mod block;

pub use block::BlockLayout;

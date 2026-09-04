//! The layout adapters. B0 ships one — [`BlockLayout`], a minimal block-flow
//! placeholder. B4 replaces it with the real box model, inline formatting
//! context and Flexbox.

pub mod block;

pub use block::BlockLayout;

//! Omnibox editing state machine and text buffer.

mod buffer;
mod state;

#[cfg(test)]
mod tests;

pub use buffer::{CursorPosition, OmniboxBuffer};
pub use state::OmniboxState;

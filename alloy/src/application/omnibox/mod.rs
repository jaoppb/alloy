//! Omnibox editing state machine and text buffer.

mod buffer;
mod gap_buffer;
mod state;

#[cfg(test)]
mod tests;

pub use buffer::{CursorPosition, OmniboxBuffer};
pub use gap_buffer::GapBuffer;
pub use state::OmniboxState;

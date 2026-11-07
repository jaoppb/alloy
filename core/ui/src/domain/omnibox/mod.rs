//! The omnibox's text, cursor and the destination its input resolves to.

mod buffer;
mod classifier;
mod edit_key;
mod gap_buffer;
mod state;

pub use buffer::{CursorPosition, OmniboxBuffer};
pub use classifier::{OmniboxClassifier, OmniboxDestination, SearchTemplate};
pub use edit_key::EditKey;
pub use gap_buffer::GapBuffer;
pub use state::OmniboxState;

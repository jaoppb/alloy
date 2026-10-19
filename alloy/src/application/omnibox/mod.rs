//! Omnibox editing state machine and text buffer.

mod buffer;
mod gap_buffer;
pub mod paint;
mod state;

#[cfg(test)]
mod tests;

pub use crate::application::theme;
pub use buffer::{CursorPosition, OmniboxBuffer};
pub use gap_buffer::GapBuffer;
pub use paint::{
    CARET_COLOR, CARET_WIDTH, CHROME_BACKGROUND_COLOR, ChromeHeight, DEFAULT_CHROME_HEIGHT,
    FOCUS_BORDER_COLOR, FOCUS_BORDER_WIDTH, FOCUS_INNER_RADIUS, INPUT_BACKGROUND_COLOR,
    INPUT_BOX_HEIGHT, INPUT_BOX_MARGIN_X, INPUT_BOX_RADIUS, OmniboxPainter, OmniboxTheme,
    TEXT_COLOR, TEXT_PADDING_X, Theme, compute_cursor_offset, paint_omnibox,
    paint_omnibox_with_theme,
};
pub use state::OmniboxState;

//! Omnibox controller, painter and their tests.

mod controller;
mod paint;

#[cfg(test)]
mod tests;

pub use controller::OmniboxController;
pub use paint::{
    CHROME_FONT, OmniboxPainter, compute_cursor_offset, paint_omnibox, paint_omnibox_with_theme,
};

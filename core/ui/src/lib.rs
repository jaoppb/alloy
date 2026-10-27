//! # `ui` — the browser chrome as a widget domain
//!
//! The render tree and its hit testing, the theme, and the omnibox (state,
//! controller, painter, classifier). A sibling of `core/browser`
//! (`ADR-0026`): neither depends on the other. The chrome sees the page only
//! through opaque [`region::ContentRegion`]s and never an `href`; it returns
//! effects ([`UiEffect`]) and `alloy`'s executor mediates.
//!
//! ## Layout (`ADR-0010` §1)
//!
//! - [`domain`] — render tree, theme, omnibox state and classifier, the keymap and resolved
//!   input, typed errors.
//! - [`application`] — the input router, the chrome controller, the omnibox controller and painter.

#![forbid(unsafe_code)]

pub mod application;
pub mod domain;

pub use application::input::{InputRouter, UiController};
pub use application::omnibox::{
    CHROME_FONT, OmniboxController, OmniboxPainter, compute_cursor_offset, paint_omnibox,
    paint_omnibox_with_theme,
};
pub use domain::error::UiError;
pub use domain::input::{
    ActionName, BrowserCommand, ChordKey, ChordText, FocusTarget, KeyChord, Keymap, KeymapBinding,
    KeymapError, UiEffect, UiEffects, UiInput,
};
pub use domain::omnibox::{
    CursorPosition, EditKey, GapBuffer, OmniboxBuffer, OmniboxClassifier, OmniboxDestination,
    OmniboxState, SearchTemplate,
};
pub use domain::render_tree::{UiHitTarget, UiNode, UiNodeChildren, UiNodeKind, UiRenderTree};
pub use domain::theme::{self, ChromeHeight, OmniboxTheme, Theme};

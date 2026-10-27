//! # `ui` — the browser chrome as a widget domain
//!
//! The render tree and its hit testing, the theme, and the omnibox (state,
//! controller, painter, classifier). A sibling of `core/browser`
//! (`ADR-0026`): neither depends on the other. The chrome sees the page only
//! through opaque [`region::ContentRegion`]s and never an `href`; it returns
//! effects ([`OmniboxEffect`]) and `alloy`'s executor mediates.
//!
//! ## Layout (`ADR-0010` §1)
//!
//! - [`domain`] — render tree, theme, omnibox state and classifier, typed errors.
//! - [`application`] — the omnibox controller and painter.

#![forbid(unsafe_code)]

pub mod application;
pub mod domain;

pub use application::omnibox::{
    CHROME_FONT, OmniboxController, OmniboxPainter, compute_cursor_offset, paint_omnibox,
    paint_omnibox_with_theme,
};
pub use domain::error::UiError;
pub use domain::omnibox::{
    CursorPosition, EditKey, GapBuffer, OmniboxBuffer, OmniboxClassifier, OmniboxDestination,
    OmniboxEffect, OmniboxInput, OmniboxState, SearchTemplate,
};
pub use domain::render_tree::{UiHitTarget, UiNode, UiNodeChildren, UiNodeKind, UiRenderTree};
pub use domain::theme::{self, ChromeHeight, OmniboxTheme, Theme};

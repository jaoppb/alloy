//! What a node of the UI render tree is.

use region::RegionId;

/// The role and identity of a node in the UI render tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UiNodeKind {
    /// The root window surface.
    Window,
    /// The browser chrome top bar container.
    Chrome,
    /// The omnibox address/search input field within the chrome.
    Omnibox,
    /// The web document content area below the chrome.
    Content,
    /// A hyperlink within the web document content, by the id the browsing
    /// context minted for it — `core/ui` never sees an `href` (#110 decision 12).
    Link(RegionId),
}

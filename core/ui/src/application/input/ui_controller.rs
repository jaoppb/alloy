//! [`UiController`] — the chrome's input entry point.

use network::Url;
use platform::HostOs;
use window::{InputText, KeyEvent, PhysicalPosition};

use super::input_router::InputRouter;
use crate::application::omnibox::OmniboxController;
use crate::domain::input::{FocusTarget, Keymap, UiEffect, UiInput};
use crate::domain::render_tree::{UiHitTarget, UiRenderTree};

/// Owns the chrome's focus and controllers and turns window input into
/// [`UiEffect`]s. Nothing here touches browser state: `alloy` executes the
/// effects (`ADR-0026`).
///
/// Focus is derived from the omnibox rather than stored beside it, so the two
/// cannot disagree: the page holds it whenever the address bar does not.
#[derive(Clone, Debug)]
pub struct UiController {
    router: InputRouter,
    omnibox: OmniboxController,
}

impl UiController {
    #[must_use]
    pub fn new(keymap: Keymap, host: HostOs) -> Self {
        Self {
            router: InputRouter::new(keymap, host),
            omnibox: OmniboxController::new(),
        }
    }

    /// Who receives typing right now.
    #[must_use]
    pub const fn focus(&self) -> FocusTarget {
        if self.omnibox.is_focused() {
            FocusTarget::Omnibox
        } else {
            FocusTarget::Content
        }
    }

    #[must_use]
    pub const fn omnibox(&self) -> &OmniboxController {
        &self.omnibox
    }

    /// Shows `url` in the address bar after the browser navigated.
    pub fn reset_to_url(&mut self, url: &Url) {
        self.omnibox.reset_to_url(url);
    }

    /// Replaces the key bindings (init or reload).
    pub fn set_keymap(&mut self, keymap: Keymap) {
        self.router.set_keymap(keymap);
    }

    /// Routes a key event and applies what it resolves to.
    pub fn handle_key(&mut self, event: &KeyEvent) -> UiEffect {
        let routed = self.router.route_key(event, self.focus());
        self.apply(routed)
    }

    /// Applies an input-method commit.
    pub fn handle_text_committed(&mut self, text: InputText) -> UiEffect {
        let routed = self.router.route_text(text, self.focus());
        self.apply(routed)
    }

    /// Resolves a primary-button press at `position` through `tree`: a click
    /// on the chrome focuses the address bar, a click on the page moves focus
    /// to it (and cancels address editing), a click on a link also asks the
    /// browser to follow it.
    pub fn handle_click(&mut self, tree: &UiRenderTree, position: PhysicalPosition) -> UiEffect {
        match tree.hit_test(position) {
            Some(UiHitTarget::Omnibox | UiHitTarget::Chrome) => self.omnibox.focus(),
            Some(UiHitTarget::PageLink(region)) => {
                self.omnibox.handle_escape();
                UiEffect::FollowLink(region)
            }
            Some(UiHitTarget::PageContent) => self.omnibox.handle_escape(),
            None => UiEffect::None,
        }
    }

    fn apply(&mut self, input: Option<UiInput>) -> UiEffect {
        input.map_or(UiEffect::None, |input| self.omnibox.handle_input(&input))
    }
}

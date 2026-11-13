//! The one place the chrome and the browsing context meet (`ADR-0026`).
//!
//! `core/ui` and `core/browser` are siblings that return effects and never call
//! each other; [`EffectExecutor`] reads each effect and mutates the other side.
//! It borrows exactly what it mediates for the duration of one dispatch and
//! owns nothing, so durable state stays with whoever built the borrows.

use browser::{BrowserEffect, BrowsingContext, Effects, FetchResult, MarkupDiscoverer};
use network::Url;
use ui::{UiController, UiEffect, UiEffects};

use crate::application::compositor::{Damage, PendingDamage};
use crate::application::pending_title::PendingTitle;
use crate::application::ports::FetchExecutor;

#[cfg(test)]
mod tests;

/// Carries out [`UiEffect`]s and [`BrowserEffect`]s against the borrowed state.
pub struct EffectExecutor<'a, X> {
    browsing: &'a mut BrowsingContext<MarkupDiscoverer>,
    ui: &'a mut UiController,
    damage: &'a mut PendingDamage,
    title: &'a mut PendingTitle,
    fetcher: &'a X,
}

impl<'a, X: FetchExecutor> EffectExecutor<'a, X> {
    pub const fn new(
        browsing: &'a mut BrowsingContext<MarkupDiscoverer>,
        ui: &'a mut UiController,
        damage: &'a mut PendingDamage,
        title: &'a mut PendingTitle,
        fetcher: &'a X,
    ) -> Self {
        Self {
            browsing,
            ui,
            damage,
            title,
            fetcher,
        }
    }

    /// Starts navigating to `url`; the document arrives later as a
    /// [`FetchResult`]. Supersedes every navigation and subresource fetch still
    /// in flight: their results are dropped by the browsing context.
    pub fn navigate(&mut self, url: Url) {
        let effects = self.browsing.navigate(url);
        self.execute_browser(effects);
    }

    /// Applies a finished fetch to the browsing context.
    pub fn complete(&mut self, result: FetchResult) {
        let effects = self.browsing.complete(result);
        self.execute_browser(effects);
    }

    /// Carries out what the chrome asked for: a redraw is chrome damage, never
    /// a relayout of the document.
    pub fn execute_ui(&mut self, effects: UiEffects) {
        for effect in effects {
            self.execute_ui_effect(effect);
        }
    }

    fn execute_ui_effect(&mut self, effect: UiEffect) {
        match effect {
            UiEffect::None => {}
            UiEffect::ChromeDamaged => self.damage.record(Damage::Chrome),
            UiEffect::Navigate(url) => self.navigate(url),
            UiEffect::FollowLink(region) => {
                let effects = self.browsing.follow_link(region);
                self.execute_browser(effects);
            }
        }
    }

    /// Carries out what the browsing context asked for: fetches go to the
    /// fetch executor, the omnibox follows the URL (chrome damage), document
    /// damage schedules a relayout, a new title waits for the loop to apply it.
    fn execute_browser(&mut self, effects: Effects) {
        for effect in effects {
            match effect {
                BrowserEffect::NavigationStarted(_, url)
                | BrowserEffect::DocumentLoaded(_, url) => {
                    self.ui.reset_to_url(&url);
                    self.damage.record(Damage::Chrome);
                }
                BrowserEffect::Fetch(request, navigation) => {
                    self.fetcher.execute(request, navigation);
                }
                BrowserEffect::ContentDamaged(_) => self.damage.record(Damage::Content),
                BrowserEffect::TitleChanged(_, title) => self.title.record(&title),
            }
        }
    }
}

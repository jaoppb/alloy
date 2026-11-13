//! The window title a browser effect asked for, until the loop applies it.
//!
//! The [`EffectExecutor`](crate::application::EffectExecutor) is not generic
//! over the window (ADR-0019): it records the title here, and `pump_once`,
//! which holds the `WindowSystem`, hands it to `set_title` (#84).

use browser::DocumentTitle;
use window::WindowTitle;

/// The latest title not yet given to the window; a newer one replaces it.
#[derive(Debug, Default)]
pub struct PendingTitle(Option<WindowTitle>);

impl PendingTitle {
    /// Queues `title`, replacing any title the loop has not applied yet.
    pub fn record(&mut self, title: DocumentTitle) {
        self.0 = Some(WindowTitle::from(title.into_string()));
    }

    /// Hands over the queued title, leaving none.
    #[must_use]
    pub const fn take(&mut self) -> Option<WindowTitle> {
        self.0.take()
    }
}

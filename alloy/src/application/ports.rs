//! Ports the application layer calls and `infrastructure` implements
//! (`ADR-0010`): the application says *what* to fetch or decode, never which
//! thread or codec does it.

use browser::{FetchRequest, NavigationId};
use graphics::Framebuffer;

use crate::error::AlloyError;

/// Runs the fetches `core/browser` asks for (`BrowserEffect::Fetch`).
///
/// `execute` never blocks the caller: it starts the work and returns. The
/// outcome comes back as a `FetchResult` stamped with `navigation` — and so
/// with its `ContextId` (#110 decision 14) — on the [`FetchInbox`] the
/// executor was built over, where the loop drains it (`ADR-0019`). There is no
/// cancellation: a superseded navigation's result still arrives and the
/// browsing context drops it by id.
///
/// [`FetchInbox`]: crate::application::fetch_channel::FetchInbox
pub trait FetchExecutor {
    /// Starts fetching `request` on behalf of `navigation`.
    fn execute(&self, request: FetchRequest, navigation: NavigationId);
}

/// What the loop does when a pump cycle found nothing to do.
///
/// It keeps the loop from spinning a CPU core while a fetch is in flight
/// (`ADR-0019`: no async runtime and no OS wait primitive spanning the window
/// and the fetch channel).
pub trait IdlePause {
    /// Gives the CPU up for a short while.
    fn pause(&self);
}

/// Turns the bytes of a fetched image into pixels.
///
/// Declared here with its sibling ports (#114), though nothing in `application`
/// calls it: `infrastructure` composes it inside `HttpFetcher::fetch_image`. It
/// is `Send + Sync + 'static` because that fetcher crosses threads in an `Arc`,
/// which the two sibling ports, driven from the loop's own thread, never do.
pub trait ImageDecoder: Send + Sync + 'static {
    /// Decodes `bytes`, or says why they are not a supported image.
    fn decode(&self, bytes: &[u8]) -> Result<Framebuffer, AlloyError>;
}

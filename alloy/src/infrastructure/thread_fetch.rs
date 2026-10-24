//! [`FetchExecutor`] with one `std::thread` per fetch (`ADR-0019`).
//!
//! Blocking I/O runs off the loop's thread and reports back over the
//! [`FetchOutbox`]; no async runtime, no pool, no cancellation — a superseded
//! navigation's thread runs to completion and its result is dropped on arrival
//! by `NavigationId` (#110 non-goals).

use std::sync::Arc;
use std::thread;

use browser::{FetchRequest, FetchResult, NavigationId};
use network::{HttpTransport, RequestPolicy};

use super::http_fetch::HttpFetcher;
use super::png_decoder::PngImageDecoder;
use crate::application::fetch_channel::{FetchInbox, FetchOutbox, fetch_channel};
use crate::application::ports::{FetchExecutor, ImageDecoder};

/// Runs every fetch on its own thread over a shared [`HttpFetcher`].
pub struct ThreadFetchExecutor<T, P, D> {
    fetcher: Arc<HttpFetcher<T, P, D>>,
    outbox: FetchOutbox,
}

impl<T, P> ThreadFetchExecutor<T, P, PngImageDecoder>
where
    T: HttpTransport + 'static,
    P: RequestPolicy + 'static,
{
    /// An executor decoding PNGs over `transport` and `policy`, and the inbox
    /// its results arrive in.
    #[must_use]
    pub fn connected(transport: T, policy: P) -> (Self, FetchInbox) {
        let (outbox, inbox) = fetch_channel();
        let executor = Self::new(transport, policy, PngImageDecoder, outbox);
        (executor, inbox)
    }
}

impl<T, P, D> ThreadFetchExecutor<T, P, D>
where
    T: HttpTransport + 'static,
    P: RequestPolicy + 'static,
    D: ImageDecoder,
{
    /// An executor reporting on `outbox`.
    #[must_use]
    pub fn new(transport: T, policy: P, decoder: D, outbox: FetchOutbox) -> Self {
        Self {
            fetcher: Arc::new(HttpFetcher::new(transport, policy, decoder)),
            outbox,
        }
    }
}

impl<T, P, D> FetchExecutor for ThreadFetchExecutor<T, P, D>
where
    T: HttpTransport + 'static,
    P: RequestPolicy + 'static,
    D: ImageDecoder,
{
    fn execute(&self, request: FetchRequest, navigation: NavigationId) {
        let fetcher = Arc::clone(&self.fetcher);
        let outbox = self.outbox.clone();
        thread::spawn(move || {
            let payload = fetcher.fetch(request);
            outbox.send(FetchResult::new(navigation, payload));
        });
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::time::{Duration, Instant};

    use browser::{FetchRequest, FetchResult, Navigations, StylesheetRequest, SubresourceRequest};
    use network::{AllowAllPolicy, MockTransport, Url};
    use region::ContextId;

    use super::ThreadFetchExecutor;
    use crate::application::fetch_channel::FetchInbox;
    use crate::application::ports::FetchExecutor;

    /// The adapter really runs on another thread, so this is the one test that
    /// has to wait for it; the loop's own tests use a synchronous executor.
    fn wait_for_result(inbox: &FetchInbox) -> FetchResult {
        let started = Instant::now();
        loop {
            if let Some(result) = inbox.next_ready() {
                return result;
            }
            assert!(
                started.elapsed() < Duration::from_secs(10),
                "the fetch never reported"
            );
            std::thread::yield_now();
        }
    }

    #[test]
    fn a_result_carries_the_navigation_and_context_it_was_started_for() {
        let (executor, inbox) =
            ThreadFetchExecutor::connected(MockTransport::new(), AllowAllPolicy::new());
        let mut navigations = Navigations::new(ContextId::ROOT);
        navigations.supersede();
        let navigation = navigations.latest();
        let sheet = StylesheetRequest::new(
            Url::parse("http://example.com/a.css").unwrap(),
            css::MediaQuery::always(),
        );

        executor.execute(
            FetchRequest::Subresource(SubresourceRequest::Stylesheet(sheet)),
            navigation,
        );
        let result = wait_for_result(&inbox);

        assert_eq!(result.navigation(), navigation);
        assert_eq!(result.context(), ContextId::ROOT);
    }
}

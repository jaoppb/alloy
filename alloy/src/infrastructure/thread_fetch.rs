//! [`FetchExecutor`] with one `std::thread` per fetch (`ADR-0019`).
//!
//! Blocking I/O runs off the loop's thread and reports back over the
//! [`FetchOutbox`]; no async runtime, no pool, no cancellation — a superseded
//! navigation's thread runs to completion and its result is dropped on arrival
//! by `NavigationId` (#110 non-goals).

use std::panic::{self, AssertUnwindSafe};
use std::sync::Arc;
use std::thread;

use browser::{FetchFailure, FetchPayload, FetchRequest, FetchResult, NavigationId};
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
        let request = Arc::new(request);
        let job = {
            let fetcher = Arc::clone(&self.fetcher);
            let outbox = self.outbox.clone();
            let request = Arc::clone(&request);
            move || {
                let payload = fetch_reporting_panics(&fetcher, &request, navigation);
                outbox.send(FetchResult::new(navigation, payload));
            }
        };
        let spawned = thread::Builder::new()
            .name("alloy-fetch".to_owned())
            .spawn(job);
        if let Err(error) = spawned {
            // The OS refused a thread: say so on the page instead of aborting the loop.
            tracing::error!(%error, %navigation, url = %request.url(), "could not start a fetch thread");
            let payload = request.failed(page_not_loaded());
            self.outbox.send(FetchResult::new(navigation, payload));
        }
    }
}

/// What the error card says when the work could not run at all; the cause goes
/// to the log, where a person debugging can correlate it by navigation.
fn page_not_loaded() -> FetchFailure {
    FetchFailure::new("the page could not be loaded")
}

/// Fetches `request`, turning a panic into a failed payload.
///
/// A worker nobody joins would otherwise die silently: no result would ever
/// reach the loop and the page would stay "loading" with nothing to show. Like
/// a trapped script panic (ADR-0004), it must never go unreported.
fn fetch_reporting_panics<T, P, D>(
    fetcher: &HttpFetcher<T, P, D>,
    request: &FetchRequest,
    navigation: NavigationId,
) -> FetchPayload
where
    T: HttpTransport,
    P: RequestPolicy,
    D: ImageDecoder,
{
    panic::catch_unwind(AssertUnwindSafe(|| fetcher.fetch(request))).unwrap_or_else(|_| {
        tracing::error!(%navigation, url = %request.url(), "a fetch worker panicked");
        request.failed(page_not_loaded())
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::time::{Duration, Instant};

    use browser::{
        FetchPayload, FetchRequest, FetchResult, NavigationId, Navigations, StylesheetRequest,
        SubresourceRequest,
    };
    use network::{
        AllowAllPolicy, HttpRequest, HttpResponse, HttpTransport, MockTransport, NetworkError, Url,
    };
    use region::ContextId;

    use super::{ThreadFetchExecutor, fetch_reporting_panics};
    use crate::application::fetch_channel::{FetchInbox, fetch_channel};
    use crate::application::ports::FetchExecutor;
    use crate::infrastructure::{HttpFetcher, PngImageDecoder};

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
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    /// A stylesheet fetch and the navigation it was started for.
    fn stylesheet_fetch() -> (FetchRequest, NavigationId) {
        let mut navigations = Navigations::new(ContextId::ROOT);
        navigations.supersede();
        let sheet = StylesheetRequest::new(
            Url::parse("http://example.com/a.css").unwrap(),
            css::MediaQuery::always(),
        );
        let request = FetchRequest::Subresource(SubresourceRequest::Stylesheet(sheet));
        (request, navigations.latest())
    }

    #[test]
    fn a_result_carries_the_navigation_it_was_started_for() {
        let (executor, inbox) =
            ThreadFetchExecutor::connected(MockTransport::new(), AllowAllPolicy::new());
        let (request, navigation) = stylesheet_fetch();

        executor.execute(request, navigation);

        assert_eq!(wait_for_result(&inbox).navigation(), navigation);
    }

    struct PanickingTransport;

    impl HttpTransport for PanickingTransport {
        #[allow(clippy::panic_in_result_fn)] // panicking is the point of this fixture
        fn execute(&self, _request: &HttpRequest) -> Result<HttpResponse, NetworkError> {
            panic!("the transport blew up");
        }
    }

    #[test]
    fn a_panicking_fetch_is_answered_with_a_failure_shaped_like_its_request() {
        let fetcher = HttpFetcher::new(PanickingTransport, AllowAllPolicy::new(), PngImageDecoder);
        let (request, navigation) = stylesheet_fetch();

        let payload = fetch_reporting_panics(&fetcher, &request, navigation);

        assert!(
            matches!(payload, FetchPayload::Stylesheet(_, Err(_))),
            "got {payload:?}"
        );
    }

    #[test]
    fn a_panicking_worker_still_reports_its_navigation() {
        let (outbox, inbox) = fetch_channel();
        let executor = ThreadFetchExecutor::new(
            PanickingTransport,
            AllowAllPolicy::new(),
            PngImageDecoder,
            outbox,
        );
        let (request, navigation) = stylesheet_fetch();

        executor.execute(request, navigation);

        assert_eq!(wait_for_result(&inbox).navigation(), navigation);
    }
}

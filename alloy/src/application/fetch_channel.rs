//! The channel a fetch executor reports on and the loop drains (`ADR-0019`).
//!
//! One pair for a session's whole life: a result from a page the user left is
//! not cut off here but dropped by the browsing context, because it is stamped
//! with a superseded `NavigationId`.

use std::sync::mpsc::{self, Receiver, Sender};

use browser::FetchResult;

/// The sending half: what an executor reports finished fetches on. Cheap to
/// clone, so every worker holds its own.
#[derive(Clone, Debug)]
pub struct FetchOutbox(Sender<FetchResult>);

impl FetchOutbox {
    /// Reports `result` to the loop. If the loop has already ended there is
    /// nobody left to tell, which is not an error worth surfacing.
    pub fn send(&self, result: FetchResult) {
        if self.0.send(result).is_err() {
            tracing::debug!("fetch result dropped: the event loop has ended");
        }
    }
}

/// The receiving half, owned by the loop's session.
#[derive(Debug)]
pub struct FetchInbox(Receiver<FetchResult>);

impl FetchInbox {
    /// The next result already waiting, without blocking.
    #[must_use]
    pub fn next_ready(&self) -> Option<FetchResult> {
        self.0.try_recv().ok()
    }
}

/// A connected outbox and inbox.
#[must_use]
pub fn fetch_channel() -> (FetchOutbox, FetchInbox) {
    let (sender, receiver) = mpsc::channel();
    (FetchOutbox(sender), FetchInbox(receiver))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use browser::{FetchPayload, FetchResult, Navigations};
    use region::ContextId;

    use super::fetch_channel;

    fn result() -> FetchResult {
        let mut navigations = Navigations::new(ContextId::ROOT);
        navigations.supersede();
        let sheet = FetchPayload::Stylesheet(css::MediaQuery::always(), Ok(String::new()));
        FetchResult::new(navigations.latest(), sheet)
    }

    #[test]
    fn a_sent_result_is_ready_in_order_and_then_the_inbox_is_empty() {
        let (outbox, inbox) = fetch_channel();
        let first = result();
        let expected = first.navigation();
        outbox.send(first);

        assert_eq!(inbox.next_ready().unwrap().navigation(), expected);
        assert!(inbox.next_ready().is_none());
    }

    #[test]
    fn sending_after_the_inbox_is_gone_does_not_panic() {
        let (outbox, inbox) = fetch_channel();
        drop(inbox);

        outbox.send(result());
    }
}

//! A panic the fetch worker traps puts its message on the log (ADR-0014, #114).
//!
//! Its own test binary with a single test: the worker logs from its own thread, so the subscriber
//! has to be the process-wide one, and a scoped one would race other tests' callsites.

#![allow(clippy::unwrap_used, clippy::panic, clippy::panic_in_result_fn)]

use std::io;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use alloy::application::FetchExecutor;
use alloy::infrastructure::ThreadFetchExecutor;
use browser::{FetchRequest, Navigations, StylesheetRequest, SubresourceRequest};
use network::{AllowAllPolicy, HttpRequest, HttpResponse, HttpTransport, NetworkError, Url};
use region::ContextId;
use tracing_subscriber::fmt::MakeWriter;

#[derive(Clone, Default)]
struct LogBuffer(Arc<Mutex<Vec<u8>>>);

impl io::Write for LogBuffer {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl MakeWriter<'_> for LogBuffer {
    type Writer = Self;

    fn make_writer(&self) -> Self {
        self.clone()
    }
}

struct PanickingTransport;

impl HttpTransport for PanickingTransport {
    fn execute(&self, _request: &HttpRequest) -> Result<HttpResponse, NetworkError> {
        panic!("the transport blew up");
    }
}

#[test]
fn a_trapped_panics_message_reaches_the_log() {
    let log = LogBuffer::default();
    tracing::subscriber::set_global_default(
        tracing_subscriber::fmt()
            .with_writer(log.clone())
            .with_ansi(false)
            .finish(),
    )
    .unwrap();
    let (executor, inbox) =
        ThreadFetchExecutor::connected(PanickingTransport, AllowAllPolicy::new());
    let mut navigations = Navigations::new(ContextId::ROOT);
    navigations.supersede();
    let sheet = StylesheetRequest::new(
        Url::parse("http://example.com/a.css").unwrap(),
        css::MediaQuery::always(),
    );

    executor.execute(
        FetchRequest::Subresource(SubresourceRequest::Stylesheet(sheet)),
        navigations.latest(),
    );

    let started = Instant::now();
    while inbox.next_ready().is_none() {
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "the fetch never reported"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
    let logged = String::from_utf8(log.0.lock().unwrap().clone()).unwrap();
    assert!(logged.contains("the transport blew up"), "got {logged}");
}

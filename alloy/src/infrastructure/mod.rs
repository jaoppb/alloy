//! Adapters implementing the ports `application` declares (`ADR-0010`).
//!
//! The only code in `alloy` that spawns a thread, sleeps, calls
//! `HttpTransport::execute` or decodes image bytes (#110 decision 26, #114).

pub mod http_fetch;
pub mod navigation;
pub mod png_decoder;
pub mod sleep_pause;
pub mod thread_fetch;

pub use http_fetch::HttpFetcher;
pub use navigation::navigate;
pub use png_decoder::PngImageDecoder;
pub use sleep_pause::SleepPause;
pub use thread_fetch::ThreadFetchExecutor;

//! Adapters implementing the ports `application` declares (`ADR-0010`) and the composition root's wiring.
//!
//! The only code in `alloy` that spawns a thread, sleeps, calls
//! `HttpTransport::execute` or decodes image bytes (#110 decision 26, #114).

pub mod http_fetch;
pub mod png_decoder;
pub mod sleep_pause;
mod software_compositor;
pub mod thread_fetch;

pub use http_fetch::HttpFetcher;
pub use png_decoder::PngImageDecoder;
pub use sleep_pause::SleepPause;
pub use software_compositor::software_compositor;
pub use thread_fetch::ThreadFetchExecutor;

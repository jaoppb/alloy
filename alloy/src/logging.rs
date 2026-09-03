//! `tracing` subscriber setup for the CLI (ADR-0014).
//!
//! Level comes from `ALLOY_LOG` (or `RUST_LOG`), default `info` — never a
//! hardcoded level. Format is pretty by default, `json` when
//! `ALLOY_LOG_FORMAT=json`. Output goes to stderr.

use std::io;

use tracing_subscriber::EnvFilter;

/// Install the global subscriber. Call once, first thing in `main`.
pub fn init() {
    let filter =
        EnvFilter::try_from_env(level_var()).unwrap_or_else(|_| EnvFilter::new(DEFAULT_LEVEL));

    let builder = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(io::stderr);

    if json_requested() {
        builder.json().init();
        return;
    }
    builder.pretty().init();
}

const DEFAULT_LEVEL: &str = "info";

/// `ALLOY_LOG` takes precedence over `RUST_LOG` when set.
fn level_var() -> &'static str {
    std::env::var_os("ALLOY_LOG").map_or("RUST_LOG", |_| "ALLOY_LOG")
}

fn json_requested() -> bool {
    std::env::var("ALLOY_LOG_FORMAT").is_ok_and(|value| value.eq_ignore_ascii_case("json"))
}

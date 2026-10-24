//! [`IdlePause`] over `std::thread::sleep`.

use std::thread;
use std::time::Duration;

use crate::application::ports::IdlePause;

/// How long an idle pump cycle sleeps — keeps the loop from busy-spinning a CPU
/// core while a fetch is in flight, with no async runtime and no OS blocking
/// primitive spanning both the window and the fetch channel (`ADR-0019`).
const IDLE_POLL: Duration = Duration::from_millis(4);

/// Sleeps [`IDLE_POLL`] per pause.
#[derive(Clone, Copy, Debug, Default)]
pub struct SleepPause;

impl IdlePause for SleepPause {
    fn pause(&self) {
        thread::sleep(IDLE_POLL);
    }
}

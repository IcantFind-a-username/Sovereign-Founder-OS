//! Named crash points, compiled only under the `fault-injection` feature.
//!
//! A durability claim is only worth the crash it was tested against, and the
//! interesting crash is never "somewhere during the write" — it is at one
//! exact instant. `publish_record` writes a temp file, fsyncs it, then
//! publishes by hard link. A process killed between those two steps has a
//! complete, durable temp file and no record. Whether a later reader can be
//! confused by that debris is a real question, and this is how a test asks it.
//!
//! Two properties this must have, and both are structural rather than
//! promised:
//!
//!   - **It cannot exist in a shipped artifact.** The whole module is behind a
//!     non-default feature, so the barrier name, the environment variable, and
//!     the code that reads them are absent from a default build. There is no
//!     runtime flag to get wrong. `scripts/check-fault-injection-excluded.sh`
//!     proves it against the actual release binary.
//!   - **It is an exact point, not an approximation.** The child stops
//!     *because it reached this line*, not because a test slept and hoped. It
//!     announces the barrier on stdout and then waits to be killed, so the
//!     parent acts on an observation instead of a timer.

use std::io::Write;

/// The environment variable naming the barrier this process should stop at.
/// Read only from this module, which does not exist without the feature.
pub const BARRIER_ENV: &str = "SOVEREIGN_AUTHORITY_BARRIER";

/// Printed on stdout the moment a barrier is reached, so a parent can wait
/// for the fact rather than for a duration.
pub const REACHED_PREFIX: &str = "authority-barrier-reached: ";

/// How long a stopped process waits to be killed. Finite so a parent that
/// regresses and never kills cannot hang a suite forever; far longer than any
/// parent needs, so it never fires in a healthy run.
const WAIT: std::time::Duration = std::time::Duration::from_secs(30);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Barrier {
    /// Inside `publish_record`: the temp file is written and fsynced, and the
    /// hard link that publishes it has not been attempted.
    LegacyAfterTempSyncBeforePublish,
}

impl Barrier {
    pub const fn name(self) -> &'static str {
        match self {
            Barrier::LegacyAfterTempSyncBeforePublish => "LegacyAfterTempSyncBeforePublish",
        }
    }
}

/// Stop here if this process was asked to. Otherwise do nothing at all.
pub(crate) fn reach(barrier: Barrier) {
    let requested = match std::env::var(BARRIER_ENV) {
        Ok(value) => value,
        Err(_) => return,
    };
    if requested != barrier.name() {
        return;
    }
    // Announce, flush, and wait to be killed. The flush matters: a parent
    // blocked on this line must not be waiting on a buffer.
    let mut stdout = std::io::stdout();
    let _ = writeln!(stdout, "{REACHED_PREFIX}{}", barrier.name());
    let _ = stdout.flush();
    std::thread::sleep(WAIT);
}

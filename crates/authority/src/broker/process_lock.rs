//! The lock that makes "sole writable owner" true rather than intended.
//!
//! Everything before this point authenticates *who* may talk to the broker.
//! This answers a different question: whether this process may own the store
//! at all, when another broker may already be running against the same root.
//!
//! Two failure modes are deliberately not treated alike. A lock already held
//! means a second broker is running, which is an ordinary, recoverable
//! situation with an exact name. Every other error — the file cannot be
//! opened, the filesystem does not support locking, the path is not what was
//! expected — means the question could not be answered, and an unanswered
//! question about exclusivity must never be read as "yes". Collapsing the two
//! is how a store ends up with two writers.
//!
//! The lock is a real advisory lock on a real file, not a PID file. A PID file
//! records a claim; a lock *is* one, and the kernel releases it when the
//! holder dies, so a broker killed with SIGKILL leaves nothing stale behind.

use std::fs::{File, OpenOptions};
use std::path::Path;

/// Fixed name beside the store. Not configurable: two brokers that disagreed
/// about which file to lock would both succeed.
pub const LOCK_FILE: &str = ".owner-effect-broker.lock";

#[derive(Debug)]
pub enum LockError {
    /// Another broker holds the lock. The only recoverable outcome.
    BrokerAlreadyRunning,
    /// The question could not be answered. Never treated as success.
    Unavailable,
}

/// The held lock. Dropping it releases the lock, so the type is the lifetime:
/// there is no `unlock` to forget to call, and a broker that returns early
/// cannot leave the store claimed.
///
/// The `File` is deliberately kept private and never cloned. A duplicated
/// descriptor could outlive this value and hold the lock past the point the
/// broker believes it released it.
#[derive(Debug)]
pub struct HeldLock {
    _file: File,
}

/// Take the lock, or say precisely why not.
pub fn acquire(root: &Path) -> Result<HeldLock, LockError> {
    let path = root.join(LOCK_FILE);

    // Read+write: a lock on a read-only handle is refused on some platforms,
    // and owner-only because the file sits beside the store.
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options.open(&path).map_err(|_| LockError::Unavailable)?;

    // Non-blocking. A blocking lock would make a second broker wait instead of
    // reporting the fact, and the caller has a deadline to respect.
    match file.try_lock() {
        Ok(()) => Ok(HeldLock { _file: file }),
        Err(std::fs::TryLockError::WouldBlock) => Err(LockError::BrokerAlreadyRunning),
        // Anything else means exclusivity is unknown, which is not permission.
        Err(_) => Err(LockError::Unavailable),
    }
}

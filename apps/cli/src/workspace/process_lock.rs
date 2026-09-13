//! Exclusive writer lock for one workspace root across processes.
//!
//! Two founders' tools must not mutate the same root concurrently; without a
//! real advisory lock, two `decide` calls on one pending approval can race
//! through the durable send workflow and double-consume authority even when the
//! outbox write is exclusive. The lock is a regular file with `try_lock`, not
//! a PID claim — the kernel releases it when the holder dies.

use std::fs::{File, OpenOptions};
use std::path::Path;

pub const LOCK_FILE: &str = ".workspace-writer.lock";

#[derive(Debug)]
pub enum LockError {
    /// Another process holds the lock.
    AlreadyRunning,
    /// Exclusivity could not be established; never treated as permission.
    Unavailable,
}

pub struct HeldLock {
    _file: File,
}

pub fn acquire(root: &Path) -> Result<HeldLock, LockError> {
    let path = root.join(LOCK_FILE);
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options.open(&path).map_err(|_| LockError::Unavailable)?;
    match file.try_lock() {
        Ok(()) => Ok(HeldLock { _file: file }),
        Err(std::fs::TryLockError::WouldBlock) => Err(LockError::AlreadyRunning),
        Err(_) => Err(LockError::Unavailable),
    }
}

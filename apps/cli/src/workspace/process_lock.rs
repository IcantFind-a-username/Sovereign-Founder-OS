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
    file: File,
}

impl Drop for HeldLock {
    fn drop(&mut self) {
        // Closing the fd also releases the advisory lock. Unlock first so a
        // same-process reacquire immediately after drop is not WouldBlock on
        // filesystems that delay close-time release (GHA overlayfs).
        let _ = self.file.unlock();
    }
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
        Ok(()) => Ok(HeldLock { file }),
        Err(std::fs::TryLockError::WouldBlock) => Err(LockError::AlreadyRunning),
        Err(_) => Err(LockError::Unavailable),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn dropping_the_lock_releases_it_for_immediate_reacquire() {
        let dir = tempdir().unwrap();
        for _ in 0..64 {
            let held = acquire(dir.path()).expect("acquire");
            drop(held);
            acquire(dir.path()).expect("reacquire after drop must succeed");
        }
    }

    #[test]
    fn a_second_holder_is_already_running() {
        let dir = tempdir().unwrap();
        let _first = acquire(dir.path()).expect("first acquire succeeds");
        assert!(
            matches!(acquire(dir.path()), Err(LockError::AlreadyRunning)),
            "a second holder must get AlreadyRunning, not a generic failure"
        );
    }
}

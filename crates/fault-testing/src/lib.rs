//! Shared fault injection for tests.
//!
//! Durability claims — "the entry survives a failed write", "the chain still
//! verifies", "no partial file is left behind" — are only worth as much as the
//! failures they were tested against. Today every crate hand-rolls its own
//! corruption helper, nothing injects a *failing write*, and only
//! `crates/sandbox` kills a real child. These three primitives are that
//! missing half, in one place so each crate tests the same fault.
//!
//! This is a dev-dependency. No shipping crate may depend on it.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

/// Makes a filesystem location unavailable for the guard's lifetime, and puts
/// back whatever was there when it drops.
///
/// It works by putting a regular file where a directory is expected, so every
/// write beneath it fails with `ENOTDIR`. That is deliberate: the obvious
/// alternative, removing the write permission, is a silent no-op for root, and
/// both the project's containers and nightly CI run as root — a chmod-based
/// guard would report success while injecting nothing. A regular file denies
/// every uid equally.
#[derive(Debug)]
pub struct BlockedPath {
    path: PathBuf,
    /// Where the original directory was moved, if one existed.
    stashed: Option<PathBuf>,
}

impl BlockedPath {
    /// Block `path`. It may be an existing directory, which is moved aside and
    /// restored on drop, or a location that does not exist yet, which is the
    /// case when the code under test is expected to create it.
    pub fn block(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let stashed = match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.is_dir() => {
                let stashed = sibling(&path, ".blocked-original");
                fs::rename(&path, &stashed)?;
                Some(stashed)
            }
            Ok(_) => {
                return Err(io::Error::other(format!(
                    "{} is not a directory; blocking it would prove nothing",
                    path.display()
                )))
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => return Err(error),
        };
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, b"blocked by sovereign-fault-testing")?;
        Ok(Self { path, stashed })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for BlockedPath {
    fn drop(&mut self) {
        // Best effort by necessity: a panicking drop would replace the test's
        // real failure with this one. A leftover blocker is confined to the
        // test's own temporary directory.
        let _ = fs::remove_file(&self.path);
        if let Some(stashed) = &self.stashed {
            let _ = fs::rename(stashed, &self.path);
        }
    }
}

/// Flip one byte of a file in place, leaving its length unchanged.
///
/// A changed byte and a changed length are different faults: this one survives
/// every length check and is caught only by a hash or a signature.
pub fn corrupt_byte(path: impl AsRef<Path>, offset: usize) -> io::Result<()> {
    let path = path.as_ref();
    let mut bytes = fs::read(path)?;
    let len = bytes.len();
    let byte = bytes.get_mut(offset).ok_or_else(|| {
        io::Error::other(format!(
            "offset {offset} is past the end of {} ({len} bytes)",
            path.display()
        ))
    })?;
    *byte = byte.wrapping_add(1);
    fs::write(path, &bytes)
}

/// Drop the last `n` bytes of a file: the truncation an interrupted write
/// leaves behind.
pub fn truncate_by(path: impl AsRef<Path>, n: u64) -> io::Result<()> {
    let path = path.as_ref();
    let len = fs::metadata(path)?.len();
    let shorter = len.checked_sub(n).ok_or_else(|| {
        io::Error::other(format!(
            "cannot remove {n} bytes from {} ({len} bytes)",
            path.display()
        ))
    })?;
    fs::OpenOptions::new()
        .write(true)
        .open(path)?
        .set_len(shorter)
}

/// Run one `#[ignore]`d test in a fresh copy of this test binary, so the
/// caller can kill it mid-operation and inspect what it left on disk.
///
/// The worker is an ignored test guarded by `marker`, so it never runs in an
/// ordinary `cargo test`; it runs only when this function sets that variable.
/// The child's stdout is piped so the caller can wait for a progress marker
/// before killing it, rather than sleeping and hoping.
pub fn respawn_self(worker_test_name: &str, marker: (&str, &str)) -> io::Result<Child> {
    Command::new(std::env::current_exe()?)
        .args([
            "--exact",
            worker_test_name,
            "--test-threads=1",
            "--ignored",
            "--nocapture",
        ])
        .env(marker.0, marker.1)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
}

fn sibling(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(suffix);
    path.with_file_name(name)
}

#[cfg(test)]
mod tests;

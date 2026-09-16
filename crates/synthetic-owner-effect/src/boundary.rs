//! Classify the root, retain the OS lock, then open the sole redb store.
//!
//! Ordering is the contract:
//!
//! 1. classify via the existing fixture-root classifier — product, unmarked,
//!    and symlink roots end here, before a listener or any database file;
//! 2. acquire and retain the existing OS file lock;
//! 3. open redb through `OwnedStore`, which is reachable only with `&HeldLock`.
//!
//! A second live process is refused at step 2. That is the only two-live-process
//! claim in this slice. There is no HMAC supervisor, hidden child, or second
//! TCP listener on this path.

use std::path::{Path, PathBuf};

use sovereign_authority::broker::bootstrap::classify;
use sovereign_authority::broker::process_lock::{self, HeldLock, LockError};
use sovereign_authority::broker::store::{OwnedStore, StoreError};
use sovereign_capability::v2::VerifiedCapabilityV2;

/// Upper-crate seam: D05 will consume verified proofs from capability by
/// value. D02 pins the dependency so the coordinator cannot live below.
type CapabilityProofSeam = VerifiedCapabilityV2;
const _: Option<CapabilityProofSeam> = None;

/// Value-free outcomes. A path, a lock inode, or a store error string would
/// turn this interface into a filesystem oracle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundaryError {
    RootRejected,
    AlreadyRunning,
    LockUnavailable,
    StoreUnavailable,
    SignerUnavailable,
}

impl BoundaryError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::RootRejected => "E-ROOT-REJECTED",
            Self::AlreadyRunning => "E-BROKER-ALREADY-RUNNING",
            Self::LockUnavailable => "E-LOCK-UNAVAILABLE",
            Self::StoreUnavailable => "E-STORE-UNAVAILABLE",
            Self::SignerUnavailable => "E-SIGNER-UNAVAILABLE",
        }
    }
}

impl std::fmt::Display for BoundaryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

/// A classified fixture root whose OS lock this process holds.
///
/// Opening the store is a separate step so tests can observe that the lock
/// file exists while `authority.redb` does not. `open_store` is the only
/// production redb open in this crate.
pub struct ProcessBoundary {
    root: PathBuf,
    lock: HeldLock,
}

impl std::fmt::Debug for ProcessBoundary {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProcessBoundary")
            .field("root", &"<redacted>")
            .finish_non_exhaustive()
    }
}

impl ProcessBoundary {
    /// Classify `candidate`, then take the lock. Does not open redb and does
    /// not bind the public listener.
    pub fn acquire(candidate: &Path) -> Result<Self, BoundaryError> {
        let classified = classify(candidate).map_err(|_| BoundaryError::RootRejected)?;
        let lock = process_lock::acquire(classified.path()).map_err(|error| match error {
            LockError::BrokerAlreadyRunning => BoundaryError::AlreadyRunning,
            LockError::Unavailable => BoundaryError::LockUnavailable,
        })?;
        Ok(Self {
            root: classified.path().to_path_buf(),
            lock,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The only production redb open. The `&HeldLock` constructor is what
    /// makes "lock then store" unforgeable rather than a comment.
    pub fn open_store(&self) -> Result<OwnedStore<'_>, BoundaryError> {
        OwnedStore::open(&self.root, &self.lock).map_err(|error| match error {
            StoreError::AlreadyOpen | StoreError::Unavailable => BoundaryError::StoreUnavailable,
        })
    }

    /// Keep the sole store open for as long as `body` runs. The fixture
    /// process uses this so a second process meets the retained lock while
    /// this process still holds the database handle.
    pub fn hold_store<R>(
        &self,
        body: impl FnOnce(&OwnedStore<'_>) -> R,
    ) -> Result<R, BoundaryError> {
        let store = self.open_store()?;
        Ok(body(&store))
    }

    /// Generate the ephemeral approval signer. Must run after the lock is
    /// held and before redb opens: the caller still has a `ProcessBoundary`
    /// with no store.
    pub fn with_signer(self) -> Result<crate::LockedSigner, BoundaryError> {
        crate::LockedSigner::from_boundary(self)
    }
}

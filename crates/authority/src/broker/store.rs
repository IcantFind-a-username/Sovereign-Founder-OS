//! Opening the store, and the ordering that makes the broker its sole writer.
//!
//! Redb enforces one writer per database within a process and refuses a second
//! open of the same file. That is a real guarantee and it is not the one this
//! module is about: it says nothing about a *second process* that opens the
//! file after the first has exited, or about a caller that never went through
//! the broker at all. The process lock covers those, and this type exists to
//! make the ordering unforgeable rather than merely documented.
//!
//! `open` takes a `&HeldLock`. There is no other constructor. So a store
//! cannot be opened by code that has not already proved exclusivity, and the
//! borrow keeps the lock alive for as long as the store is — the two cannot
//! come apart, which is exactly the failure a comment would not prevent.

use super::process_lock::HeldLock;
// `begin_read` lives on a trait in redb 4, not on the type.
use redb::{Database, ReadableDatabase};
use std::path::Path;

/// Fixed name beside the lock. Two brokers that disagreed about which file is
/// the store would each own a different one and both believe they were alone.
pub const STORE_FILE: &str = "authority.redb";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreError {
    /// Another handle already has this database open.
    AlreadyOpen,
    /// The file could not be opened or created, or is not a redb database.
    Unavailable,
}

/// A database this process alone may write, for as long as the lock is held.
pub struct OwnedStore<'lock> {
    database: Database,
    // Not read, and not removable: it is what ties the store's lifetime to the
    // lock's. Dropping the lock first would make "sole writer" a claim about
    // the past.
    _lock: &'lock HeldLock,
}

impl std::fmt::Debug for OwnedStore<'_> {
    /// Value-free. A database handle's own `Debug` prints its path, and a path
    /// is a detail of the owner's machine that nothing here needs to log.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("OwnedStore(<redacted>)")
    }
}

impl<'lock> OwnedStore<'lock> {
    /// Open the store. Reachable only with a held lock, by construction.
    pub fn open(root: &Path, lock: &'lock HeldLock) -> Result<Self, StoreError> {
        let path = root.join(STORE_FILE);
        let database = match Database::create(&path) {
            Ok(database) => database,
            Err(redb::DatabaseError::DatabaseAlreadyOpen) => return Err(StoreError::AlreadyOpen),
            // Every other error leaves ownership unknown, which is not
            // ownership. No fallback to read-only and no fresh store: either
            // would turn a corrupt or unexpected file into a silent new one.
            Err(_) => return Err(StoreError::Unavailable),
        };
        Ok(Self {
            database,
            _lock: lock,
        })
    }

    /// The one write helper. Redb's write transactions are already two-phase
    /// and durable on commit; funnelling every write through here means there
    /// is a single place to reason about, rather than one per call site.
    /// Generic over the body's error so a transaction can abort for its own
    /// reasons — a conflict, a replay — without those becoming variants of
    /// `StoreError`, which describes the store and not what a caller was
    /// trying to do.
    ///
    /// A body returning `Err` drops the transaction without committing, so
    /// nothing it wrote is visible. That is the atomicity guarantee in one
    /// line: there is no partial state to clean up because there is no
    /// partial state.
    pub fn write<T, E>(
        &self,
        body: impl FnOnce(&redb::WriteTransaction) -> Result<T, E>,
    ) -> Result<T, E>
    where
        E: From<StoreError>,
    {
        let transaction = self
            .database
            .begin_write()
            .map_err(|_| E::from(StoreError::Unavailable))?;
        let value = body(&transaction)?;
        transaction.commit().map_err(|_| StoreError::Unavailable)?;
        Ok(value)
    }

    pub fn read<T, E>(
        &self,
        body: impl FnOnce(&redb::ReadTransaction) -> Result<T, E>,
    ) -> Result<T, E>
    where
        E: From<StoreError>,
    {
        let transaction = self
            .database
            .begin_read()
            .map_err(|_| E::from(StoreError::Unavailable))?;
        body(&transaction)
    }
}

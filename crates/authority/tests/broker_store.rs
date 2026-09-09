//! Opening the store, and the ordering that makes the broker its sole writer.
//!
//! Redb refuses a second open of the same file within a process, which is a
//! real guarantee and not the one at issue here. What these tests are about is
//! that the store cannot be reached without the process lock — so a second
//! *process*, or a caller that never went through the broker, is stopped by
//! the lock rather than by luck — and that failures leave ownership unknown
//! instead of silently creating a fresh database.

#![cfg(feature = "owner-effect-fixture")]

use sovereign_authority::broker::process_lock::acquire;
use sovereign_authority::broker::store::{OwnedStore, StoreError, STORE_FILE};

#[test]
fn a_locked_root_yields_a_store() {
    let dir = tempfile::tempdir().unwrap();
    let lock = acquire(dir.path()).unwrap();
    let store = OwnedStore::open(dir.path(), &lock).expect("a locked root must open");
    assert!(
        dir.path().join(STORE_FILE).is_file(),
        "the store must be a real file beside the lock"
    );
    // Value-free rendering: a database's own Debug prints its path.
    assert_eq!(format!("{store:?}"), "OwnedStore(<redacted>)");
}

/// The ordering is enforced by the type, not by a comment. `open` takes a
/// `&HeldLock` and there is no other constructor, so this test is really a
/// statement about what the API permits: without a lock there is nothing to
/// pass, and the code below would not compile.
#[test]
fn the_store_cannot_be_opened_without_a_held_lock() {
    let dir = tempfile::tempdir().unwrap();
    // Deliberately not compiled, and deliberately written out so a reader can
    // see what is being claimed:
    //
    //     OwnedStore::open(dir.path(), /* no lock exists to pass */);
    //
    // What is testable at runtime is the consequence: nothing created the
    // store, because nothing could.
    assert!(!dir.path().join(STORE_FILE).exists());
}

/// Two opens in one process: redb's own guarantee, asserted so a later change
/// that swapped the backend cannot quietly lose it.
#[test]
fn a_second_open_in_one_process_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let lock = acquire(dir.path()).unwrap();
    let _first = OwnedStore::open(dir.path(), &lock).unwrap();
    assert!(
        matches!(
            OwnedStore::open(dir.path(), &lock),
            Err(StoreError::AlreadyOpen)
        ),
        "redb must refuse a second open of the same database"
    );
}

/// A file that is not a redb database must not be replaced by a fresh one.
/// Creating a new store over unrecognised bytes is how a corrupt or
/// unexpected file becomes an empty, apparently healthy one.
#[test]
fn an_unrecognised_file_is_unavailable_not_recreated() {
    let dir = tempfile::tempdir().unwrap();
    let lock = acquire(dir.path()).unwrap();
    let path = dir.path().join(STORE_FILE);
    std::fs::write(&path, b"this is not a database").unwrap();

    assert!(
        matches!(
            OwnedStore::open(dir.path(), &lock),
            Err(StoreError::Unavailable)
        ),
        "an unrecognised file must fail closed"
    );
    assert_eq!(
        std::fs::read(&path).unwrap(),
        b"this is not a database",
        "the original file must be left exactly as it was"
    );
}

/// An unusable root fails closed rather than being reported as a second
/// broker — the same distinction the lock makes, kept at this layer too.
#[test]
fn an_unavailable_root_fails_closed() {
    let dir = tempfile::tempdir().unwrap();
    let store_dir = dir.path().join("store");
    std::fs::create_dir(&store_dir).unwrap();
    let lock = acquire(&store_dir).unwrap();
    let blocked = sovereign_fault_testing::BlockedPath::block(&store_dir).unwrap();

    match OwnedStore::open(blocked.path(), &lock) {
        Err(StoreError::Unavailable) => {}
        Err(StoreError::AlreadyOpen) => panic!("an unusable root was reported as already open"),
        Ok(_) => panic!("an unusable root opened a store"),
    }
}

/// Writes commit, and survive being reopened by a later broker. This is the
/// only reason any of the ordering above matters.
#[test]
fn a_committed_write_survives_reopening() {
    const TABLE: redb::TableDefinition<&str, &str> = redb::TableDefinition::new("fixture");
    let dir = tempfile::tempdir().unwrap();

    {
        let lock = acquire(dir.path()).unwrap();
        let store = OwnedStore::open(dir.path(), &lock).unwrap();
        store
            .write(
                |transaction: &redb::WriteTransaction| -> Result<(), StoreError> {
                    let mut table = transaction
                        .open_table(TABLE)
                        .map_err(|_| StoreError::Unavailable)?;
                    table
                        .insert("key", "value")
                        .map_err(|_| StoreError::Unavailable)?;
                    Ok(())
                },
            )
            .expect("the write must commit");
    }

    let lock = acquire(dir.path()).unwrap();
    let store = OwnedStore::open(dir.path(), &lock).unwrap();
    let value = store
        .read(
            |transaction: &redb::ReadTransaction| -> Result<Option<String>, StoreError> {
                let table = transaction
                    .open_table(TABLE)
                    .map_err(|_| StoreError::Unavailable)?;
                let found = table
                    .get("key")
                    .map_err(|_| StoreError::Unavailable)?
                    .map(|entry| entry.value().to_owned());
                Ok(found)
            },
        )
        .unwrap();
    assert_eq!(value.as_deref(), Some("value"));
}

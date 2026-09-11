//! The closed SQLCipher connection factory.
//!
//! One function opens a database, and it does three things in a fixed order:
//! open without following symlinks or accepting URIs, key the connection
//! through the FFI boundary before any statement runs, then prove the key is
//! right by reading the schema. A wrong key does not fail at step two —
//! SQLCipher accepts any key there — it fails at the first page read, which
//! is why step three exists and why a connection is only returned after it.
//!
//! What this module does *not* offer is as deliberate as what it does: no
//! method runs arbitrary SQL, no method returns the raw handle, and the
//! connection is neither `Send` nor `Sync`, so it stays on the thread that
//! owns the key material.

use super::ffi;
use super::secret::DbKey;
use rusqlite::OpenFlags;
use std::marker::PhantomData;
use std::path::Path;

/// How a connection may be opened. The internal create mode is not reachable
/// from outside the engine; the plan's compile-fail fixtures prove that once
/// the public API exists.
// The three names are fixed by the Program 1A plan
// (`ConnectionMode::{ReadWriteCreateInternal,ReadWrite,ReadOnlyRecovery}`),
// and the compile-fail fixtures that prove create mode is unreachable name them
// too. Sharing a prefix is the lint's complaint, not a defect in the names.
#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConnectionMode {
    /// Create a new, empty, encrypted container. Engine-internal only.
    ReadWriteCreateInternal,
    /// Open an existing container for writing.
    ReadWrite,
    /// Open an existing container read-only, for recovery and verification.
    ReadOnlyRecovery,
}

impl ConnectionMode {
    fn flags(self) -> OpenFlags {
        // No `SQLITE_OPEN_URI` in any mode: a path is a path, and a URI would
        // let a caller smuggle `?vfs=` or other options through it.
        let hardening = OpenFlags::SQLITE_OPEN_NOFOLLOW
            | OpenFlags::SQLITE_OPEN_NO_MUTEX
            | OpenFlags::SQLITE_OPEN_EXRESCODE;
        match self {
            ConnectionMode::ReadWriteCreateInternal => {
                hardening | OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE
            }
            ConnectionMode::ReadWrite => hardening | OpenFlags::SQLITE_OPEN_READ_WRITE,
            ConnectionMode::ReadOnlyRecovery => hardening | OpenFlags::SQLITE_OPEN_READ_ONLY,
        }
    }
}

/// Why a database did not open. Every variant is value-free: none carries a
/// path, a key, a schema name, or a message from the library, because each of
/// those can hold something the caller should not be shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OpenError {
    /// The file could not be opened at all (missing, a symlink, unreadable).
    Unopenable,
    /// SQLCipher refused the key outright.
    KeyRejected,
    /// The key was accepted but the first page read failed: a wrong key, or
    /// a file that is not an encrypted database. SQLCipher cannot tell these
    /// apart, and saying which would itself be a leak.
    WrongKeyOrNotADatabase,
}

/// The cipher's own integrity check found pages it could not authenticate.
/// Carries a count, never the pages or their contents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IntegrityFailure {
    /// `cipher_integrity_check` reported this many failures.
    Pages(usize),
    /// The check itself could not run to completion.
    Unreadable,
}

/// An open, keyed, verified connection.
pub(crate) struct HardenedConnection {
    connection: rusqlite::Connection,
    /// `*const ()` is neither `Send` nor `Sync`, which pins this connection
    /// to the thread that opened it.
    _single_thread: PhantomData<*const ()>,
}

/// Open a SQLCipher database under the fixed profile.
pub(crate) fn open_sqlcipher(
    path: &Path,
    key: &DbKey,
    mode: ConnectionMode,
) -> Result<HardenedConnection, OpenError> {
    let connection = rusqlite::Connection::open_with_flags(path, mode.flags())
        .map_err(|_| OpenError::Unopenable)?;

    // First action after open. The raw token lives only for this statement.
    ffi::key_main_database(&connection, &key.raw()).map_err(|_| OpenError::KeyRejected)?;

    // The key is only proven by reading a page. `sqlite_master` exists in
    // every database, so this touches page 1 and nothing the caller wrote.
    connection
        .query_row("SELECT count(*) FROM sqlite_master", [], |row| {
            row.get::<_, i64>(0)
        })
        .map_err(|_| OpenError::WrongKeyOrNotADatabase)?;

    Ok(HardenedConnection {
        connection,
        _single_thread: PhantomData,
    })
}

impl HardenedConnection {
    /// Authenticate every page against its HMAC. Returns only whether they
    /// all passed and, if not, how many did not.
    pub(crate) fn cipher_integrity_check(&self) -> Result<(), IntegrityFailure> {
        let mut statement = self
            .connection
            .prepare("PRAGMA cipher_integrity_check")
            .map_err(|_| IntegrityFailure::Unreadable)?;
        let failures = statement
            .query_map([], |_| Ok(()))
            .map_err(|_| IntegrityFailure::Unreadable)?
            .count();
        if failures == 0 {
            Ok(())
        } else {
            Err(IntegrityFailure::Pages(failures))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use static_assertions::assert_not_impl_any;
    use std::fs;
    use std::path::PathBuf;

    assert_not_impl_any!(HardenedConnection: Send, Sync);

    /// Distinctive enough that finding it anywhere is proof of a leak, and
    /// not a string SQLite or SQLCipher would ever produce on its own.
    const CANARY: &str = "SFO-PLAINTEXT-CANARY-7f3a9c41";
    const REPLACEMENT_CANARY: &str = "SFO-REPLACEMENT-CANARY-e25d08b6";

    fn key(fill: u8) -> DbKey {
        DbKey::from_bytes([fill; 32])
    }

    /// A temporary directory addressed by its canonical path.
    ///
    /// `SQLITE_OPEN_NOFOLLOW` refuses a symlink in *any* component of the
    /// path, not only the last one — measured, not assumed: on macOS `/var`
    /// is a link to `/private/var`, so every `tempdir()` path is refused as
    /// `SQLITE_CANTOPEN_SYMLINK` (extended code 1550) until it is resolved.
    /// The factory is right to refuse it; the fixture resolves it.
    fn canonical_tempdir() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = fs::canonicalize(dir.path()).expect("canonical tempdir");
        (dir, path)
    }

    /// Every file SQLite may have written beside the database: the database
    /// itself and its rollback journal, write-ahead log and shared memory.
    fn every_file_beside(db: &Path) -> Vec<(PathBuf, Vec<u8>)> {
        ["", "-journal", "-wal", "-shm"]
            .iter()
            .map(|suffix| PathBuf::from(format!("{}{suffix}", db.display())))
            .filter_map(|path| fs::read(&path).ok().map(|bytes| (path, bytes)))
            .collect()
    }

    fn contains(haystack: &[u8], needle: &[u8]) -> bool {
        haystack
            .windows(needle.len())
            .any(|window| window == needle)
    }

    /// Create a database holding the canary, through the factory, and close
    /// it. Tests reach the inner connection directly because they sit in this
    /// module; production code has no method that runs arbitrary SQL.
    fn create_with_canary(db: &Path, fill: u8) {
        let opened = open_sqlcipher(db, &key(fill), ConnectionMode::ReadWriteCreateInternal)
            .expect("create an encrypted container");
        opened
            .connection
            .execute_batch(&format!(
                "CREATE TABLE ledger_entries (note TEXT);
                 INSERT INTO ledger_entries VALUES ('{CANARY}');"
            ))
            .expect("write the canary");
    }

    /// The plaintext is nowhere on disk — not in the database, and not in the
    /// rollback journal while a transaction that replaces it is still open.
    /// The key is nowhere on disk either, in any of its three forms.
    ///
    /// Scanning raw bytes is the independent evidence: it does not ask the
    /// engine whether it encrypted anything, it looks.
    #[test]
    fn database_header_and_journal_do_not_contain_plaintext_canary() {
        let (_dir, root) = canonical_tempdir();
        let db = root.join("vault.db");
        create_with_canary(&db, 0xab);

        // At rest.
        let files = every_file_beside(&db);
        assert!(!files.is_empty(), "the database was not written at all");
        for (path, bytes) in &files {
            assert!(
                !contains(bytes, CANARY.as_bytes()),
                "plaintext canary found in {}",
                path.display()
            );
            // Plain SQLite files begin with this header; an encrypted file
            // must not, or the "encryption" was never applied.
            assert!(
                !bytes.starts_with(b"SQLite format 3\0"),
                "{} carries the plaintext SQLite header",
                path.display()
            );
        }

        // Mid-transaction: the rollback journal holds the page being replaced,
        // which is exactly where an old value would leak if the journal were
        // not encrypted too.
        let opened = open_sqlcipher(&db, &key(0xab), ConnectionMode::ReadWrite).expect("reopen");
        opened
            .connection
            .execute_batch(&format!(
                "BEGIN IMMEDIATE;
                 UPDATE ledger_entries SET note = '{REPLACEMENT_CANARY}';"
            ))
            .expect("open a write transaction");
        let during = every_file_beside(&db);
        assert!(
            during
                .iter()
                .any(|(path, _)| path.to_string_lossy().ends_with("-journal")),
            "no rollback journal exists mid-transaction, so this scan would prove nothing"
        );
        for (path, bytes) in &during {
            for canary in [CANARY, REPLACEMENT_CANARY] {
                assert!(
                    !contains(bytes, canary.as_bytes()),
                    "{canary} found in {} while a transaction was open",
                    path.display()
                );
            }
        }
        opened.connection.execute_batch("COMMIT;").expect("commit");
        drop(opened);

        // The key, in every form it takes: raw bytes, lowercase hex, and the
        // full SQLCipher token.
        let raw_key = [0xabu8; 32];
        let hex = "ab".repeat(32);
        let token = format!("x'{hex}'");
        for (path, bytes) in every_file_beside(&db) {
            assert!(
                !contains(&bytes, &raw_key),
                "raw DBK found in {}",
                path.display()
            );
            assert!(
                !contains(&bytes, hex.as_bytes()),
                "hex DBK found in {}",
                path.display()
            );
            assert!(
                !contains(&bytes, token.as_bytes()),
                "DBK token found in {}",
                path.display()
            );
        }
    }

    /// A wrong key does not open the database, does not reveal what is in it,
    /// and does not change it.
    #[test]
    fn wrong_dbk_fails_without_schema_or_plaintext() {
        let (_dir, root) = canonical_tempdir();
        let db = root.join("vault.db");
        create_with_canary(&db, 0x11);
        let before = fs::read(&db).expect("read before");

        let result = open_sqlcipher(&db, &key(0x22), ConnectionMode::ReadWrite);
        assert!(
            matches!(result, Err(OpenError::WrongKeyOrNotADatabase)),
            "a wrong key opened the database"
        );

        // The error is the only thing the caller gets back. It must carry
        // neither the table name nor the plaintext.
        let rendered = format!("{:?}", result.err());
        assert!(
            !rendered.contains("ledger_entries"),
            "the error names the schema: {rendered}"
        );
        assert!(
            !rendered.contains(CANARY),
            "the error carries plaintext: {rendered}"
        );

        // Failing is not repairing: the file is byte-for-byte what it was.
        assert_eq!(
            fs::read(&db).expect("read after"),
            before,
            "a failed open changed the file"
        );

        // And the right key still works, so the failure was the key and not
        // a database the wrong-key attempt had damaged.
        assert!(open_sqlcipher(&db, &key(0x11), ConnectionMode::ReadWrite).is_ok());
    }

    /// One flipped bit in page 2 is caught by the cipher's authentication,
    /// not silently decrypted into a wrong value.
    ///
    /// Offset 4224 is 128 bytes into page 2 at the 4096-byte page size this
    /// crate pins — past page 1's salt and header, inside encrypted content.
    #[test]
    fn page_2_ciphertext_bitflip_is_detected_cryptographically() {
        let (_dir, root) = canonical_tempdir();
        let db = root.join("vault.db");
        {
            let opened = open_sqlcipher(&db, &key(0x5a), ConnectionMode::ReadWriteCreateInternal)
                .expect("create");
            // Enough rows to occupy well past page 2.
            opened
                .connection
                .execute_batch(
                    "CREATE TABLE filler (body BLOB);
                     INSERT INTO filler SELECT randomblob(900) FROM
                       (WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i+1 FROM n WHERE i < 24)
                        SELECT i FROM n);",
                )
                .expect("fill pages");
        }

        let mut bytes = fs::read(&db).expect("read");
        assert!(
            bytes.len() >= 8192 && bytes.len() % 4096 == 0,
            "the database is {} bytes, not at least two whole pages",
            bytes.len()
        );

        // Untouched, it passes — so a failure below is the flip, not the setup.
        let intact = open_sqlcipher(&db, &key(0x5a), ConnectionMode::ReadOnlyRecovery)
            .expect("reopen the intact file");
        assert_eq!(
            intact.cipher_integrity_check(),
            Ok(()),
            "the untouched file failed its own check"
        );
        drop(intact);

        bytes[4224] ^= 0x01;
        fs::write(&db, &bytes).expect("write the flipped file");

        // The key is right and page 1 is untouched, so the open succeeds; the
        // damage is on page 2 and must be found there.
        let opened = open_sqlcipher(&db, &key(0x5a), ConnectionMode::ReadOnlyRecovery)
            .expect("page 1 is intact, so opening still succeeds");
        assert!(
            matches!(opened.cipher_integrity_check(), Err(IntegrityFailure::Pages(n)) if n > 0),
            "a flipped ciphertext bit passed the cipher integrity check"
        );
    }

    /// A path that passes through a symlink is refused, and refused before
    /// anything is created at the other end.
    ///
    /// Built from an explicit link rather than relying on how a platform lays
    /// out its temp directory, so it means the same thing on macOS and Linux.
    #[test]
    fn a_path_through_a_symlink_is_refused_and_creates_nothing() {
        let (_dir, root) = canonical_tempdir();
        let real = root.join("real");
        fs::create_dir(&real).expect("real dir");
        let link = root.join("link");
        std::os::unix::fs::symlink(&real, &link).expect("symlink");

        let result = open_sqlcipher(
            &link.join("vault.db"),
            &key(0x33),
            ConnectionMode::ReadWriteCreateInternal,
        );
        assert!(
            matches!(result, Err(OpenError::Unopenable)),
            "a path through a symlink was opened"
        );
        assert!(
            fs::read_dir(&real).expect("list real").next().is_none(),
            "the refused open still created a file behind the link"
        );

        // The same directory, addressed without the link, is fine: the refusal
        // was the link and not the location.
        assert!(open_sqlcipher(
            &real.join("vault.db"),
            &key(0x33),
            ConnectionMode::ReadWriteCreateInternal
        )
        .is_ok());
    }
}

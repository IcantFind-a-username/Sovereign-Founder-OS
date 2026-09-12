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

use super::ffi::{self, HardeningFailure};
use super::process::CryptoProcessOwner;
use super::secret::DbKey;
use rusqlite::config::DbConfig;
use rusqlite::limits::Limit;
use rusqlite::OpenFlags;
use std::marker::PhantomData;
use std::path::Path;

/// The runtime limits RFC 0005 Program 1A fixes, in the order the plan lists
/// them. Each is set before the first page is read and then read back from
/// the C library, because SQLite silently clamps a limit to its compile-time
/// maximum and a clamped value would otherwise pass unnoticed.
pub(crate) const FIXED_LIMITS: [(Limit, i32); 10] = [
    (Limit::SQLITE_LIMIT_SQL_LENGTH, 64 * 1024),
    (Limit::SQLITE_LIMIT_LENGTH, 16 * 1024 * 1024),
    (Limit::SQLITE_LIMIT_COLUMN, 128),
    (Limit::SQLITE_LIMIT_EXPR_DEPTH, 32),
    (Limit::SQLITE_LIMIT_COMPOUND_SELECT, 16),
    (Limit::SQLITE_LIMIT_VARIABLE_NUMBER, 999),
    (Limit::SQLITE_LIMIT_TRIGGER_DEPTH, 0),
    (Limit::SQLITE_LIMIT_ATTACHED, 0),
    (Limit::SQLITE_LIMIT_LIKE_PATTERN_LENGTH, 256),
    (Limit::SQLITE_LIMIT_WORKER_THREADS, 0),
];

/// Connection safety switches the plan requires, with the value each must
/// hold. `DEFENSIVE` stops `writable_schema` from being used to corrupt the
/// schema; `TRUSTED_SCHEMA` off stops SQL functions from running inside
/// schema objects; the two `DQS` switches stop a double-quoted identifier
/// from being silently reinterpreted as a string literal.
const SAFETY_SWITCHES: [(DbConfig, bool); 4] = [
    (DbConfig::SQLITE_DBCONFIG_DEFENSIVE, true),
    (DbConfig::SQLITE_DBCONFIG_TRUSTED_SCHEMA, false),
    (DbConfig::SQLITE_DBCONFIG_DQS_DML, false),
    (DbConfig::SQLITE_DBCONFIG_DQS_DDL, false),
];

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
    /// A required setting did not take: native-code loading could not be
    /// shown off, a safety switch did not hold, or a limit read back as
    /// something other than the value set. The connection is refused rather
    /// than returned half-hardened.
    ProfileNotApplied,
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
    owner: &CryptoProcessOwner,
    path: &Path,
    key: &DbKey,
    mode: ConnectionMode,
) -> Result<HardenedConnection, OpenError> {
    // Open, key first, and switch off native-code loading — one audited
    // unsafe entry point. The raw token lives only for this call.
    let connection = ffi::open_keyed_hardened(owner, path, mode.flags().bits(), &key.raw())
        .map_err(|failure| match failure {
            HardeningFailure::PathNotRepresentable | HardeningFailure::Unopenable => {
                OpenError::Unopenable
            }
            HardeningFailure::KeyRejected => OpenError::KeyRejected,
            HardeningFailure::ExtensionLoadingNotDisabled
            | HardeningFailure::ThreadingModeNotAdmitted => OpenError::ProfileNotApplied,
        })?;

    // The remaining no-page settings, each confirmed by what SQLite reports
    // back rather than by the call having returned.
    for (switch, required) in SAFETY_SWITCHES {
        let held = connection
            .set_db_config(switch, required)
            .map_err(|_| OpenError::ProfileNotApplied)?;
        if held != required {
            return Err(OpenError::ProfileNotApplied);
        }
    }
    for (limit, value) in FIXED_LIMITS {
        connection
            .set_limit(limit, value)
            .map_err(|_| OpenError::ProfileNotApplied)?;
        if connection
            .limit(limit)
            .map_err(|_| OpenError::ProfileNotApplied)?
            != value
        {
            return Err(OpenError::ProfileNotApplied);
        }
    }

    // Only now is a page read, and the read is what proves the key.
    // `sqlite_schema` exists in every database, so this touches page 1 and
    // nothing the caller wrote.
    connection
        .query_row("SELECT count(*) FROM sqlite_schema", [], |row| {
            row.get::<_, i64>(0)
        })
        .map_err(|_| OpenError::WrongKeyOrNotADatabase)?;

    Ok(HardenedConnection {
        connection,
        _single_thread: PhantomData,
    })
}

impl HardenedConnection {
    /// Which crypto provider SQLCipher is actually using on this connection,
    /// and its version — read from the library rather than inferred from the
    /// build features.
    ///
    /// SQLCipher answers these only once a connection is keyed, which is why
    /// this is a method on an opened connection. Both values are the
    /// library's own strings; neither is derived from the key.
    pub(crate) fn cipher_profile(&self) -> Option<(String, String)> {
        let provider = self
            .connection
            .query_row("PRAGMA cipher_provider", [], |row| row.get::<_, String>(0))
            .ok()?;
        let version = self
            .connection
            .query_row("PRAGMA cipher_provider_version", [], |row| {
                row.get::<_, String>(0)
            })
            .ok()?;
        Some((provider, version))
    }

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

    /// Every open needs the process bootstrap; the tests run in one process,
    /// which is exactly what the owner represents.
    fn owner() -> &'static CryptoProcessOwner {
        super::super::process::bootstrap_crypto_process().expect("OpenSSL initialises")
    }

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
        let opened = open_sqlcipher(
            owner(),
            db,
            &key(fill),
            ConnectionMode::ReadWriteCreateInternal,
        )
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
        let opened =
            open_sqlcipher(owner(), &db, &key(0xab), ConnectionMode::ReadWrite).expect("reopen");
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

        let result = open_sqlcipher(owner(), &db, &key(0x22), ConnectionMode::ReadWrite);
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
        assert!(open_sqlcipher(owner(), &db, &key(0x11), ConnectionMode::ReadWrite).is_ok());
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
            let opened = open_sqlcipher(
                owner(),
                &db,
                &key(0x5a),
                ConnectionMode::ReadWriteCreateInternal,
            )
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
        let intact = open_sqlcipher(owner(), &db, &key(0x5a), ConnectionMode::ReadOnlyRecovery)
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
        let opened = open_sqlcipher(owner(), &db, &key(0x5a), ConnectionMode::ReadOnlyRecovery)
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
            owner(),
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
            owner(),
            &real.join("vault.db"),
            &key(0x33),
            ConnectionMode::ReadWriteCreateInternal
        )
        .is_ok());
    }

    /// Prepare *and step* a statement, returning SQLite's refusal if any.
    ///
    /// Stepping matters. Some limits are enforced when a statement compiles —
    /// expression depth, SQL length, columns, compound terms, variable numbers
    /// — and others only when it runs: LIKE patterns, value length, ATTACH,
    /// extension loading, triggers. A helper that only prepared would report
    /// the runtime ones as passing whatever their size, which is exactly the
    /// false green a first probe of this code produced.
    fn run(connection: &rusqlite::Connection, sql: &str) -> Result<(), String> {
        let mut statement = connection.prepare(sql).map_err(|error| error.to_string())?;
        let mut rows = statement.raw_query();
        rows.next().map(|_| ()).map_err(|error| error.to_string())
    }

    /// Refused, and refused for the stated reason. A boundary-plus-one case
    /// that failed on a typo in the constructed SQL would prove nothing, so
    /// every refusal is matched against the message of the limit it tests.
    fn assert_refused(connection: &rusqlite::Connection, sql: &str, because: &str) {
        match run(connection, sql) {
            Ok(()) => panic!("accepted past the limit: {} bytes of SQL", sql.len()),
            Err(message) => assert!(
                message.contains(because),
                "refused, but not by the limit under test — expected {because:?}, got {message:?}"
            ),
        }
    }

    fn assert_accepted(connection: &rusqlite::Connection, sql: &str, what: &str) {
        if let Err(message) = run(connection, sql) {
            panic!("{what} at its boundary was refused: {message}");
        }
    }

    /// Every fixed limit, read back from the C library and then exercised at
    /// its boundary and one past it.
    #[test]
    fn oversized_values_and_sql_fail_at_fixed_limits() {
        let (_dir, root) = canonical_tempdir();
        let opened = open_sqlcipher(
            owner(),
            &root.join("vault.db"),
            &key(0x44),
            ConnectionMode::ReadWriteCreateInternal,
        )
        .expect("open");
        let connection = &opened.connection;

        // What SQLite reports, not what was requested: a limit clamped to a
        // compile-time maximum would show up here.
        for (limit, value) in FIXED_LIMITS {
            assert_eq!(
                connection.limit(limit).expect("read a limit back"),
                value,
                "{limit:?} is not the fixed value"
            );
        }

        // SQL text: 64 KiB. The statement is padded inside a comment so its
        // length can be set to the byte.
        let sql_of = |length: usize| {
            let (head, tail) = ("SELECT 1 /*", "*/");
            format!(
                "{head}{}{tail}",
                "x".repeat(length - head.len() - tail.len())
            )
        };
        assert_accepted(connection, &sql_of(64 * 1024), "SQL of 64 KiB");
        assert_refused(connection, &sql_of(64 * 1024 + 1), "statement too long");

        // One value: 16 MiB.
        assert_accepted(connection, "SELECT zeroblob(16777216)", "a 16 MiB value");
        assert_refused(
            connection,
            "SELECT zeroblob(16777217)",
            "string or blob too big",
        );

        // Columns: 128.
        let columns = |count: usize| {
            let list: Vec<String> = (0..count).map(|index| index.to_string()).collect();
            format!("SELECT {}", list.join(","))
        };
        assert_accepted(connection, &columns(128), "128 columns");
        assert_refused(connection, &columns(129), "too many columns");

        // Expression depth: 32. A literal is depth one and each unary minus
        // adds one, so 31 nested minuses is depth 32 exactly.
        let nested = |depth: usize| format!("SELECT {}1{}", "-(".repeat(depth), ")".repeat(depth));
        assert_accepted(connection, &nested(31), "expression depth 32");
        assert_refused(connection, &nested(32), "Expression tree is too large");

        // Compound terms: 16.
        let compound = |terms: usize| vec!["SELECT 1"; terms].join(" UNION ALL ");
        assert_accepted(connection, &compound(16), "16 compound terms");
        assert_refused(
            connection,
            &compound(17),
            "too many terms in compound SELECT",
        );

        // Variables: 999.
        assert_accepted(connection, "SELECT ?999", "variable ?999");
        assert_refused(
            connection,
            "SELECT ?1000",
            "variable number must be between ?1 and ?999",
        );

        // LIKE pattern: 256 bytes.
        let like = |length: usize| format!("SELECT 'a' LIKE '{}'", "a".repeat(length));
        assert_accepted(connection, &like(256), "a 256-byte LIKE pattern");
        assert_refused(connection, &like(257), "LIKE or GLOB pattern too complex");

        // Attached databases: 0. The boundary is the connection as it stands;
        // one attachment is past it.
        assert_refused(
            connection,
            "ATTACH DATABASE ':memory:' AS other",
            "too many attached databases",
        );

        // Trigger depth: 0, so no trigger may fire at all. The boundary is a
        // write that fires none; one past it is a write that fires one.
        connection
            .execute_batch(
                "CREATE TABLE fired (x);
                 CREATE TABLE plain (x);
                 CREATE TRIGGER on_fired AFTER INSERT ON fired
                   BEGIN INSERT INTO plain VALUES (new.x); END;",
            )
            .expect("create the tables and the trigger");
        assert_accepted(
            connection,
            "INSERT INTO plain VALUES (1)",
            "a write that fires no trigger",
        );
        assert_refused(
            connection,
            "INSERT INTO fired VALUES (1)",
            "too many levels of trigger recursion",
        );

        // Worker threads: 0. There is no statement that behaves differently at
        // one past it — the limit governs how many helper threads a sort may
        // start — so the readback above is the whole of the evidence.
    }

    /// Every route by which a connection could reach outside its own file is
    /// shut: native code, another database, and the schema itself.
    ///
    /// "No dynamic SQL path" in the plan is a property of this module's API —
    /// `HardenedConnection` has no method that takes SQL text — and it is the
    /// source-closure gate's to prove once the public boundary exists, not a
    /// runtime check. What runs here is everything that can be observed.
    #[test]
    fn extensions_attach_writable_schema_and_dynamic_sql_are_denied() {
        let (_dir, root) = canonical_tempdir();
        let opened = open_sqlcipher(
            owner(),
            &root.join("vault.db"),
            &key(0x55),
            ConnectionMode::ReadWriteCreateInternal,
        )
        .expect("open");
        let connection = &opened.connection;

        // Native code. "not authorized" means the call was refused before any
        // load was attempted; with loading enabled SQLite instead calls dlopen,
        // fails on the missing file, and reports every path it searched —
        // a different message, and a disclosure of its own.
        //
        // What this proves is that the SQL function is refused, not which of
        // the two switches refused it: either one alone is enough, and the
        // factory reads route two back and will not return a connection with it
        // on. Route one's disable is pinned in source by `tests/ast_gate.rs`,
        // which is the only place its presence can be shown.
        assert_refused(
            connection,
            "SELECT load_extension('/no/such/library')",
            "not authorized",
        );

        // Another database.
        assert_refused(
            connection,
            "ATTACH DATABASE ':memory:' AS elsewhere",
            "too many attached databases",
        );

        // The schema. Defensive mode is what makes `writable_schema` harmless:
        // the pragma may be accepted, but the schema must not change. That is
        // checked on the schema itself rather than on an error message.
        connection
            .execute_batch("CREATE TABLE records (x);")
            .expect("create a table");
        let before: String = connection
            .query_row(
                "SELECT sql FROM sqlite_schema WHERE name = 'records'",
                [],
                |row| row.get(0),
            )
            .expect("read the schema");
        let _ = connection.execute_batch("PRAGMA writable_schema = ON;");
        let tamper = connection.execute_batch(
            "UPDATE sqlite_schema SET sql = 'CREATE TABLE records (x, smuggled)' WHERE name = 'records';",
        );
        assert!(tamper.is_err(), "the schema table accepted a write");
        let after: String = connection
            .query_row(
                "SELECT sql FROM sqlite_schema WHERE name = 'records'",
                [],
                |row| row.get(0),
            )
            .expect("read the schema again");
        assert_eq!(after, before, "the schema changed under writable_schema");
    }
}

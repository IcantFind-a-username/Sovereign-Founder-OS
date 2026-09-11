//! The one place this crate authors `unsafe` code against the C library.
//!
//! RFC 0005 Program 1A allows exactly two project-authored unsafe FFI
//! boundary files, `ffi.rs` and `process.rs`, and the source-closure gate
//! enforces that list. This is the first of them. It does one thing: hand the
//! database key to SQLCipher as bytes.
//!
//! Why not `PRAGMA key`? Because a pragma is SQL text. The key would be
//! formatted into a statement, and SQL text is what tracing, statement logs
//! and error messages are made of. `sqlite3_key_v2` takes a pointer and a
//! length, so the key never exists as SQL at all.
//!
//! The declaration is written here rather than taken from the bindings
//! because the bindings do not have it: `libsqlite3-sys` generates them from
//! stock `sqlite3.h`, where SQLCipher's key functions are not declared. The
//! symbol is in the linked library — `_sqlite3_key_v2` is exported from the
//! engine binary — and this declaration is its only Rust-side name.

use super::secret::RawSqlcipherKey;
use rusqlite::ffi::{sqlite3, SQLITE_OK};
use std::os::raw::{c_char, c_int, c_void};

extern "C" {
    /// SQLCipher: key the named attached database. `p_key` is read for
    /// exactly `n_key` bytes; it is not a C string.
    fn sqlite3_key_v2(
        db: *mut sqlite3,
        z_db_name: *const c_char,
        p_key: *const c_void,
        n_key: c_int,
    ) -> c_int;
}

/// The SQLite result code a failed keying returned. Value-free on purpose:
/// it carries no part of the key and no text from the library.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct KeyRejected(pub(crate) c_int);

/// Key the `main` database of an open connection.
///
/// This must be the first thing done to a connection after it opens, before
/// any statement runs: SQLCipher derives the page cipher from this key, and a
/// read before keying would treat an encrypted file as plain SQLite.
pub(crate) fn key_main_database(
    connection: &rusqlite::Connection,
    key: &RawSqlcipherKey,
) -> Result<(), KeyRejected> {
    let bytes = key.token_bytes();
    let length = c_int::try_from(bytes.len()).expect("a 67-byte token fits a c_int");
    // SAFETY: `handle()` returns the live `sqlite3*` owned by `connection`,
    // which outlives this call because it is borrowed for its duration.
    // `c"main"` is a static NUL-terminated string. `bytes` points to exactly
    // `length` readable bytes for the duration of the call, and SQLCipher
    // copies the key into its own codec context rather than retaining the
    // pointer.
    let code = unsafe {
        sqlite3_key_v2(
            connection.handle(),
            c"main".as_ptr(),
            bytes.as_ptr().cast::<c_void>(),
            length,
        )
    };
    if code == SQLITE_OK {
        Ok(())
    } else {
        Err(KeyRejected(code))
    }
}

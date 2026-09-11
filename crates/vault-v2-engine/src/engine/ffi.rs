//! The one place this crate authors `unsafe` code against the C library.
//!
//! RFC 0005 Program 1A allows exactly two project-authored unsafe FFI
//! boundary files, `ffi.rs` and `process.rs`, and the source-closure gate
//! enforces that list. This is the first. It has one entry point, and the
//! plan is specific about what that entry point owns: it opens the database,
//! keys it as the very first operation, and turns off both routes by which
//! SQLite can load native code — reading the second one back to prove it.
//!
//! Why open here rather than through rusqlite's safe `open_with_flags`? The
//! first version did exactly that, and it was a departure from the plan: the
//! extension-loading off switch `SQLITE_DBCONFIG_ENABLE_LOAD_EXTENSION` (1005)
//! is absent from rusqlite's safe `DbConfig` — commented out in rusqlite's own
//! source — and exposing it would mean enabling rusqlite's forbidden
//! `load_extension` feature. So it has to be set through the raw handle, and
//! the plan permits no second raw-handle shim. Owning the whole sequence here
//! is what keeps the unsafe surface to one function.
//!
//! Why the key goes in as bytes rather than `PRAGMA key`: a pragma is SQL
//! text, and SQL text is what tracing, statement logs and error messages are
//! made of. `sqlite3_key_v2` takes a pointer and a length.
//!
//! `sqlite3_key_v2` is declared here because the generated bindings do not
//! have it — `libsqlite3-sys` builds them from stock `sqlite3.h`, where
//! SQLCipher's key functions are not declared. Everything else comes from the
//! bindings.

use super::secret::RawSqlcipherKey;
use rusqlite::ffi::{
    sqlite3, sqlite3_close, sqlite3_db_config, sqlite3_enable_load_extension, sqlite3_open_v2,
    SQLITE_DBCONFIG_ENABLE_LOAD_EXTENSION, SQLITE_OK,
};
use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_void};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::ptr;

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

/// Why the hardened open failed. Value-free: no path, no key, no text from
/// the library.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HardeningFailure {
    /// The path contains a NUL byte and cannot be handed to C.
    PathNotRepresentable,
    /// `sqlite3_open_v2` refused the path (missing, a symlink, unreadable).
    Unopenable,
    /// SQLCipher refused the key.
    KeyRejected,
    /// A native-code loading route could not be shown to be off.
    ExtensionLoadingNotDisabled,
}

/// Open, key, and disable native-code loading, in that order, and hand back a
/// connection that owns the handle.
///
/// Nothing touches a page here: the key is set before any statement exists,
/// and the extension switches are connection configuration. The caller
/// applies the remaining no-page settings and only then reads a page.
pub(crate) fn open_keyed_hardened(
    path: &Path,
    flags: c_int,
    key: &RawSqlcipherKey,
) -> Result<rusqlite::Connection, HardeningFailure> {
    let c_path = CString::new(path.as_os_str().as_bytes())
        .map_err(|_| HardeningFailure::PathNotRepresentable)?;
    let token = key.token_bytes();
    let token_len = c_int::try_from(token.len()).expect("a 67-byte token fits a c_int");
    let mut db: *mut sqlite3 = ptr::null_mut();

    // SAFETY: `c_path` and `c"main"` are NUL-terminated and outlive every
    // call that reads them. `db` is written only by `sqlite3_open_v2` and is
    // closed exactly once on every failure path below, before it could be
    // adopted. `sqlite3_close` rather than `_v2`: no statement has been
    // prepared on any of those paths, so the two behave the same, and
    // `_close_v2` is not in the generated bindings — using it would mean a
    // second project-authored symbol declaration for no gain. `token`
    // points to exactly `token_len` readable bytes for the duration of
    // `sqlite3_key_v2`, and SQLCipher copies the key into its own
    // codec context rather than keeping the pointer. `now_enabled` is a live
    // `c_int` for the `sqlite3_db_config` call that writes it, and the
    // variadic arguments match the documented signature for option 1005:
    // `(int onoff, int *pOut)`. On success the handle is adopted by
    // `from_handle_owned`, which closes it on drop, so it is never closed
    // here after that point.
    unsafe {
        if sqlite3_open_v2(c_path.as_ptr(), &mut db, flags, ptr::null()) != SQLITE_OK {
            // `sqlite3_open_v2` can allocate a handle even when it fails, and
            // that handle still has to be released.
            if !db.is_null() {
                sqlite3_close(db);
            }
            return Err(HardeningFailure::Unopenable);
        }

        // The first operation on the new connection.
        if sqlite3_key_v2(
            db,
            c"main".as_ptr(),
            token.as_ptr().cast::<c_void>(),
            token_len,
        ) != SQLITE_OK
        {
            sqlite3_close(db);
            return Err(HardeningFailure::KeyRejected);
        }

        // Route one: the switch behind the `load_extension()` SQL function.
        //
        // It has no readback, and its effect cannot be observed on any
        // connection this factory returns — measured, not assumed: the SQL
        // function is refused if *either* this switch or route two is off,
        // and only loads when both are on. Route two is read back below, so
        // on a returned connection it is always off, and it alone produces the
        // refusal a test sees. This call is therefore defence in depth, and
        // the evidence for it is structural: `tests/ast_gate.rs` pins that it
        // is made, once, with the literal `0`.
        if sqlite3_enable_load_extension(db, 0) != SQLITE_OK {
            sqlite3_close(db);
            return Err(HardeningFailure::ExtensionLoadingNotDisabled);
        }

        // Route two: the C API `sqlite3_load_extension`. Set off, then read
        // the value SQLite reports back rather than trusting the call.
        let mut now_enabled: c_int = -1;
        let set = sqlite3_db_config(
            db,
            SQLITE_DBCONFIG_ENABLE_LOAD_EXTENSION,
            0 as c_int,
            &mut now_enabled as *mut c_int,
        );
        if set != SQLITE_OK || now_enabled != 0 {
            sqlite3_close(db);
            return Err(HardeningFailure::ExtensionLoadingNotDisabled);
        }

        // Adopted: from here the connection owns and closes the handle.
        rusqlite::Connection::from_handle_owned(db).map_err(|_| HardeningFailure::Unopenable)
    }
}

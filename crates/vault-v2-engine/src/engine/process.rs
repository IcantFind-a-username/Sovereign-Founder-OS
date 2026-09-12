//! The process-wide crypto bootstrap: the second and last place this crate
//! authors `unsafe` code against a C library.
//!
//! RFC 0005 Program 1A is a dedicated process, and this is the first thing
//! that process does. OpenSSL will, on first use, read a configuration file
//! named by `OPENSSL_CONF` and load whatever providers and engines it names.
//! That is a reasonable default for a general-purpose program and the wrong
//! one for a process that exists to hold a database key: it lets anything
//! that can set an environment variable for this process choose the code that
//! performs its cryptography. `OPENSSL_INIT_NO_LOAD_CONFIG` initialises the
//! library and skips that file.
//!
//! The plan is specific about how: "the bootstrap declares the exact official
//! C ABI and the pinned `OPENSSL_INIT_NO_LOAD_CONFIG` constant locally because
//! `openssl-sys 0.9.117` does not expose them, calls
//! `OPENSSL_init_crypto(..., NULL)`, and requires return value `1`". Both
//! absences were checked against this build rather than assumed: the constant
//! appears only in the vendored header (`crypto.h`, `OPENSSL_INIT_NO_LOAD_CONFIG
//! 0x00000080L`), and `openssl-sys 0.9.117` declares neither it nor
//! `OPENSSL_init_crypto`. The exact `openssl-sys` dependency is still what
//! supplies the reviewed link and version boundary; this file only calls into
//! what that boundary already links.
//!
//! Why a token type rather than a "did we initialise" flag: a flag is
//! something later code must remember to check. `CryptoProcessOwner` cannot be
//! constructed anywhere else, and `open_sqlcipher` takes a reference to one,
//! so a database open that skipped this bootstrap does not compile.

use std::os::raw::{c_int, c_void};
use std::ptr;
use std::sync::OnceLock;

extern "C" {
    /// OpenSSL 3.x: `int OPENSSL_init_crypto(uint64_t opts, const
    /// OPENSSL_INIT_SETTINGS *settings)`. Declared here because the generated
    /// `openssl-sys` bindings for the pinned version do not carry it.
    fn OPENSSL_init_crypto(opts: u64, settings: *const c_void) -> c_int;
}

/// `OPENSSL_INIT_NO_LOAD_CONFIG` from the vendored `crypto.h`. Pinned here as
/// a literal for the same reason the function is declared here.
const OPENSSL_INIT_NO_LOAD_CONFIG: u64 = 0x0000_0080;

/// Proof that this process initialised OpenSSL itself, with configuration
/// loading off. It carries no data and has no public constructor: the only
/// way to hold one is to have called [`bootstrap_crypto_process`].
pub(crate) struct CryptoProcessOwner {
    /// Blocks construction by struct literal outside this module.
    _process_owned: (),
}

/// Why the bootstrap could not vouch for this process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BootstrapFailure {
    /// `OPENSSL_init_crypto` returned something other than `1`.
    OpenSslInitRefused,
}

/// The one owner of this process, so the initialisation happens once however
/// many times the bootstrap is called.
static OWNER: OnceLock<CryptoProcessOwner> = OnceLock::new();

/// Initialise OpenSSL for this process with configuration loading off, and
/// return the token every database open requires.
///
/// Calling this again returns the same owner: OpenSSL's initialisation is
/// process-wide, so a second call must not be able to produce a second,
/// differently-configured "owner" of it.
pub(crate) fn bootstrap_crypto_process() -> Result<&'static CryptoProcessOwner, BootstrapFailure> {
    if let Some(owner) = OWNER.get() {
        return Ok(owner);
    }

    // SAFETY: `OPENSSL_init_crypto` takes an option bitmask and an optional
    // settings pointer. The mask is the single pinned constant above, and the
    // settings pointer is null, which the OpenSSL API documents as "no
    // settings" — there is nothing for the call to read through it. The call
    // touches no memory this program owns, returns an `int`, and is safe to
    // make from multiple threads: OpenSSL guards its own initialisation, so
    // the race two callers can lose here is a second harmless call, never a
    // second initialisation.
    let initialised = unsafe { OPENSSL_init_crypto(OPENSSL_INIT_NO_LOAD_CONFIG, ptr::null()) };
    if initialised != 1 {
        return Err(BootstrapFailure::OpenSslInitRefused);
    }

    Ok(OWNER.get_or_init(|| CryptoProcessOwner { _process_owned: () }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use static_assertions::assert_not_impl_any;

    // The token is evidence, not data: copying one would let code that never
    // bootstrapped hold "proof" that something else did.
    assert_not_impl_any!(CryptoProcessOwner: Clone, Copy, std::fmt::Debug, std::fmt::Display);

    /// The bootstrap succeeds against the linked OpenSSL, and a second call
    /// hands back the very same owner rather than initialising again.
    #[test]
    fn the_process_has_exactly_one_crypto_owner() {
        let first = bootstrap_crypto_process().expect("OpenSSL initialises with config off");
        let second = bootstrap_crypto_process().expect("a second call is the same owner");
        assert!(
            ptr::eq(first, second),
            "a second bootstrap produced a different owner"
        );
    }
}

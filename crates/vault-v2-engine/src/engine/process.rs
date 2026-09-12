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

    // -----------------------------------------------------------------
    // Fresh-process qualification.
    //
    // OpenSSL's initialisation is process-global, so these cases cannot
    // share a process with each other or with the tests above: whichever
    // ran first would decide the answer for the rest. Each therefore runs
    // in a worker — this same test binary, re-executed for exactly one
    // `#[ignore]`d case — and the parent reads what the worker reports.
    //
    // The evidence is the provider SQLCipher actually used and whether the
    // open succeeded, never a flag a worker set for itself.
    // -----------------------------------------------------------------

    /// A configuration file that activates OpenSSL's `null` provider and
    /// nothing else. Measured against this build before it was written into
    /// a test: with it loaded, keying fails and `cipher_provider` answers
    /// nothing — so it is a real adversary and not a decorative one.
    const HOSTILE_OPENSSL_CONF: &str = "\
openssl_conf = openssl_init

[openssl_init]
providers = provider_sect

[provider_sect]
null = null_sect

[null_sect]
activate = 1
";

    /// `OPENSSL_INIT_LOAD_CONFIG` from the same vendored header as the
    /// constant the bootstrap pins. Test-only: this is the negative control
    /// the plan admits, and the source gate allows exactly this one extra
    /// call because it is inside `#[cfg(test)]`.
    const OPENSSL_INIT_LOAD_CONFIG: u64 = 0x0000_0040;

    /// Do what an unprotected process does on first use: read the
    /// configuration file the environment names and load what it says.
    fn load_configuration_from_the_environment() -> c_int {
        // SAFETY: identical contract to the bootstrap — a pinned option
        // bitmask and a null settings pointer, reading no memory this
        // program owns.
        unsafe { OPENSSL_init_crypto(OPENSSL_INIT_LOAD_CONFIG, ptr::null()) }
    }

    /// Open a database the way the engine does and report what the crypto
    /// provider turned out to be. Shared by both workers so the two differ
    /// in exactly one thing: whether the hostile configuration was loaded
    /// first.
    fn report_profile_of_a_real_open() {
        use super::super::secret::DbKey;
        use super::super::sqlcipher::{open_sqlcipher, ConnectionMode};

        let owner = bootstrap_crypto_process().expect("bootstrap");
        let dir = tempfile::tempdir().expect("tempdir");
        let root = std::fs::canonicalize(dir.path()).expect("canonical tempdir");
        match open_sqlcipher(
            owner,
            &root.join("vault.db"),
            &DbKey::from_bytes([0x5a; 32]),
            ConnectionMode::ReadWriteCreateInternal,
        ) {
            Ok(connection) => {
                let (provider, version) = connection
                    .cipher_profile()
                    .expect("an opened connection reports its provider");
                println!("WORKER opened provider={provider} version={version}");
            }
            Err(error) => println!("WORKER refused {error:?}"),
        }
    }

    #[test]
    #[ignore = "re-executed as a fresh process by its parent test"]
    fn worker_opens_under_a_hostile_environment() {
        report_profile_of_a_real_open();
    }

    #[test]
    #[ignore = "re-executed as a fresh process by its parent test"]
    fn worker_loads_the_hostile_configuration_before_bootstrapping() {
        // The one thing this worker does differently, and it happens first.
        println!(
            "WORKER load_config={}",
            load_configuration_from_the_environment()
        );
        report_profile_of_a_real_open();
    }

    /// Run one `#[ignore]`d worker in a fresh process with the hostile
    /// environment set, and hand back everything it printed.
    fn run_worker(name: &str) -> String {
        let directory = tempfile::tempdir().expect("tempdir");
        let conf = directory.path().join("hostile.cnf");
        std::fs::write(&conf, HOSTILE_OPENSSL_CONF).expect("write hostile config");
        let absent = directory.path().join("no-such-directory");

        let output = std::process::Command::new(
            std::env::current_exe().expect("this test binary's own path"),
        )
        .args(["--exact", name, "--ignored", "--nocapture"])
        // Every environment variable the plan names, all hostile.
        .env("OPENSSL_CONF", &conf)
        .env("OPENSSL_CONF_INCLUDE", &absent)
        .env("OPENSSL_ENGINES", &absent)
        .env("OPENSSL_MODULES", &absent)
        .output()
        .expect("run the worker");

        let text = String::from_utf8_lossy(&output.stdout).into_owned();
        assert!(
            output.status.success(),
            "worker {name} failed: {text}{}",
            String::from_utf8_lossy(&output.stderr)
        );
        text
    }

    /// A fresh process with every OpenSSL environment variable pointed
    /// somewhere hostile still opens under the admitted provider, because it
    /// initialised the library itself before anything could read them.
    #[test]
    fn fresh_process_ignores_hostile_openssl_configuration() {
        let reported =
            run_worker("engine::process::tests::worker_opens_under_a_hostile_environment");
        assert!(
            reported.contains("WORKER opened provider=openssl"),
            "the hostile configuration reached the provider: {reported}"
        );
        assert!(
            reported.contains("version=OpenSSL 3."),
            "unexpected provider version: {reported}"
        );
    }

    /// And the same process, if it lets the configuration load first, is
    /// refused rather than quietly running on whatever that configuration
    /// chose. This is what makes the test above mean something: the hostile
    /// file is effective, so ignoring it is an achievement rather than a
    /// no-op.
    #[test]
    fn prior_load_config_process_is_rejected_by_profile_gate() {
        let reported = run_worker(
            "engine::process::tests::worker_loads_the_hostile_configuration_before_bootstrapping",
        );
        assert!(
            reported.contains("WORKER load_config=1"),
            "the negative control did not load the configuration: {reported}"
        );
        assert!(
            reported.contains("WORKER refused"),
            "a process that loaded the hostile configuration still opened a database: {reported}"
        );
        assert!(
            !reported.contains("WORKER opened"),
            "the open succeeded under the hostile provider: {reported}"
        );
    }
}

//! What the linked crypto actually is, read back from it.
//!
//! RFC 0005 Program 1A pins SQLCipher 4.14.0 and a vendored OpenSSL, and the
//! reason for pinning is that a different build is a different security
//! claim: 4.17.0 ships `sqlcipher_export`, which this engine's threat model
//! rejects. A constant in `lib.rs` saying "4.14.0" proves nothing about what
//! the linker found, so these tests ask the C library and compare.

use sovereign_vault_v2_engine::PINNED_SQLCIPHER_VERSION;

/// Ask the engine what it is, rather than trusting a declared profile.
///
/// `cipher_version` answering at all is the first assertion: plain SQLite
/// does not know that pragma, so an accidental build without SQLCipher — the
/// failure that would silently leave the store unencrypted — shows up here
/// rather than as a missing encryption nobody noticed.
#[test]
fn sqlcipher_runtime_is_exactly_4_14_0_for_released_profile() {
    let connection = rusqlite::Connection::open_in_memory().expect("in-memory connection");

    let cipher_version: String = connection
        .query_row("PRAGMA cipher_version", [], |row| row.get(0))
        .expect("PRAGMA cipher_version is unknown to plain SQLite: this build has no SQLCipher");

    // "4.14.0 community" — the edition follows the version, so compare the
    // version field rather than the whole string.
    let reported = cipher_version
        .split_whitespace()
        .next()
        .expect("a version before the edition");
    assert_eq!(
        reported, PINNED_SQLCIPHER_VERSION,
        "linked SQLCipher is {cipher_version:?}, not the pinned {PINNED_SQLCIPHER_VERSION}"
    );

    // 4.17.0 added `sqlcipher_export`, which this engine must not offer. The
    // pin is what keeps it out, so a newer runtime is a failure and not an
    // upgrade to wave through.
    assert!(
        !reported.starts_with("4.17"),
        "this runtime ships sqlcipher_export, which the Program 1A threat model rejects"
    );
}

/// The provider is the vendored OpenSSL, not whatever the host had.
///
/// Read back after setting a key: SQLCipher answers `cipher_provider` with no
/// rows until the connection is keyed, which is a real ordering property of
/// the library and the reason this is asserted here rather than assumed from
/// the build features.
#[test]
fn crypto_provider_is_the_vendored_openssl_and_only_answers_once_keyed() {
    let connection = rusqlite::Connection::open_in_memory().expect("in-memory connection");

    let before: Result<String, _> =
        connection.query_row("PRAGMA cipher_provider", [], |row| row.get(0));
    assert!(
        before.is_err(),
        "cipher_provider answered {before:?} before a key was set; the ordering assumption changed"
    );

    connection
        .pragma_update(None, "key", "probe-only-never-a-real-dbk")
        .expect("set a key");

    let provider: String = connection
        .query_row("PRAGMA cipher_provider", [], |row| row.get(0))
        .expect("cipher_provider after keying");
    assert_eq!(provider, "openssl", "unexpected crypto provider");

    let provider_version: String = connection
        .query_row("PRAGMA cipher_provider_version", [], |row| row.get(0))
        .expect("cipher_provider_version after keying");
    assert!(
        provider_version.starts_with("OpenSSL 3."),
        "provider is {provider_version:?}, not the vendored OpenSSL 3.x this crate pins"
    );
}

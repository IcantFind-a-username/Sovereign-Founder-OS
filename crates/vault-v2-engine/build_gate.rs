// Shared by `build.rs` and `tests/build_gate.rs` via `include!`, so the check
// that runs at build time and the check under test are literally the same
// code. A build script cannot be linked as a library, and a reimplementation
// in the test would only prove that the copy agrees with itself.
//
// Scope, stated plainly: this is defense in depth, not the qualification gate.
// A downstream build script runs after its dependencies have already been
// resolved and possibly reused from Cargo's cache, so it cannot prevent an
// overridden upstream dependency from having been built
// (docs/superpowers/plans/2026-08-13-dual-root-vault-v2-implementation.md,
// lines 185-188). `scripts/qualify-vault-v2.sh` is the real entry point and is
// a separate queued item.

/// Environment variables that reshape how this crate's future SQLCipher and
/// OpenSSL dependencies are located, configured, or built.
///
/// A closed allowlist by design: a newly observed dependency-shaping or
/// tool-selection variable stops for RFC review rather than being waved
/// through (same plan, lines 152-155).
pub const DEPENDENCY_SHAPING_VARIABLES: &[&str] = &[
    "LIBSQLITE3_SYS_USE_PKG_CONFIG",
    "LIBSQLITE3_FLAGS",
    "SQLITE_MAX_VARIABLE_NUMBER",
    "SQLITE_MAX_EXPR_DEPTH",
    "SQLITE_MAX_COLUMN",
    "SQLCIPHER_LIB_DIR",
    "SQLCIPHER_INCLUDE_DIR",
    "SQLCIPHER_STATIC",
    "OPENSSL_NO_VENDOR",
    "OPENSSL_DIR",
    "OPENSSL_LIB_DIR",
    "OPENSSL_INCLUDE_DIR",
    "OPENSSL_CONFIG_DIR",
    "OPENSSL_LIBS",
    "OPENSSL_STATIC",
    "OPENSSL_SRC_PERL",
    "OPENSSL_RUST_USE_NASM",
    "PERL",
    "PERL5OPT",
    "PERL5LIB",
    "VCPKGRS_DYNAMIC",
    "RUSTFLAGS",
    "CARGO_ENCODED_RUSTFLAGS",
];

/// Prefixes whose whole family is rejected.
pub const DEPENDENCY_SHAPING_PREFIXES: &[&str] = &["PKG_CONFIG_"];

/// Which variables in `environment` are rejected, in the order given.
///
/// Two deliberate rules:
///
/// - An empty value is not an override. Cargo itself always hands build
///   scripts a `CARGO_ENCODED_RUSTFLAGS`, empty when no flags are set;
///   treating its mere presence as hostile would reject every ordinary build
///   and teach everyone to ignore this check.
/// - A target-prefixed form counts. `openssl-sys` and friends honour
///   `<TARGET>_OPENSSL_DIR` (for example `X86_64_UNKNOWN_LINUX_GNU_OPENSSL_DIR`)
///   just as they honour the bare name, so a suffix match is the honest test.
pub fn rejected_variables(environment: &[(String, String)]) -> Vec<String> {
    let mut rejected = Vec::new();
    for (name, value) in environment {
        if value.trim().is_empty() {
            continue;
        }
        let shaping = DEPENDENCY_SHAPING_VARIABLES.iter().any(|candidate| {
            name == candidate
                || (name.ends_with(candidate)
                    && name.len() > candidate.len()
                    && name.as_bytes()[name.len() - candidate.len() - 1] == b'_')
        }) || DEPENDENCY_SHAPING_PREFIXES
            .iter()
            .any(|prefix| name.starts_with(prefix));
        if shaping {
            rejected.push(name.clone());
        }
    }
    rejected
}

//! Vault v2 engine — split library and dedicated process binary.
//!
//! **Maturity: Program 1A in progress (Developer Preview).** RFC 0005 puts raw
//! key material in a separate process; this crate is that process and its
//! reviewed link boundary. It is not a product-facing vault API yet.
//!
//! **Library target (`sovereign_vault_v2_engine`):** deliberately value-free —
//! on-disk format and SQLCipher profile constants only, with
//! `#![forbid(unsafe_code)]`. Downstream crates may depend on the pins without
//! pulling in key holders, FFI, or database handles.
//!
//! **Binary target (`sovereign-vault-v2-engine`):** compiles the private
//! `engine` module (OpenSSL process bootstrap, SQLCipher FFI, zeroizing secret
//! types). Cryptography is implemented there, not in the library's public API,
//! and nothing in `lib.rs` re-exports those modules
//! (`docs/superpowers/plans/2026-08-13-dual-root-vault-v2-implementation.md`,
//! lines 399-406 and 465-469).
//!
//! Scope of the protection, so the label stays honest: no network transport
//! exists in this workspace (`crates/effects/src/lib.rs`:26-30), so encryption
//! here means at-rest and backup confidentiality — never transit, and never
//! end-to-end.

#![forbid(unsafe_code)]

/// On-disk format version for the vault v2 store.
///
/// Version 1 is unreleased and has no importer yet. Once a released build can
/// write this format, the number is frozen: RFC 0005 Program 1A requires the
/// importer to read it byte-exactly, so a change here is a migration, not an
/// increment.
pub const VAULT_V2_FORMAT_VERSION: u32 = 1;

/// Wire protocol version for engine IPC (value-free library surface).
///
/// Downstream crates may depend on this constant to prove the protocol crate
/// resolved; the process engine itself is not linked from the library target.
pub const ENGINE_PROTOCOL_VERSION: u32 = 1;

/// The SQLCipher release the qualified connection profile is pinned to.
///
/// Verified at runtime against the linked library by a later item, not
/// asserted here — a constant on its own proves nothing about what got linked.
/// Program 1B0 (filtered encrypted backup) cannot start on this release
/// because of its fixed `sqlcipher_export` defensive-mode bypass; moving the
/// pin needs an RFC 0005 amendment, which is its own queued entry.
pub const PINNED_SQLCIPHER_VERSION: &str = "4.14.0";

/// Cipher page size, in bytes, for the fixed connection profile.
pub const CIPHER_PAGE_SIZE_BYTES: u32 = 4096;

/// SQLCipher compatibility level for the fixed connection profile.
pub const CIPHER_COMPATIBILITY: u32 = 4;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pinned_profile_constants_are_the_reviewed_values() {
        // These are load-bearing across the program: a silent edit here would
        // move the format or the cipher profile without an RFC amendment.
        assert_eq!(VAULT_V2_FORMAT_VERSION, 1);
        assert_eq!(PINNED_SQLCIPHER_VERSION, "4.14.0");
        assert_eq!(CIPHER_PAGE_SIZE_BYTES, 4096);
        assert_eq!(CIPHER_COMPATIBILITY, 4);
        assert_eq!(ENGINE_PROTOCOL_VERSION, 1);
    }
}

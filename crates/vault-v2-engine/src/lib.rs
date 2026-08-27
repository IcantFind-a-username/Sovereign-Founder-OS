//! Vault v2 engine — crate skeleton only.
//!
//! **Maturity: skeleton.** This crate carries no cryptography, no database
//! connection, no engine API, and no raw handle yet. Nothing here protects
//! anything; it exists so that the pieces that will (RFC 0005 Program 1A) land
//! against a fixed, reviewed build boundary instead of creating one in a hurry.
//!
//! The library target is deliberately value-free: protocol and version
//! constants only. The engine, its single `unsafe` FFI module, and the
//! zeroizing key holder are separate queued items, and the plan requires them
//! to live in private modules that the library never re-exports
//! (`docs/superpowers/plans/2026-08-13-dual-root-vault-v2-implementation.md`,
//! lines 399-406 and 465-469).
//!
//! Scope of the eventual protection, so the label stays honest: no network
//! transport exists in this workspace (`crates/effects/src/lib.rs`:26-30), so
//! "encryption" here will mean at-rest and backup confidentiality — never
//! transit, and never end-to-end.

#![forbid(unsafe_code)]

/// On-disk format version for the vault v2 store.
///
/// Version 1 is unreleased and has no importer yet. Once a released build can
/// write this format, the number is frozen: RFC 0005 Program 1A requires the
/// importer to read it byte-exactly, so a change here is a migration, not an
/// increment.
pub const VAULT_V2_FORMAT_VERSION: u32 = 1;

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
    }
}

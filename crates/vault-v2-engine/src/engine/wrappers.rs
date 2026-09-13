//! Typed AAD encodings and XChaCha20-Poly1305 DBK/KEK wrappers (RFC 0005).
//!
//! Three distinct internal AAD types — no generic purpose enum. DBK equality
//! after unwrap uses only constant-time comparison.

use chacha20poly1305::aead::{AeadInOut, KeyInit};
use chacha20poly1305::aead::inout::InOutBuf;
use chacha20poly1305::{Key, Tag, XChaCha20Poly1305, XNonce};
use subtle::ConstantTimeEq;
use zeroize::{Zeroize, ZeroizeOnDrop};

pub(crate) const WRAPPER_VERSION: u16 = 1;
pub(crate) const SUITE_VERSION: u16 = 1;
pub(crate) const DB_KEY_EPOCH: u64 = 1;
pub(crate) const DATABASE_ROLE_LIVE: u8 = 1;
pub(crate) const ARGON_PROFILE_TAG_V1: u16 = 1;

pub(crate) const DEVICE_DBK_AAD_LEN: usize = 206;
pub(crate) const PWK_RECOVERY_KEK_AAD_LEN: usize = 190;
pub(crate) const RECOVERY_DBK_AAD_LEN: usize = 177;

pub(crate) const WRAPPED_KEY_LEN: usize = 32;
pub(crate) const WRAPPED_TAG_LEN: usize = 16;
pub(crate) const WRAPPED_RECORD_LEN: usize = WRAPPED_KEY_LEN + WRAPPED_TAG_LEN;
pub(crate) const XCHACHA_NONCE_LEN: usize = 24;

/// 32-byte protocol identifier (workspace, database, record IDs, …).
pub(crate) type ProtocolId = [u8; 32];

#[derive(Zeroize, ZeroizeOnDrop)]
pub(crate) struct DeviceKek([u8; 32]);

#[derive(Zeroize, ZeroizeOnDrop)]
pub(crate) struct RecoveryKek([u8; 32]);

#[derive(Zeroize, ZeroizeOnDrop)]
pub(crate) struct Pwk([u8; 32]);

impl DeviceKek {
    pub(crate) fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub(crate) fn expose(&self) -> &[u8; 32] {
        &self.0
    }
}

impl RecoveryKek {
    pub(crate) fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub(crate) fn expose(&self) -> &[u8; 32] {
        &self.0
    }
}

impl Pwk {
    pub(crate) fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub(crate) fn expose(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Device-route DBK wrap AAD (206 bytes).
pub(crate) struct DeviceDbkAad {
    pub(crate) workspace_id: ProtocolId,
    pub(crate) database_id: ProtocolId,
    pub(crate) protector_record_id: ProtocolId,
    pub(crate) device_wrapper_id: ProtocolId,
    pub(crate) recovery_slot_commitment: [u8; 32],
}

impl DeviceDbkAad {
    pub(crate) fn to_bytes(&self) -> [u8; DEVICE_DBK_AAD_LEN] {
        let mut out = [0u8; DEVICE_DBK_AAD_LEN];
        let mut offset = 0;
        offset = write_literal(&mut out, offset, b"sovereign:vault:v2:device-dbk-wrap");
        offset = write_u16_be(&mut out, offset, WRAPPER_VERSION);
        offset = write_u16_be(&mut out, offset, SUITE_VERSION);
        offset = write_bytes32(&mut out, offset, &self.workspace_id);
        offset = write_bytes32(&mut out, offset, &self.database_id);
        offset = write_bytes32(&mut out, offset, &self.protector_record_id);
        offset = write_bytes32(&mut out, offset, &self.device_wrapper_id);
        offset = write_u64_be(&mut out, offset, DB_KEY_EPOCH);
        write_bytes32(&mut out, offset, &self.recovery_slot_commitment);
        out
    }
}

/// PWK → RecoveryKEK wrap AAD (190 bytes).
pub(crate) struct PwkRecoveryKekAad {
    pub(crate) workspace_id: ProtocolId,
    pub(crate) database_id: ProtocolId,
    pub(crate) recovery_record_id: ProtocolId,
    pub(crate) recovery_kek_id: ProtocolId,
    pub(crate) argon_salt: [u8; 16],
}

impl PwkRecoveryKekAad {
    pub(crate) fn to_bytes(&self) -> [u8; PWK_RECOVERY_KEK_AAD_LEN] {
        let mut out = [0u8; PWK_RECOVERY_KEK_AAD_LEN];
        let mut offset = 0;
        offset = write_literal(&mut out, offset, b"sovereign:vault:v2:pwk-recovery-kek-wrap");
        offset = write_u16_be(&mut out, offset, WRAPPER_VERSION);
        offset = write_u16_be(&mut out, offset, SUITE_VERSION);
        offset = write_bytes32(&mut out, offset, &self.workspace_id);
        offset = write_bytes32(&mut out, offset, &self.database_id);
        offset = write_bytes32(&mut out, offset, &self.recovery_record_id);
        offset = write_bytes32(&mut out, offset, &self.recovery_kek_id);
        offset = write_u16_be(&mut out, offset, ARGON_PROFILE_TAG_V1);
        write_bytes16(&mut out, offset, &self.argon_salt);
        out
    }
}

/// Recovery-route DBK wrap AAD (177 bytes).
pub(crate) struct RecoveryDbkAad {
    pub(crate) workspace_id: ProtocolId,
    pub(crate) database_id: ProtocolId,
    pub(crate) recovery_record_id: ProtocolId,
    pub(crate) recovery_kek_id: ProtocolId,
    pub(crate) database_role: u8,
}

impl RecoveryDbkAad {
    pub(crate) fn to_bytes(&self) -> [u8; RECOVERY_DBK_AAD_LEN] {
        let mut out = [0u8; RECOVERY_DBK_AAD_LEN];
        let mut offset = 0;
        offset = write_literal(&mut out, offset, b"sovereign:vault:v2:recovery-dbk-wrap");
        offset = write_u16_be(&mut out, offset, WRAPPER_VERSION);
        offset = write_u16_be(&mut out, offset, SUITE_VERSION);
        offset = write_bytes32(&mut out, offset, &self.workspace_id);
        offset = write_bytes32(&mut out, offset, &self.database_id);
        offset = write_bytes32(&mut out, offset, &self.recovery_record_id);
        offset = write_bytes32(&mut out, offset, &self.recovery_kek_id);
        offset = write_u64_be(&mut out, offset, DB_KEY_EPOCH);
        out[offset] = self.database_role;
        out
    }
}

#[derive(Clone, Copy)]
pub(crate) struct WrappedRecord {
    pub(crate) nonce: [u8; XCHACHA_NONCE_LEN],
    /// Ciphertext || tag (48 bytes).
    pub(crate) ciphertext: [u8; WRAPPED_RECORD_LEN],
}

#[derive(Debug)]
pub(crate) enum WrapError {
    Crypto,
}

#[derive(Debug)]
pub(crate) enum UnwrapError {
    Crypto,
}

pub(crate) fn wrap_device_dbk(
    kek: &DeviceKek,
    aad: &DeviceDbkAad,
    dbk_plaintext: &[u8; 32],
    nonce: &[u8; XCHACHA_NONCE_LEN],
) -> Result<WrappedRecord, WrapError> {
    wrap_with_aad(kek.expose(), &aad.to_bytes(), dbk_plaintext, nonce)
}

pub(crate) fn unwrap_device_dbk(
    kek: &DeviceKek,
    aad: &DeviceDbkAad,
    record: &WrappedRecord,
) -> Result<[u8; 32], UnwrapError> {
    unwrap_with_aad(kek.expose(), &aad.to_bytes(), record)
}

pub(crate) fn wrap_recovery_kek_with_pwk(
    pwk: &Pwk,
    aad: &PwkRecoveryKekAad,
    recovery_kek_plaintext: &[u8; 32],
    nonce: &[u8; XCHACHA_NONCE_LEN],
) -> Result<WrappedRecord, WrapError> {
    wrap_with_aad(pwk.expose(), &aad.to_bytes(), recovery_kek_plaintext, nonce)
}

pub(crate) fn unwrap_recovery_kek_with_pwk(
    pwk: &Pwk,
    aad: &PwkRecoveryKekAad,
    record: &WrappedRecord,
) -> Result<[u8; 32], UnwrapError> {
    unwrap_with_aad(pwk.expose(), &aad.to_bytes(), record)
}

pub(crate) fn wrap_recovery_dbk(
    recovery_kek: &RecoveryKek,
    aad: &RecoveryDbkAad,
    dbk_plaintext: &[u8; 32],
    nonce: &[u8; XCHACHA_NONCE_LEN],
) -> Result<WrappedRecord, WrapError> {
    wrap_with_aad(recovery_kek.expose(), &aad.to_bytes(), dbk_plaintext, nonce)
}

pub(crate) fn unwrap_recovery_dbk(
    recovery_kek: &RecoveryKek,
    aad: &RecoveryDbkAad,
    record: &WrappedRecord,
) -> Result<[u8; 32], UnwrapError> {
    unwrap_with_aad(recovery_kek.expose(), &aad.to_bytes(), record)
}

/// Constant-time DBK equality after unwrap (Task 2 / Task 4 path).
pub(crate) fn dbk_matches_expected(unwrapped: &[u8; 32], expected: &[u8; 32]) -> bool {
    unwrapped.ct_eq(expected).into()
}

#[allow(deprecated)]
fn wrap_with_aad(
    key: &[u8; 32],
    aad: &[u8],
    plaintext: &[u8; 32],
    nonce: &[u8; XCHACHA_NONCE_LEN],
) -> Result<WrappedRecord, WrapError> {
    let cipher = XChaCha20Poly1305::new(Key::from_slice(key));
    let mut buffer = *plaintext;
    let inout = InOutBuf::from(buffer.as_mut_slice());
    let tag: Tag = cipher
        .encrypt_inout_detached(XNonce::from_slice(nonce), aad, inout)
        .map_err(|_| WrapError::Crypto)?;
    let mut ciphertext = [0u8; WRAPPED_RECORD_LEN];
    ciphertext[..WRAPPED_KEY_LEN].copy_from_slice(&buffer);
    ciphertext[WRAPPED_KEY_LEN..].copy_from_slice(tag.as_ref());
    Ok(WrappedRecord {
        nonce: *nonce,
        ciphertext,
    })
}

#[allow(deprecated)]
fn unwrap_with_aad(
    key: &[u8; 32],
    aad: &[u8],
    record: &WrappedRecord,
) -> Result<[u8; 32], UnwrapError> {
    let cipher = XChaCha20Poly1305::new(Key::from_slice(key));
    let mut buffer = [0u8; WRAPPED_KEY_LEN];
    buffer.copy_from_slice(&record.ciphertext[..WRAPPED_KEY_LEN]);
    let tag_bytes = &record.ciphertext[WRAPPED_KEY_LEN..];
    let tag = Tag::from_slice(tag_bytes);
    let inout = InOutBuf::from(buffer.as_mut_slice());
    cipher
        .decrypt_inout_detached(XNonce::from_slice(&record.nonce), aad, inout, tag)
        .map_err(|_| UnwrapError::Crypto)?;
    Ok(buffer)
}

fn write_literal(out: &mut [u8], offset: usize, literal: &[u8]) -> usize {
    out[offset..offset + literal.len()].copy_from_slice(literal);
    offset + literal.len()
}

fn write_u16_be(out: &mut [u8], offset: usize, value: u16) -> usize {
    out[offset] = (value >> 8) as u8;
    out[offset + 1] = (value & 0xff) as u8;
    offset + 2
}

fn write_u64_be(out: &mut [u8], offset: usize, value: u64) -> usize {
    for index in 0..8 {
        out[offset + index] = (value >> (56 - index * 8)) as u8;
    }
    offset + 8
}

fn write_bytes32(out: &mut [u8], offset: usize, value: &[u8; 32]) -> usize {
    out[offset..offset + 32].copy_from_slice(value);
    offset + 32
}

fn write_bytes16(out: &mut [u8], offset: usize, value: &[u8; 16]) -> usize {
    out[offset..offset + 16].copy_from_slice(value);
    offset + 16
}

#[cfg(test)]
mod tests {
    use super::*;
    use static_assertions::assert_not_impl_any;

    assert_not_impl_any!(DeviceKek: Clone, std::fmt::Debug, std::fmt::Display);
    assert_not_impl_any!(RecoveryKek: Clone, std::fmt::Debug, std::fmt::Display);
    assert_not_impl_any!(Pwk: Clone, std::fmt::Debug, std::fmt::Display);

    fn fixed_id(byte: u8) -> ProtocolId {
        [byte; 32]
    }

    #[test]
    fn typed_aad_lengths_match_rfc_golden_profile() {
        let device = DeviceDbkAad {
            workspace_id: fixed_id(0x01),
            database_id: fixed_id(0x02),
            protector_record_id: fixed_id(0x03),
            device_wrapper_id: fixed_id(0x04),
            recovery_slot_commitment: fixed_id(0x05),
        };
        assert_eq!(device.to_bytes().len(), DEVICE_DBK_AAD_LEN);
        assert_eq!(
            device.to_bytes()[..34],
            b"sovereign:vault:v2:device-dbk-wrap"[..]
        );

        let pwk_aad = PwkRecoveryKekAad {
            workspace_id: fixed_id(0x11),
            database_id: fixed_id(0x12),
            recovery_record_id: fixed_id(0x13),
            recovery_kek_id: fixed_id(0x14),
            argon_salt: [0xab; 16],
        };
        assert_eq!(pwk_aad.to_bytes().len(), PWK_RECOVERY_KEK_AAD_LEN);

        let recovery = RecoveryDbkAad {
            workspace_id: fixed_id(0x21),
            database_id: fixed_id(0x22),
            recovery_record_id: fixed_id(0x23),
            recovery_kek_id: fixed_id(0x24),
            database_role: DATABASE_ROLE_LIVE,
        };
        assert_eq!(recovery.to_bytes().len(), RECOVERY_DBK_AAD_LEN);
    }

    #[test]
    fn one_field_mutation_changes_device_aad_bytes() {
        let base = DeviceDbkAad {
            workspace_id: fixed_id(0xaa),
            database_id: fixed_id(0xbb),
            protector_record_id: fixed_id(0xcc),
            device_wrapper_id: fixed_id(0xdd),
            recovery_slot_commitment: fixed_id(0xee),
        };
        let bytes = base.to_bytes();
        let mut mutated = base;
        mutated.workspace_id[0] ^= 0x01;
        assert_ne!(mutated.to_bytes(), bytes);
    }

    #[test]
    fn cross_purpose_aad_is_not_substitutable_at_decrypt() {
        let kek = DeviceKek::from_bytes([0x42; 32]);
        let dbk = [0x99; 32];
        let nonce = [0x07; 24];
        let device_aad = DeviceDbkAad {
            workspace_id: fixed_id(1),
            database_id: fixed_id(2),
            protector_record_id: fixed_id(3),
            device_wrapper_id: fixed_id(4),
            recovery_slot_commitment: fixed_id(5),
        };
        let record = wrap_device_dbk(&kek, &device_aad, &dbk, &nonce).expect("wrap");
        let wrong_aad = DeviceDbkAad {
            workspace_id: fixed_id(9),
            database_id: fixed_id(2),
            protector_record_id: fixed_id(3),
            device_wrapper_id: fixed_id(4),
            recovery_slot_commitment: fixed_id(5),
        };
        assert!(unwrap_device_dbk(&kek, &wrong_aad, &record).is_err());
    }

    #[test]
    fn fresh_nonces_produce_distinct_ciphertexts() {
        let kek = DeviceKek::from_bytes([0x11; 32]);
        let dbk = [0x22; 32];
        let aad = DeviceDbkAad {
            workspace_id: fixed_id(1),
            database_id: fixed_id(2),
            protector_record_id: fixed_id(3),
            device_wrapper_id: fixed_id(4),
            recovery_slot_commitment: fixed_id(5),
        };
        let first = wrap_device_dbk(&kek, &aad, &dbk, &[1; 24]).expect("wrap");
        let second = wrap_device_dbk(&kek, &aad, &dbk, &[2; 24]).expect("wrap");
        assert_ne!(first.ciphertext, second.ciphertext);
    }

    #[test]
    fn dbk_equality_uses_constant_time_path() {
        let a = [0u8; 32];
        let b = [0u8; 32];
        let c = [1u8; 32];
        assert!(dbk_matches_expected(&a, &b));
        assert!(!dbk_matches_expected(&a, &c));
    }

    #[test]
    fn round_trip_device_and_recovery_routes_independently() {
        let device_kek = DeviceKek::from_bytes([0x01; 32]);
        let recovery_kek = RecoveryKek::from_bytes([0x02; 32]);
        let pwk = Pwk::from_bytes([0x03; 32]);
        let dbk = [0x44; 32];
        let nonce = [0x55; 24];

        let commitment = [0x66; 32];
        let device_aad = DeviceDbkAad {
            workspace_id: fixed_id(0x10),
            database_id: fixed_id(0x20),
            protector_record_id: fixed_id(0x30),
            device_wrapper_id: fixed_id(0x40),
            recovery_slot_commitment: commitment,
        };
        let device_record = wrap_device_dbk(&device_kek, &device_aad, &dbk, &nonce).expect("d");
        let from_device = unwrap_device_dbk(&device_kek, &device_aad, &device_record).expect("du");
        assert!(dbk_matches_expected(&from_device, &dbk));

        let salt = [0x77; 16];
        let pwk_aad = PwkRecoveryKekAad {
            workspace_id: fixed_id(0x10),
            database_id: fixed_id(0x20),
            recovery_record_id: fixed_id(0x31),
            recovery_kek_id: fixed_id(0x41),
            argon_salt: salt,
        };
        let kek_record =
            wrap_recovery_kek_with_pwk(&pwk, &pwk_aad, recovery_kek.expose(), &[0x88; 24])
                .expect("k");
        let recovered_kek =
            unwrap_recovery_kek_with_pwk(&pwk, &pwk_aad, &kek_record).expect("ku");
        let rk = RecoveryKek::from_bytes(recovered_kek);

        let recovery_aad = RecoveryDbkAad {
            workspace_id: fixed_id(0x10),
            database_id: fixed_id(0x20),
            recovery_record_id: fixed_id(0x31),
            recovery_kek_id: fixed_id(0x41),
            database_role: DATABASE_ROLE_LIVE,
        };
        let recovery_record = wrap_recovery_dbk(&rk, &recovery_aad, &dbk, &[0x99; 24]).expect("r");
        let from_recovery =
            unwrap_recovery_dbk(&rk, &recovery_aad, &recovery_record).expect("ru");
        assert!(dbk_matches_expected(&from_recovery, &dbk));
    }
}

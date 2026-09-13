//! Fixed `vault.slots` sidecar parsing and recovery-slot commitment (RFC 0005).

use crate::engine::wrappers::{
    ARGON_PROFILE_TAG_V1, ProtocolId, WRAPPED_RECORD_LEN, XCHACHA_NONCE_LEN,
};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicUsize, Ordering};

pub(crate) const SLOTS_FORMAT_VERSION: u32 = 2;
pub(crate) const SLOTS_SUITE_VERSION: u32 = 1;
pub(crate) const SLOTS_DB_KEY_EPOCH: u64 = 1;
pub(crate) const SLOTS_MAX_BYTES: usize = 256 * 1024;

const RECOVERY_COMMITMENT_DOMAIN: &[u8] = b"sovereign:vault:v2:recovery-slot-commitment";

/// Observable admission counters for tests (KDF/unwrap gates).
pub(crate) struct AdmissionCounters {
    pub(crate) kdf: AtomicUsize,
    pub(crate) unwrap: AtomicUsize,
    pub(crate) keyring: AtomicUsize,
}

impl AdmissionCounters {
    pub(crate) fn new() -> Self {
        Self {
            kdf: AtomicUsize::new(0),
            unwrap: AtomicUsize::new(0),
            keyring: AtomicUsize::new(0),
        }
    }

    pub(crate) fn snapshot(&self) -> (usize, usize, usize) {
        (
            self.kdf.load(Ordering::SeqCst),
            self.unwrap.load(Ordering::SeqCst),
            self.keyring.load(Ordering::SeqCst),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SlotsParseError;

#[derive(Debug, Clone)]
pub(crate) struct RecoverySubrecord {
    pub(crate) argon_profile_tag: u16,
    pub(crate) argon_salt: [u8; 16],
    pub(crate) recovery_record_id: ProtocolId,
    pub(crate) recovery_kek_id: ProtocolId,
    pub(crate) kek_nonce: [u8; XCHACHA_NONCE_LEN],
    pub(crate) kek_ciphertext: [u8; WRAPPED_RECORD_LEN],
    pub(crate) dbk_nonce: [u8; XCHACHA_NONCE_LEN],
    pub(crate) dbk_ciphertext: [u8; WRAPPED_RECORD_LEN],
}

#[derive(Debug, Clone)]
pub(crate) struct DeviceSubrecord {
    pub(crate) protector_record_id: ProtocolId,
    pub(crate) device_wrapper_id: ProtocolId,
    pub(crate) recovery_slot_commitment: [u8; 32],
    pub(crate) dbk_nonce: [u8; XCHACHA_NONCE_LEN],
    pub(crate) dbk_ciphertext: [u8; WRAPPED_RECORD_LEN],
}

#[derive(Debug, Clone)]
pub(crate) struct VaultSlotsRecord {
    pub(crate) workspace_id: ProtocolId,
    pub(crate) database_id: ProtocolId,
    pub(crate) format_version: u32,
    pub(crate) suite_version: u32,
    pub(crate) db_key_epoch: u64,
    pub(crate) device: DeviceSubrecord,
    pub(crate) recovery: RecoverySubrecord,
    pub(crate) canonical_bytes: Vec<u8>,
}

/// Recompute `recovery_slot_commitment` from the parsed recovery subrecord.
pub(crate) fn recovery_slot_commitment(recovery: &RecoverySubrecord) -> Result<[u8; 32], SlotsParseError> {
    let jcs = recovery_subrecord_jcs(recovery)?;
    let mut hasher = Sha256::new();
    hasher.update(RECOVERY_COMMITMENT_DOMAIN);
    hasher.update([0x00, 0x01]);
    hasher.update(&jcs);
    Ok(hasher.finalize().into())
}

pub(crate) fn parse_vault_slots(
    bytes: &[u8],
    expected_workspace: &ProtocolId,
    expected_database: &ProtocolId,
) -> Result<VaultSlotsRecord, SlotsParseError> {
    if bytes.len() > SLOTS_MAX_BYTES {
        return Err(SlotsParseError);
    }
    let value: serde_json::Value = serde_json::from_slice(bytes).map_err(|_| SlotsParseError)?;
    if !value.is_object() {
        return Err(SlotsParseError);
    }
    let object = value.as_object().ok_or(SlotsParseError)?;
    let allowed = [
        "database_id",
        "db_key_epoch",
        "device",
        "format_version",
        "recovery",
        "suite_version",
        "workspace_id",
    ];
    if object.len() != allowed.len() {
        return Err(SlotsParseError);
    }
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(SlotsParseError);
        }
    }

    let workspace_id = decode_id(object.get("workspace_id").ok_or(SlotsParseError)?)?;
    let database_id = decode_id(object.get("database_id").ok_or(SlotsParseError)?)?;
    if workspace_id != *expected_workspace || database_id != *expected_database {
        return Err(SlotsParseError);
    }

    let format_version = decode_u32_decimal(object.get("format_version").ok_or(SlotsParseError)?)?;
    let suite_version = decode_u32_decimal(object.get("suite_version").ok_or(SlotsParseError)?)?;
    let db_key_epoch = decode_u64_decimal(object.get("db_key_epoch").ok_or(SlotsParseError)?)?;
    if format_version != SLOTS_FORMAT_VERSION
        || suite_version != SLOTS_SUITE_VERSION
        || db_key_epoch != SLOTS_DB_KEY_EPOCH
    {
        return Err(SlotsParseError);
    }

    let device = parse_device(object.get("device").ok_or(SlotsParseError)?)?;
    let recovery = parse_recovery(object.get("recovery").ok_or(SlotsParseError)?)?;

    let recomputed = recovery_slot_commitment(&recovery)?;
    if recomputed != device.recovery_slot_commitment {
        return Err(SlotsParseError);
    }

    let canonical = serde_json_canonicalizer::to_vec(&value).map_err(|_| SlotsParseError)?;
    if canonical != bytes {
        return Err(SlotsParseError);
    }

    Ok(VaultSlotsRecord {
        workspace_id,
        database_id,
        format_version,
        suite_version,
        db_key_epoch,
        device,
        recovery,
        canonical_bytes: bytes.to_vec(),
    })
}

fn recovery_subrecord_jcs(recovery: &RecoverySubrecord) -> Result<Vec<u8>, SlotsParseError> {
    let value = recovery_subrecord_value(recovery)?;
    serde_json_canonicalizer::to_vec(&value).map_err(|_| SlotsParseError)
}

fn recovery_subrecord_value(recovery: &RecoverySubrecord) -> Result<serde_json::Value, SlotsParseError> {
    let mut object = serde_json::Map::new();
    object.insert(
        "argon_profile_tag".into(),
        serde_json::Value::String(recovery.argon_profile_tag.to_string()),
    );
    object.insert(
        "argon_salt".into(),
        serde_json::Value::String(URL_SAFE_NO_PAD.encode(recovery.argon_salt)),
    );
    object.insert(
        "dbk_ciphertext".into(),
        serde_json::Value::String(URL_SAFE_NO_PAD.encode(recovery.dbk_ciphertext)),
    );
    object.insert(
        "dbk_nonce".into(),
        serde_json::Value::String(URL_SAFE_NO_PAD.encode(recovery.dbk_nonce)),
    );
    object.insert(
        "kek_ciphertext".into(),
        serde_json::Value::String(URL_SAFE_NO_PAD.encode(recovery.kek_ciphertext)),
    );
    object.insert(
        "kek_nonce".into(),
        serde_json::Value::String(URL_SAFE_NO_PAD.encode(recovery.kek_nonce)),
    );
    object.insert(
        "recovery_kek_id".into(),
        serde_json::Value::String(URL_SAFE_NO_PAD.encode(recovery.recovery_kek_id)),
    );
    object.insert(
        "recovery_record_id".into(),
        serde_json::Value::String(URL_SAFE_NO_PAD.encode(recovery.recovery_record_id)),
    );
    Ok(serde_json::Value::Object(object))
}

fn parse_device(value: &serde_json::Value) -> Result<DeviceSubrecord, SlotsParseError> {
    let object = value.as_object().ok_or(SlotsParseError)?;
    let allowed = [
        "dbk_ciphertext",
        "dbk_nonce",
        "device_wrapper_id",
        "protector_record_id",
        "recovery_slot_commitment",
    ];
    if object.len() != allowed.len() {
        return Err(SlotsParseError);
    }
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(SlotsParseError);
        }
    }
    Ok(DeviceSubrecord {
        protector_record_id: decode_id(object.get("protector_record_id").ok_or(SlotsParseError)?)?,
        device_wrapper_id: decode_id(object.get("device_wrapper_id").ok_or(SlotsParseError)?)?,
        recovery_slot_commitment: decode_id(
            object
                .get("recovery_slot_commitment")
                .ok_or(SlotsParseError)?,
        )?,
        dbk_nonce: decode_nonce(object.get("dbk_nonce").ok_or(SlotsParseError)?)?,
        dbk_ciphertext: decode_wrapped(object.get("dbk_ciphertext").ok_or(SlotsParseError)?)?,
    })
}

fn parse_recovery(value: &serde_json::Value) -> Result<RecoverySubrecord, SlotsParseError> {
    let object = value.as_object().ok_or(SlotsParseError)?;
    let allowed = [
        "argon_profile_tag",
        "argon_salt",
        "dbk_ciphertext",
        "dbk_nonce",
        "kek_ciphertext",
        "kek_nonce",
        "recovery_kek_id",
        "recovery_record_id",
    ];
    if object.len() != allowed.len() {
        return Err(SlotsParseError);
    }
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(SlotsParseError);
        }
    }
    let tag = decode_u16_decimal(object.get("argon_profile_tag").ok_or(SlotsParseError)?)?;
    if tag != ARGON_PROFILE_TAG_V1 {
        return Err(SlotsParseError);
    }
    Ok(RecoverySubrecord {
        argon_profile_tag: tag,
        argon_salt: decode_salt(object.get("argon_salt").ok_or(SlotsParseError)?)?,
        recovery_record_id: decode_id(object.get("recovery_record_id").ok_or(SlotsParseError)?)?,
        recovery_kek_id: decode_id(object.get("recovery_kek_id").ok_or(SlotsParseError)?)?,
        kek_nonce: decode_nonce(object.get("kek_nonce").ok_or(SlotsParseError)?)?,
        kek_ciphertext: decode_wrapped(object.get("kek_ciphertext").ok_or(SlotsParseError)?)?,
        dbk_nonce: decode_nonce(object.get("dbk_nonce").ok_or(SlotsParseError)?)?,
        dbk_ciphertext: decode_wrapped(object.get("dbk_ciphertext").ok_or(SlotsParseError)?)?,
    })
}

fn decode_id(value: &serde_json::Value) -> Result<ProtocolId, SlotsParseError> {
    let text = value.as_str().ok_or(SlotsParseError)?;
    decode_fixed_b64::<32>(text)
}

fn decode_nonce(value: &serde_json::Value) -> Result<[u8; XCHACHA_NONCE_LEN], SlotsParseError> {
    let text = value.as_str().ok_or(SlotsParseError)?;
    decode_fixed_b64::<XCHACHA_NONCE_LEN>(text)
}

fn decode_wrapped(value: &serde_json::Value) -> Result<[u8; WRAPPED_RECORD_LEN], SlotsParseError> {
    let text = value.as_str().ok_or(SlotsParseError)?;
    decode_fixed_b64::<WRAPPED_RECORD_LEN>(text)
}

fn decode_salt(value: &serde_json::Value) -> Result<[u8; 16], SlotsParseError> {
    let text = value.as_str().ok_or(SlotsParseError)?;
    decode_fixed_b64::<16>(text)
}

fn decode_fixed_b64<const N: usize>(text: &str) -> Result<[u8; N], SlotsParseError> {
    if text.contains('=') || text.contains('+') || text.contains('/') {
        return Err(SlotsParseError);
    }
    let bytes = URL_SAFE_NO_PAD.decode(text).map_err(|_| SlotsParseError)?;
    if bytes.len() != N {
        return Err(SlotsParseError);
    }
    let mut out = [0u8; N];
    out.copy_from_slice(&bytes);
    Ok(out)
}

fn decode_u32_decimal(value: &serde_json::Value) -> Result<u32, SlotsParseError> {
    let text = value.as_str().ok_or(SlotsParseError)?;
    if text.starts_with('+') || text.starts_with('-') {
        return Err(SlotsParseError);
    }
    if text.len() > 1 && text.starts_with('0') {
        return Err(SlotsParseError);
    }
    text.parse::<u32>().map_err(|_| SlotsParseError)
}

fn decode_u64_decimal(value: &serde_json::Value) -> Result<u64, SlotsParseError> {
    let text = value.as_str().ok_or(SlotsParseError)?;
    if text.starts_with('+') || text.starts_with('-') {
        return Err(SlotsParseError);
    }
    if text.len() > 1 && text.starts_with('0') {
        return Err(SlotsParseError);
    }
    text.parse::<u64>().map_err(|_| SlotsParseError)
}

fn decode_u16_decimal(value: &serde_json::Value) -> Result<u16, SlotsParseError> {
    let parsed = decode_u32_decimal(value)?;
    u16::try_from(parsed).map_err(|_| SlotsParseError)
}

#[cfg(test)]
mod test_support {
    use super::*;
    use crate::engine::wrappers::{
        wrap_device_dbk, wrap_recovery_dbk, wrap_recovery_kek_with_pwk, DeviceDbkAad,
        DeviceKek, Pwk, PwkRecoveryKekAad, RecoveryDbkAad, RecoveryKek,
    };

    fn id(byte: u8) -> ProtocolId {
        [byte; 32]
    }

    pub(crate) fn build_test_canonical_slots(
        workspace: ProtocolId,
        database: ProtocolId,
        device_kek: &DeviceKek,
        recovery_kek: &RecoveryKek,
        pwk: &Pwk,
        dbk: [u8; 32],
    ) -> Vec<u8> {
        let salt = [0x01; 16];
        let pwk_aad = PwkRecoveryKekAad {
            workspace_id: workspace,
            database_id: database,
            recovery_record_id: id(0x31),
            recovery_kek_id: id(0x41),
            argon_salt: salt,
        };
        let kek_record =
            wrap_recovery_kek_with_pwk(pwk, &pwk_aad, recovery_kek.expose(), &[0x02; 24])
                .expect("kek wrap");
        let recovery_aad = RecoveryDbkAad {
            workspace_id: workspace,
            database_id: database,
            recovery_record_id: id(0x31),
            recovery_kek_id: id(0x41),
            database_role: 1,
        };
        let recovery_dbk =
            wrap_recovery_dbk(recovery_kek, &recovery_aad, &dbk, &[0x03; 24]).expect("dbk wrap");

        let recovery = RecoverySubrecord {
            argon_profile_tag: ARGON_PROFILE_TAG_V1,
            argon_salt: salt,
            recovery_record_id: id(0x31),
            recovery_kek_id: id(0x41),
            kek_nonce: kek_record.nonce,
            kek_ciphertext: kek_record.ciphertext,
            dbk_nonce: recovery_dbk.nonce,
            dbk_ciphertext: recovery_dbk.ciphertext,
        };
        let commitment = recovery_slot_commitment(&recovery).expect("commitment");

        let device_aad = DeviceDbkAad {
            workspace_id: workspace,
            database_id: database,
            protector_record_id: id(0x51),
            device_wrapper_id: id(0x61),
            recovery_slot_commitment: commitment,
        };
        let device_dbk =
            wrap_device_dbk(device_kek, &device_aad, &dbk, &[0x04; 24]).expect("device wrap");

        let recovery_value = recovery_subrecord_value(&recovery).expect("recovery value");
        let mut device_object = serde_json::Map::new();
        device_object.insert(
            "dbk_ciphertext".into(),
            serde_json::Value::String(URL_SAFE_NO_PAD.encode(device_dbk.ciphertext)),
        );
        device_object.insert(
            "dbk_nonce".into(),
            serde_json::Value::String(URL_SAFE_NO_PAD.encode(device_dbk.nonce)),
        );
        device_object.insert(
            "device_wrapper_id".into(),
            serde_json::Value::String(URL_SAFE_NO_PAD.encode(id(0x61))),
        );
        device_object.insert(
            "protector_record_id".into(),
            serde_json::Value::String(URL_SAFE_NO_PAD.encode(id(0x51))),
        );
        device_object.insert(
            "recovery_slot_commitment".into(),
            serde_json::Value::String(URL_SAFE_NO_PAD.encode(commitment)),
        );
        let mut top = serde_json::Map::new();
        top.insert(
            "database_id".into(),
            serde_json::Value::String(URL_SAFE_NO_PAD.encode(database)),
        );
        top.insert("db_key_epoch".into(), serde_json::Value::String("1".into()));
        top.insert("device".into(), serde_json::Value::Object(device_object));
        top.insert("format_version".into(), serde_json::Value::String("2".into()));
        top.insert("recovery".into(), recovery_value);
        top.insert("suite_version".into(), serde_json::Value::String("1".into()));
        top.insert(
            "workspace_id".into(),
            serde_json::Value::String(URL_SAFE_NO_PAD.encode(workspace)),
        );
        serde_json_canonicalizer::to_vec(&serde_json::Value::Object(top)).expect("canonical")
    }

    /// Sidecar bytes for password-recovery E2E using the checked-in v1 golden profile.
    pub(crate) fn build_wrapper_golden_v1_slots() -> (ProtocolId, ProtocolId, Vec<u8>, [u8; 32]) {
        use crate::engine::wrapper_golden_v1::{
            GOLDEN_ARGON_SALT_V1, GOLDEN_DATABASE_ID_V1, GOLDEN_DBK_PLAINTEXT_V1,
            GOLDEN_DEVICE_DBK_NONCE_V1, GOLDEN_DEVICE_KEK_V1,
            GOLDEN_PWK_KEK_CIPHERTEXT_V1, GOLDEN_PWK_KEK_NONCE_V1, GOLDEN_RECOVERY_DBK_CIPHERTEXT_V1,
            GOLDEN_RECOVERY_DBK_NONCE_V1, GOLDEN_WORKSPACE_ID_V1,
        };
        use crate::engine::wrappers::{DeviceDbkAad, DeviceKek, WRAPPED_RECORD_LEN};

        fn rid(b: u8) -> ProtocolId {
            [b; 32]
        }

        let recovery = RecoverySubrecord {
            argon_profile_tag: ARGON_PROFILE_TAG_V1,
            argon_salt: GOLDEN_ARGON_SALT_V1,
            recovery_record_id: rid(0x13),
            recovery_kek_id: rid(0x14),
            kek_nonce: GOLDEN_PWK_KEK_NONCE_V1,
            kek_ciphertext: GOLDEN_PWK_KEK_CIPHERTEXT_V1,
            dbk_nonce: GOLDEN_RECOVERY_DBK_NONCE_V1,
            dbk_ciphertext: GOLDEN_RECOVERY_DBK_CIPHERTEXT_V1,
        };
        let commitment = recovery_slot_commitment(&recovery).expect("commitment");

        let device_aad = DeviceDbkAad {
            workspace_id: GOLDEN_WORKSPACE_ID_V1,
            database_id: GOLDEN_DATABASE_ID_V1,
            protector_record_id: rid(0x03),
            device_wrapper_id: rid(0x04),
            recovery_slot_commitment: commitment,
        };
        let device_kek = DeviceKek::from_bytes(GOLDEN_DEVICE_KEK_V1);
        let device_dbk = crate::engine::wrappers::wrap_device_dbk(
            &device_kek,
            &device_aad,
            &GOLDEN_DBK_PLAINTEXT_V1,
            &GOLDEN_DEVICE_DBK_NONCE_V1,
        )
        .expect("device wrap for golden sidecar");
        assert_eq!(device_dbk.ciphertext.len(), WRAPPED_RECORD_LEN);

        let recovery_value = recovery_subrecord_value(&recovery).expect("recovery value");
        let mut device_object = serde_json::Map::new();
        device_object.insert(
            "dbk_ciphertext".into(),
            serde_json::Value::String(URL_SAFE_NO_PAD.encode(device_dbk.ciphertext)),
        );
        device_object.insert(
            "dbk_nonce".into(),
            serde_json::Value::String(URL_SAFE_NO_PAD.encode(device_dbk.nonce)),
        );
        device_object.insert(
            "device_wrapper_id".into(),
            serde_json::Value::String(URL_SAFE_NO_PAD.encode(rid(0x04))),
        );
        device_object.insert(
            "protector_record_id".into(),
            serde_json::Value::String(URL_SAFE_NO_PAD.encode(rid(0x03))),
        );
        device_object.insert(
            "recovery_slot_commitment".into(),
            serde_json::Value::String(URL_SAFE_NO_PAD.encode(commitment)),
        );
        let mut top = serde_json::Map::new();
        top.insert(
            "database_id".into(),
            serde_json::Value::String(URL_SAFE_NO_PAD.encode(GOLDEN_DATABASE_ID_V1)),
        );
        top.insert("db_key_epoch".into(), serde_json::Value::String("1".into()));
        top.insert("device".into(), serde_json::Value::Object(device_object));
        top.insert("format_version".into(), serde_json::Value::String("2".into()));
        top.insert("recovery".into(), recovery_value);
        top.insert("suite_version".into(), serde_json::Value::String("1".into()));
        top.insert(
            "workspace_id".into(),
            serde_json::Value::String(URL_SAFE_NO_PAD.encode(GOLDEN_WORKSPACE_ID_V1)),
        );
        let bytes =
            serde_json_canonicalizer::to_vec(&serde_json::Value::Object(top)).expect("canonical");
        (
            GOLDEN_WORKSPACE_ID_V1,
            GOLDEN_DATABASE_ID_V1,
            bytes,
            GOLDEN_DBK_PLAINTEXT_V1,
        )
    }
}

#[cfg(test)]
pub(crate) use test_support::{build_test_canonical_slots, build_wrapper_golden_v1_slots};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::wrappers::{DeviceKek, Pwk, RecoveryKek};

    fn id(byte: u8) -> ProtocolId {
        [byte; 32]
    }

    #[test]
    fn vault_slots_rejects_unknown_top_level_field() {
        let workspace = id(0x10);
        let database = id(0x20);
        let bytes = build_test_canonical_slots(
            workspace,
            database,
            &DeviceKek::from_bytes([0x01; 32]),
            &RecoveryKek::from_bytes([0x02; 32]),
            &Pwk::from_bytes([0x03; 32]),
            [0x44; 32],
        );
        let mut value: serde_json::Value =
            serde_json::from_slice(&bytes).expect("parse fixture json");
        if let Some(object) = value.as_object_mut() {
            object.insert("extra".to_string(), serde_json::Value::String("x".to_string()));
        }
        let bad = serde_json_canonicalizer::to_vec(&value).expect("canonical bad");
        assert!(parse_vault_slots(&bad, &workspace, &database).is_err());
    }

    #[test]
    fn vault_slots_rejects_recovery_mutation_without_commitment_update() {
        let workspace = id(0x10);
        let database = id(0x20);
        let bytes = build_test_canonical_slots(
            workspace,
            database,
            &DeviceKek::from_bytes([0x01; 32]),
            &RecoveryKek::from_bytes([0x02; 32]),
            &Pwk::from_bytes([0x03; 32]),
            [0x44; 32],
        );
        let mut value: serde_json::Value =
            serde_json::from_slice(&bytes).expect("parse fixture json");
        if let Some(recovery) = value.get_mut("recovery").and_then(|v| v.as_object_mut()) {
            recovery.insert(
                "recovery_record_id".to_string(),
                serde_json::Value::String(URL_SAFE_NO_PAD.encode(id(0xff))),
            );
        }
        let bad = serde_json_canonicalizer::to_vec(&value).expect("canonical bad");
        assert!(parse_vault_slots(&bad, &workspace, &database).is_err());
    }

    #[test]
    fn vault_slots_accepts_valid_fixture_and_rejects_cross_workspace() {
        let workspace = id(0x10);
        let database = id(0x20);
        let bytes = build_test_canonical_slots(
            workspace,
            database,
            &DeviceKek::from_bytes([0x01; 32]),
            &RecoveryKek::from_bytes([0x02; 32]),
            &Pwk::from_bytes([0x03; 32]),
            [0x44; 32],
        );
        let parsed = parse_vault_slots(&bytes, &workspace, &database).expect("parse");
        assert_eq!(parsed.workspace_id, workspace);
        assert!(parse_vault_slots(&bytes, &id(0x99), &database).is_err());
    }
}

//! Recovery read-only session and dual-root unlock orchestration (Task 2).

use crate::engine::key_slots::{parse_vault_slots, AdmissionCounters, VaultSlotsRecord};
use crate::engine::platform::{DeviceStoreError, NativeDeviceStore};
use crate::engine::process::CryptoProcessOwner;
use crate::engine::recovery_authorizer;
use crate::engine::schema::VaultSchemaBinding;
use crate::engine::secret::DbKey;
use crate::engine::sqlcipher::{open_sqlcipher, ConnectionMode, HardenedConnection, OpenError};
use crate::engine::wrappers::{
    dbk_matches_expected, unwrap_device_dbk, unwrap_recovery_dbk, unwrap_recovery_kek_with_pwk,
    DeviceDbkAad, DeviceKek, ProtocolId, Pwk, PwkRecoveryKekAad, RecoveryDbkAad, RecoveryKek,
    WrappedRecord, DATABASE_ROLE_LIVE,
};
use argon2::{Algorithm, Argon2, Params, Version};
use std::path::Path;
use zeroize::Zeroize;

const ARGON_M_COST_KIB: u32 = 65_536;
const ARGON_T_COST: u32 = 3;
const ARGON_P_COST: u32 = 4;
const ARGON_OUTPUT_LEN: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RecoveryFailed;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VaultOpenError {
    Slots,
    DeviceStore(DeviceStoreError),
    Crypto,
    Database(OpenError),
}

/// Read-only recovery session — no write transition in Program 1A Task 2.
pub(crate) struct RecoverySession<State> {
    connection: HardenedConnection,
    _state: State,
}

pub(crate) struct ReadOnly;

impl RecoverySession<ReadOnly> {
    pub(crate) fn connection(&self) -> &HardenedConnection {
        &self.connection
    }
}

struct ArgonWorkspace(ZeroizingArgonBlocks);

struct ZeroizingArgonBlocks(zeroize::Zeroizing<Vec<argon2::Block>>);

impl ArgonWorkspace {
    fn new() -> Result<Self, RecoveryFailed> {
        let mut blocks = Vec::new();
        let params = argon_params()?;
        blocks
            .try_reserve_exact(params.block_count())
            .map_err(|_| RecoveryFailed)?;
        blocks.resize_with(params.block_count(), argon2::Block::default);
        Ok(Self(ZeroizingArgonBlocks(zeroize::Zeroizing::new(blocks))))
    }

    fn blocks_mut(&mut self) -> &mut [argon2::Block] {
        self.0 .0.as_mut_slice()
    }
}

fn argon_params() -> Result<Params, RecoveryFailed> {
    Params::new(
        ARGON_M_COST_KIB,
        ARGON_T_COST,
        ARGON_P_COST,
        Some(ARGON_OUTPUT_LEN),
    )
    .map_err(|_| RecoveryFailed)
}

fn derive_pwk(
    password: &[u8],
    salt: &[u8; 16],
    workspace: &mut ArgonWorkspace,
) -> Result<Pwk, RecoveryFailed> {
    if password.is_empty() || password.len() > 1024 {
        return Err(RecoveryFailed);
    }
    let params = argon_params()?;
    if workspace.blocks_mut().len() != params.block_count() {
        return Err(RecoveryFailed);
    }
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut out = [0u8; ARGON_OUTPUT_LEN];
    argon
        .hash_password_into_with_memory(password, salt, &mut out, workspace.blocks_mut())
        .map_err(|_| RecoveryFailed)?;
    Ok(Pwk::from_bytes(out))
}

pub(crate) fn open_with_device_store(
    owner: &CryptoProcessOwner,
    db_path: &Path,
    slots_bytes: &[u8],
    workspace_id: &ProtocolId,
    database_id: &ProtocolId,
    _protector_id: &ProtocolId,
    store: &NativeDeviceStore,
    counters: &AdmissionCounters,
) -> Result<DbKey, VaultOpenError> {
    let slots = parse_vault_slots(slots_bytes, workspace_id, database_id)
        .map_err(|_| VaultOpenError::Slots)?;
    counters
        .keyring
        .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let device_kek = store
        .get_device_kek()
        .map_err(VaultOpenError::DeviceStore)?;
    open_dbk_via_device_kek(owner, db_path, &slots, &device_kek)
}

#[cfg(test)]
pub(crate) fn open_with_test_device_store(
    owner: &CryptoProcessOwner,
    db_path: &Path,
    slots_bytes: &[u8],
    workspace_id: &ProtocolId,
    database_id: &ProtocolId,
    store: &crate::engine::platform::TestOnlyDeviceStore,
    counters: &AdmissionCounters,
) -> Result<DbKey, VaultOpenError> {
    let slots = parse_vault_slots(slots_bytes, workspace_id, database_id)
        .map_err(|_| VaultOpenError::Slots)?;
    counters
        .keyring
        .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let device_kek = store
        .get_device_kek()
        .map_err(VaultOpenError::DeviceStore)?;
    open_dbk_via_device_kek(owner, db_path, &slots, &device_kek)
}

pub(crate) fn open_dbk_via_device_kek(
    _owner: &CryptoProcessOwner,
    _db_path: &Path,
    slots: &VaultSlotsRecord,
    device_kek: &DeviceKek,
) -> Result<DbKey, VaultOpenError> {
    let aad = DeviceDbkAad {
        workspace_id: slots.workspace_id,
        database_id: slots.database_id,
        protector_record_id: slots.device.protector_record_id,
        device_wrapper_id: slots.device.device_wrapper_id,
        recovery_slot_commitment: slots.device.recovery_slot_commitment,
    };
    let record = WrappedRecord {
        nonce: slots.device.dbk_nonce,
        ciphertext: slots.device.dbk_ciphertext,
    };
    let bytes = unwrap_device_dbk(device_kek, &aad, &record).map_err(|_| VaultOpenError::Crypto)?;
    Ok(DbKey::from_bytes(bytes))
}

pub(crate) fn unlock_recovery_read_only(
    owner: &CryptoProcessOwner,
    db_path: &Path,
    slots_bytes: &[u8],
    workspace_id: &ProtocolId,
    database_id: &ProtocolId,
    password: &[u8],
    counters: &AdmissionCounters,
) -> Result<RecoverySession<ReadOnly>, VaultOpenError> {
    let slots = parse_vault_slots(slots_bytes, workspace_id, database_id)
        .map_err(|_| VaultOpenError::Slots)?;
    let mut workspace = ArgonWorkspace::new().map_err(|_| VaultOpenError::Crypto)?;
    counters
        .kdf
        .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let pwk = derive_pwk(password, &slots.recovery.argon_salt, &mut workspace)
        .map_err(|_| VaultOpenError::Crypto)?;
    workspace.0 .0.zeroize();

    let pwk_aad = PwkRecoveryKekAad {
        workspace_id: slots.workspace_id,
        database_id: slots.database_id,
        recovery_record_id: slots.recovery.recovery_record_id,
        recovery_kek_id: slots.recovery.recovery_kek_id,
        argon_salt: slots.recovery.argon_salt,
    };
    let kek_record = WrappedRecord {
        nonce: slots.recovery.kek_nonce,
        ciphertext: slots.recovery.kek_ciphertext,
    };
    counters
        .unwrap
        .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let recovery_kek_bytes = unwrap_recovery_kek_with_pwk(&pwk, &pwk_aad, &kek_record)
        .map_err(|_| VaultOpenError::Crypto)?;
    let recovery_kek = RecoveryKek::from_bytes(recovery_kek_bytes);

    let recovery_aad = RecoveryDbkAad {
        workspace_id: slots.workspace_id,
        database_id: slots.database_id,
        recovery_record_id: slots.recovery.recovery_record_id,
        recovery_kek_id: slots.recovery.recovery_kek_id,
        database_role: DATABASE_ROLE_LIVE,
    };
    let dbk_record = WrappedRecord {
        nonce: slots.recovery.dbk_nonce,
        ciphertext: slots.recovery.dbk_ciphertext,
    };
    counters
        .unwrap
        .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let dbk_bytes = unwrap_recovery_dbk(&recovery_kek, &recovery_aad, &dbk_record)
        .map_err(|_| VaultOpenError::Crypto)?;

    let dbk = DbKey::from_bytes(dbk_bytes);

    let schema_binding = VaultSchemaBinding {
        workspace_id: slots.workspace_id,
        database_id: slots.database_id,
        db_key_epoch: slots.db_key_epoch,
    };
    let connection = open_sqlcipher(
        owner,
        db_path,
        &dbk,
        ConnectionMode::ReadOnlyRecovery,
        Some(&schema_binding),
    )
    .map_err(VaultOpenError::Database)?;
    recovery_authorizer::harden_recovery_connection(connection.rusqlite_connection())
        .map_err(|_| VaultOpenError::Crypto)?;

    // Drop recovery secrets before returning the session.
    drop(recovery_kek);
    drop(pwk);

    Ok(RecoverySession {
        connection,
        _state: ReadOnly,
    })
}

/// Derive PWK for test fixtures (same path as production recovery unlock).
pub(crate) fn fixture_derive_pwk(password: &[u8], salt: &[u8; 16]) -> Result<Pwk, RecoveryFailed> {
    let mut workspace = ArgonWorkspace::new()?;
    let pwk = derive_pwk(password, salt, &mut workspace)?;
    workspace.0 .0.zeroize();
    Ok(pwk)
}

#[cfg(test)]
pub(crate) fn derive_pwk_for_tests(
    password: &[u8],
    salt: &[u8; 16],
) -> Result<Pwk, RecoveryFailed> {
    fixture_derive_pwk(password, salt)
}

/// Task 4 consumes this sealed prepared rotation value.
pub(crate) struct PreparedWrapperRotation {
    pub(crate) candidate_slots: VaultSlotsRecord,
    pub(crate) _secrets: PreparedVerificationSecrets,
}

/// Task 4 consumes this sealed initial-slots value.
pub(crate) struct PreparedInitialSlots {
    pub(crate) candidate_slots: VaultSlotsRecord,
    pub(crate) _secrets: PreparedVerificationSecrets,
}

impl PreparedInitialSlots {
    pub(crate) fn device_route_matches_expected(&self, device_route_dbk: &DbKey) -> bool {
        dbk_matches_expected(
            device_route_dbk.as_bytes(),
            self._secrets._expected_dbk.as_bytes(),
        )
    }
}

pub(crate) struct PreparedVerificationSecrets {
    pub(crate) _pwk: Pwk,
    pub(crate) _recovery_kek: RecoveryKek,
    pub(crate) _expected_dbk: DbKey,
}

/// Internal fixture enrollment: both routes verify before sidecar publication (Task 4).
pub(crate) fn prepare_fixture_initial_slots(
    owner: &CryptoProcessOwner,
    db_path: &Path,
    workspace_id: ProtocolId,
    database_id: ProtocolId,
    device_kek: &DeviceKek,
    recovery_kek: &RecoveryKek,
    pwk: &Pwk,
    expected_dbk: &DbKey,
    recovery_password: &[u8],
) -> Result<PreparedInitialSlots, RecoveryFailed> {
    let bytes = crate::engine::key_slots::build_test_canonical_slots(
        workspace_id,
        database_id,
        device_kek,
        recovery_kek,
        pwk,
        *expected_dbk.as_bytes(),
    );
    let record =
        parse_vault_slots(&bytes, &workspace_id, &database_id).map_err(|_| RecoveryFailed)?;
    let counters = AdmissionCounters::new();
    let device_dbk =
        open_dbk_via_device_kek(owner, db_path, &record, device_kek).map_err(|_| RecoveryFailed)?;
    if !dbk_matches_expected(device_dbk.as_bytes(), expected_dbk.as_bytes()) {
        return Err(RecoveryFailed);
    }
    let _session = unlock_recovery_read_only(
        owner,
        db_path,
        &bytes,
        &workspace_id,
        &database_id,
        recovery_password,
        &counters,
    )
    .map_err(|_| RecoveryFailed)?;
    Ok(PreparedInitialSlots {
        candidate_slots: record,
        _secrets: PreparedVerificationSecrets {
            _pwk: Pwk::from_bytes(*pwk.expose()),
            _recovery_kek: RecoveryKek::from_bytes(*recovery_kek.expose()),
            _expected_dbk: DbKey::from_bytes(*expected_dbk.as_bytes()),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::key_slots::build_test_canonical_slots;
    use crate::engine::platform::TestOnlyDeviceStore;
    use crate::engine::process::bootstrap_crypto_process;
    use crate::engine::sqlcipher::ConnectionMode;
    use crate::engine::wrappers::{DeviceKek, RecoveryKek};
    use static_assertions::assert_not_impl_any;
    use std::path::PathBuf;

    assert_not_impl_any!(RecoverySession<ReadOnly>: Clone, std::fmt::Debug);

    fn owner() -> &'static CryptoProcessOwner {
        bootstrap_crypto_process().expect("openssl")
    }

    fn temp_db(owner: &CryptoProcessOwner, dbk: &DbKey) -> (tempfile::NamedTempFile, PathBuf) {
        let file = tempfile::NamedTempFile::new().expect("temp database file");
        let path = file.path().to_path_buf();
        open_sqlcipher(
            owner,
            &path,
            dbk,
            ConnectionMode::ReadWriteCreateInternal,
            None,
        )
        .expect("create empty encrypted database for test");
        (file, path)
    }

    #[test]
    fn device_route_opens_database_with_injected_store() {
        let workspace = [0x10; 32];
        let database = [0x20; 32];
        let dbk_bytes = [0x44; 32];
        let dbk = DbKey::from_bytes(dbk_bytes);
        let (_file, path) = temp_db(owner(), &dbk);
        let device_kek = DeviceKek::from_bytes([0x01; 32]);
        let slots = build_test_canonical_slots(
            workspace,
            database,
            &device_kek,
            &RecoveryKek::from_bytes([0x02; 32]),
            &Pwk::from_bytes([0x03; 32]),
            dbk_bytes,
        );
        let store = TestOnlyDeviceStore::with_kek(&device_kek);
        let counters = AdmissionCounters::new();
        let opened = open_with_test_device_store(
            owner(),
            &path,
            &slots,
            &workspace,
            &database,
            &store,
            &counters,
        )
        .expect("device open");
        let _ = open_sqlcipher(
            owner(),
            &path,
            &opened,
            ConnectionMode::ReadOnlyRecovery,
            None,
        )
        .expect("dbk from device route opens database");
    }

    #[test]
    fn device_open_fails_when_recovery_subrecord_deleted_from_sidecar() {
        let workspace = [0x10; 32];
        let database = [0x20; 32];
        let dbk_bytes = [0x44; 32];
        let device_kek = DeviceKek::from_bytes([0x01; 32]);
        let slots = build_test_canonical_slots(
            workspace,
            database,
            &device_kek,
            &RecoveryKek::from_bytes([0x02; 32]),
            &Pwk::from_bytes([0x03; 32]),
            dbk_bytes,
        );
        let mut value: serde_json::Value = serde_json::from_slice(&slots).expect("json");
        value.as_object_mut().unwrap().remove("recovery");
        let broken = serde_json::to_vec(&value).unwrap();
        let store = TestOnlyDeviceStore::with_kek(&device_kek);
        let counters = AdmissionCounters::new();
        let (_file, path) = temp_db(owner(), &DbKey::from_bytes(dbk_bytes));
        let result = open_with_test_device_store(
            owner(),
            &path,
            &broken,
            &workspace,
            &database,
            &store,
            &counters,
        );
        assert!(result.is_err());
    }

    #[test]
    fn recovery_password_unlock_opens_read_only_session_with_golden_fixture() {
        use crate::engine::key_slots::build_wrapper_golden_v1_slots;
        use crate::engine::recovery_authorizer::query_only_is_on;
        use crate::engine::wrapper_golden_v1::GOLDEN_RECOVERY_PASSWORD_V1;

        let (workspace, database, slots, dbk_bytes) = build_wrapper_golden_v1_slots();
        let dbk = DbKey::from_bytes(dbk_bytes);
        let (_file, path) = temp_db(owner(), &dbk);
        let counters = AdmissionCounters::new();
        let session = unlock_recovery_read_only(
            owner(),
            &path,
            &slots,
            &workspace,
            &database,
            GOLDEN_RECOVERY_PASSWORD_V1,
            &counters,
        )
        .expect("golden password unlock");
        let conn = session.connection().rusqlite_connection();
        assert!(session.connection().is_db_readonly().expect("readonly"));
        assert!(query_only_is_on(conn).expect("query_only"));
        let names: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_schema")
            .expect("schema read prepare")
            .query_map([], |row| row.get(0))
            .expect("schema read")
            .collect::<Result<Vec<_>, _>>()
            .expect("schema rows");
        assert!(names.is_empty());
        let (kdf, unwrap, keyring) = counters.snapshot();
        assert_eq!(keyring, 0);
        assert_eq!(kdf, 1);
        assert_eq!(unwrap, 2);
    }
}

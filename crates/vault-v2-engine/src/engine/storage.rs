//! Staging directory I/O and the sole durable publisher for `vault.slots` (RFC 0005 Task 4).

use crate::engine::key_slots::{parse_vault_slots, VaultSlotsRecord};
use crate::engine::process::CryptoProcessOwner;
use crate::engine::recovery::{
    open_dbk_via_device_kek, unlock_recovery_read_only, PreparedInitialSlots,
    PreparedWrapperRotation,
};
use crate::engine::wrappers::DeviceKek;
use cap_std::fs::Dir;
use std::io::Write;
use std::path::{Path, PathBuf};
#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering};

pub(crate) const STAGING_DB_NAME: &str = "vault.db";
pub(crate) const STAGING_SLOTS_NAME: &str = "vault.slots";
pub(crate) const STAGING_NEW_SLOTS_NAME: &str = "vault.slots.new";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StorageError {
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ExpectedSidecar {
    Absent,
    Exact(Vec<u8>),
}

/// Test-only failpoint stage for publication fault matrix (0 = disabled).
#[cfg(test)]
pub(crate) static PUBLISH_FAILPOINT: AtomicUsize = AtomicUsize::new(0);

#[cfg(test)]
fn failpoint(stage: usize) -> Result<(), StorageError> {
    if PUBLISH_FAILPOINT.load(Ordering::SeqCst) == stage {
        return Err(StorageError::Rejected);
    }
    Ok(())
}

#[cfg(not(test))]
fn failpoint(_stage: usize) -> Result<(), StorageError> {
    Ok(())
}

pub(crate) struct StagingDirectory {
    pub(crate) root: PathBuf,
    pub(crate) dir: Dir,
}

impl StagingDirectory {
    pub(crate) fn create_private_sibling(
        workspace_parent: &Path,
        opaque_id: &str,
    ) -> Result<Self, StorageError> {
        if opaque_id.is_empty()
            || opaque_id.contains('/')
            || opaque_id.contains('\\')
            || opaque_id.contains("..")
        {
            return Err(StorageError::Rejected);
        }
        let name = format!("vault-v2.staging-{opaque_id}");
        let root = workspace_parent.join(name);
        if root.exists() {
            return Err(StorageError::Rejected);
        }
        std::fs::create_dir_all(&root).map_err(|_| StorageError::Rejected)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700));
        }
        let dir = Dir::open_ambient_dir(&root, cap_std::ambient_authority())
            .map_err(|_| StorageError::Rejected)?;
        Ok(Self { root, dir })
    }

    pub(crate) fn db_path(&self) -> PathBuf {
        self.root.join(STAGING_DB_NAME)
    }

    pub(crate) fn slots_path(&self) -> PathBuf {
        self.root.join(STAGING_SLOTS_NAME)
    }
}

/// Sole consumer of `PreparedInitialSlots` — atomic sidecar publication.
pub(crate) fn publish_initial_slots(
    owner: &CryptoProcessOwner,
    staging: &StagingDirectory,
    db_path: &Path,
    prepared: PreparedInitialSlots,
    expected: ExpectedSidecar,
    workspace_id: &crate::engine::wrappers::ProtocolId,
    database_id: &crate::engine::wrappers::ProtocolId,
    device_kek: &DeviceKek,
    recovery_password: &[u8],
) -> Result<VaultSlotsRecord, StorageError> {
    failpoint(1)?;
    verify_expected_sidecar(&staging.slots_path(), &expected)?;
    failpoint(2)?;
    let new_bytes = prepared.candidate_slots.canonical_bytes.clone();
    write_new_slots_file(&staging.dir, &new_bytes)?;
    failpoint(3)?;
    atomic_replace_slots(&staging.root)?;
    failpoint(4)?;
    fsync_dir(&staging.root)?;
    failpoint(5)?;
    verify_published_slots(
        owner,
        db_path,
        &staging.slots_path(),
        &prepared,
        workspace_id,
        database_id,
        device_kek,
        recovery_password,
    )?;
    let slots_bytes = std::fs::read(staging.slots_path()).map_err(|_| StorageError::Rejected)?;
    parse_vault_slots(&slots_bytes, workspace_id, database_id).map_err(|_| StorageError::Rejected)
}

/// Sole consumer of `PreparedWrapperRotation`.
pub(crate) fn publish_wrapper_rotation(
    owner: &CryptoProcessOwner,
    staging: &StagingDirectory,
    db_path: &Path,
    prepared: PreparedWrapperRotation,
    expected_old: ExpectedSidecar,
    workspace_id: &crate::engine::wrappers::ProtocolId,
    database_id: &crate::engine::wrappers::ProtocolId,
    device_kek: &DeviceKek,
    recovery_password: &[u8],
) -> Result<VaultSlotsRecord, StorageError> {
    failpoint(1)?;
    verify_expected_sidecar(&staging.slots_path(), &expected_old)?;
    failpoint(2)?;
    let new_bytes = prepared.candidate_slots.canonical_bytes.clone();
    write_new_slots_file(&staging.dir, &new_bytes)?;
    failpoint(3)?;
    atomic_replace_slots(&staging.root)?;
    failpoint(4)?;
    fsync_dir(&staging.root)?;
    failpoint(5)?;
    let initial = PreparedInitialSlots {
        candidate_slots: prepared.candidate_slots,
        _secrets: prepared._secrets,
    };
    verify_published_slots(
        owner,
        db_path,
        &staging.slots_path(),
        &initial,
        workspace_id,
        database_id,
        device_kek,
        recovery_password,
    )?;
    let slots_bytes = std::fs::read(staging.slots_path()).map_err(|_| StorageError::Rejected)?;
    parse_vault_slots(&slots_bytes, workspace_id, database_id).map_err(|_| StorageError::Rejected)
}

fn verify_expected_sidecar(path: &Path, expected: &ExpectedSidecar) -> Result<(), StorageError> {
    match expected {
        ExpectedSidecar::Absent => {
            if path.exists() {
                return Err(StorageError::Rejected);
            }
        }
        ExpectedSidecar::Exact(bytes) => {
            let current = std::fs::read(path).map_err(|_| StorageError::Rejected)?;
            if current != *bytes {
                return Err(StorageError::Rejected);
            }
        }
    }
    Ok(())
}

fn write_new_slots_file(dir: &Dir, bytes: &[u8]) -> Result<(), StorageError> {
    let mut file = dir
        .create(STAGING_NEW_SLOTS_NAME)
        .map_err(|_| StorageError::Rejected)?;
    file.write_all(bytes).map_err(|_| StorageError::Rejected)?;
    file.sync_all().map_err(|_| StorageError::Rejected)?;
    Ok(())
}

fn atomic_replace_slots(staging_root: &Path) -> Result<(), StorageError> {
    std::fs::rename(
        staging_root.join(STAGING_NEW_SLOTS_NAME),
        staging_root.join(STAGING_SLOTS_NAME),
    )
    .map_err(|_| StorageError::Rejected)?;
    Ok(())
}

fn fsync_dir(path: &Path) -> Result<(), StorageError> {
    #[cfg(unix)]
    {
        std::fs::File::open(path)
            .and_then(|f| f.sync_all())
            .map_err(|_| StorageError::Rejected)?;
    }
    Ok(())
}

fn verify_published_slots(
    owner: &CryptoProcessOwner,
    db_path: &Path,
    slots_path: &Path,
    prepared: &PreparedInitialSlots,
    workspace_id: &crate::engine::wrappers::ProtocolId,
    database_id: &crate::engine::wrappers::ProtocolId,
    device_kek: &DeviceKek,
    recovery_password: &[u8],
) -> Result<(), StorageError> {
    let on_disk = std::fs::read(slots_path).map_err(|_| StorageError::Rejected)?;
    if on_disk != prepared.candidate_slots.canonical_bytes {
        return Err(StorageError::Rejected);
    }
    let slots = parse_vault_slots(&on_disk, workspace_id, database_id)
        .map_err(|_| StorageError::Rejected)?;
    let counters = crate::engine::key_slots::AdmissionCounters::new();
    let device_dbk = open_dbk_via_device_kek(owner, db_path, &slots, device_kek)
        .map_err(|_| StorageError::Rejected)?;
    let recovery_session = unlock_recovery_read_only(
        owner,
        db_path,
        &on_disk,
        workspace_id,
        database_id,
        recovery_password,
        &counters,
    )
    .map_err(|_| StorageError::Rejected)?;
    drop(recovery_session);
    if !prepared.device_route_matches_expected(&device_dbk) {
        return Err(StorageError::Rejected);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::key_slots::build_test_canonical_slots;
    use crate::engine::process::bootstrap_crypto_process;
    use crate::engine::recovery::{derive_pwk_for_tests, prepare_fixture_initial_slots};
    use crate::engine::schema::{initialize_vault_schema, VaultSchemaBinding};
    use crate::engine::secret::DbKey;
    use crate::engine::sqlcipher::{open_sqlcipher, ConnectionMode};
    use crate::engine::wrappers::{DeviceKek, RecoveryKek};
    use tempfile::tempdir;

    fn owner() -> &'static CryptoProcessOwner {
        bootstrap_crypto_process().expect("openssl")
    }

    fn setup_staging() -> (
        tempfile::TempDir,
        StagingDirectory,
        DbKey,
        VaultSchemaBinding,
    ) {
        let workspace = tempdir().expect("tempdir");
        let staging = StagingDirectory::create_private_sibling(workspace.path(), "testopaque")
            .expect("staging");
        let dbk = DbKey::from_bytes([0x55; 32]);
        let bind = VaultSchemaBinding {
            workspace_id: [0x10; 32],
            database_id: [0x20; 32],
            db_key_epoch: 1,
        };
        let opened = open_sqlcipher(
            owner(),
            &staging.db_path(),
            &dbk,
            ConnectionMode::ReadWriteCreateInternal,
            None,
        )
        .expect("create db");
        initialize_vault_schema(&opened, &bind).expect("schema");
        drop(opened);
        (workspace, staging, dbk, bind)
    }

    #[test]
    fn initial_publish_is_absent_or_new() {
        let (_ws, staging, dbk, bind) = setup_staging();
        let device_kek = DeviceKek::from_bytes([0x01; 32]);
        let recovery_kek = RecoveryKek::from_bytes([0x02; 32]);
        let recovery_password = b"test-recovery-password";
        let pwk = derive_pwk_for_tests(recovery_password, &[0x01; 16]).expect("pwk");
        let prepared = prepare_fixture_initial_slots(
            owner(),
            &staging.db_path(),
            bind.workspace_id,
            bind.database_id,
            &device_kek,
            &recovery_kek,
            &pwk,
            &dbk,
            recovery_password,
        )
        .expect("prepare");
        assert!(!staging.slots_path().exists());
        let _record = publish_initial_slots(
            owner(),
            &staging,
            &staging.db_path(),
            prepared,
            ExpectedSidecar::Absent,
            &bind.workspace_id,
            &bind.database_id,
            &device_kek,
            recovery_password,
        )
        .expect("publish");
        let bytes = std::fs::read(staging.slots_path()).expect("slots on disk");
        assert_eq!(
            bytes,
            build_test_canonical_slots(
                bind.workspace_id,
                bind.database_id,
                &device_kek,
                &recovery_kek,
                &pwk,
                *dbk.as_bytes(),
            )
        );
    }

    #[test]
    fn rotation_publish_is_expected_old_or_new() {
        let (_ws, staging, dbk, bind) = setup_staging();
        let device_kek = DeviceKek::from_bytes([0x01; 32]);
        let recovery_kek = RecoveryKek::from_bytes([0x02; 32]);
        let recovery_password = b"test-recovery-password";
        let pwk = derive_pwk_for_tests(recovery_password, &[0x01; 16]).expect("pwk");
        let prepared = prepare_fixture_initial_slots(
            owner(),
            &staging.db_path(),
            bind.workspace_id,
            bind.database_id,
            &device_kek,
            &recovery_kek,
            &pwk,
            &dbk,
            recovery_password,
        )
        .expect("prepare");
        let old_record = publish_initial_slots(
            owner(),
            &staging,
            &staging.db_path(),
            prepared,
            ExpectedSidecar::Absent,
            &bind.workspace_id,
            &bind.database_id,
            &device_kek,
            recovery_password,
        )
        .expect("initial publish");
        let old_bytes = old_record.canonical_bytes.clone();
        let rotated_password = b"rotated-recovery-passphrase!!!!!";
        let pwk2 = derive_pwk_for_tests(rotated_password, &[0x01; 16]).expect("pwk2");
        let prepared2 = prepare_fixture_initial_slots(
            owner(),
            &staging.db_path(),
            bind.workspace_id,
            bind.database_id,
            &device_kek,
            &recovery_kek,
            &pwk2,
            &dbk,
            rotated_password,
        )
        .expect("prepare rotation");
        let rotation = PreparedWrapperRotation {
            candidate_slots: prepared2.candidate_slots,
            _secrets: prepared2._secrets,
        };
        let _new_record = publish_wrapper_rotation(
            owner(),
            &staging,
            &staging.db_path(),
            rotation,
            ExpectedSidecar::Exact(old_bytes),
            &bind.workspace_id,
            &bind.database_id,
            &device_kek,
            rotated_password,
        )
        .expect("rotation publish");
    }
}

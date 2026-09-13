//! Internal side-by-side legacy import into a private staging directory (RFC 0005 Task 4).

use crate::engine::entropy::SystemEntropy;
use crate::engine::legacy::{open_legacy_dir, read_legacy_vault_read_only, LegacyRoleMapping};
use crate::engine::process::CryptoProcessOwner;
use crate::engine::recovery::{fixture_derive_pwk, prepare_fixture_initial_slots};
use crate::engine::schema::{
    BusinessStateV1, BusinessTransaction, VaultSchemaBinding, VentureProfileV1, ZeroizingChunk,
};
use crate::engine::secret::DbKey;
use crate::engine::sqlcipher::{open_sqlcipher, ConnectionMode};
use crate::engine::storage::{publish_initial_slots, ExpectedSidecar, StagingDirectory};
use crate::engine::wrappers::{DeviceKek, ProtocolId, RecoveryKek};
use sha2::{Digest, Sha256};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExpectedFormatState {
    LegacyAuthoritative,
    StagingOnly,
}

/// Binding issued outside parsed vault files (Program 1A staging only).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ExpectedVaultBinding {
    pub workspace_id: ProtocolId,
    pub database_id: ProtocolId,
    pub format_state: ExpectedFormatState,
    pub activation_epoch: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActivationBlockerState {
    LegacyRoleHandoffRequired,
    UnsupportedLegacyEntry,
}

#[derive(Debug, Clone)]
pub(crate) struct ActivationBlocker {
    pub blocker_id: [u8; 32],
    pub state: ActivationBlockerState,
}

#[derive(Debug, Clone)]
pub(crate) struct VerifiedV2Staging {
    pub db_commitment: [u8; 32],
    pub sidecar_commitment: [u8; 32],
    pub staging_path: std::path::PathBuf,
    pub blockers: Vec<ActivationBlocker>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MigrationError {
    Rejected,
}

/// Import closed business kinds from the legacy root into a new private staging directory.
///
/// Never writes `vault.format`, never mutates the legacy tree, and never activates product v2.
pub(crate) fn import_legacy_side_by_side(
    owner: &CryptoProcessOwner,
    workspace_parent: &Path,
    legacy_vault_path: &Path,
    binding: ExpectedVaultBinding,
    device_kek: &DeviceKek,
    recovery_kek: &RecoveryKek,
    recovery_password: &[u8],
) -> Result<VerifiedV2Staging, MigrationError> {
    if binding.format_state != ExpectedFormatState::StagingOnly || binding.activation_epoch != 0 {
        return Err(MigrationError::Rejected);
    }
    let legacy_dir = open_legacy_dir(legacy_vault_path).map_err(|_| MigrationError::Rejected)?;
    let verified_legacy =
        read_legacy_vault_read_only(&legacy_dir).map_err(|_| MigrationError::Rejected)?;

    let entropy = SystemEntropy::new();
    let mut opaque = [0u8; 16];
    entropy
        .fill16(&mut opaque)
        .map_err(|_| MigrationError::Rejected)?;
    let opaque_id = encode_hex(&opaque);
    let staging = StagingDirectory::create_private_sibling(workspace_parent, &opaque_id)
        .map_err(|_| MigrationError::Rejected)?;

    let mut dbk_bytes = [0u8; 32];
    entropy
        .fill32(&mut dbk_bytes)
        .map_err(|_| MigrationError::Rejected)?;
    let dbk = DbKey::from_bytes(dbk_bytes);

    let schema_binding = VaultSchemaBinding {
        workspace_id: binding.workspace_id,
        database_id: binding.database_id,
        db_key_epoch: 1,
    };

    let opened = open_sqlcipher(
        owner,
        &staging.db_path(),
        &dbk,
        ConnectionMode::ReadWriteCreateInternal,
        None,
    )
    .map_err(|_| MigrationError::Rejected)?;
    crate::engine::schema::initialize_vault_schema(&opened, &schema_binding)
        .map_err(|_| MigrationError::Rejected)?;
    drop(opened);

    let mut blockers = Vec::new();
    if verified_legacy.has_identity_handoff_blockers {
        let mut blocker_id = [0u8; 32];
        entropy
            .fill32(&mut blocker_id)
            .map_err(|_| MigrationError::Rejected)?;
        blockers.push(ActivationBlocker {
            blocker_id,
            state: ActivationBlockerState::LegacyRoleHandoffRequired,
        });
    }
    if verified_legacy.has_unsupported_entries {
        let mut blocker_id = [0u8; 32];
        entropy
            .fill32(&mut blocker_id)
            .map_err(|_| MigrationError::Rejected)?;
        blockers.push(ActivationBlocker {
            blocker_id,
            state: ActivationBlockerState::UnsupportedLegacyEntry,
        });
    }

    let hardened = open_sqlcipher(
        owner,
        &staging.db_path(),
        &dbk,
        ConnectionMode::ReadWrite,
        Some(&schema_binding),
    )
    .map_err(|_| MigrationError::Rejected)?;
    let txn = BusinessTransaction::begin(&hardened, schema_binding)
        .map_err(|_| MigrationError::Rejected)?;

    for entry in &verified_legacy.entries {
        match entry.role {
            LegacyRoleMapping::BusinessStateV1 => {
                let plaintext = entry.plaintext.as_ref().ok_or(MigrationError::Rejected)?;
                let mut object_id = [0u8; 32];
                entropy
                    .fill32(&mut object_id)
                    .map_err(|_| MigrationError::Rejected)?;
                let object = BusinessStateV1 {
                    object_id,
                    revision: 1,
                    chunks: vec![ZeroizingChunk::from_bytes(plaintext.as_slice().to_vec())],
                };
                txn.put_business_state_v1(&object)
                    .map_err(|_| MigrationError::Rejected)?;
            }
            LegacyRoleMapping::VentureProfileV1 => {
                let plaintext = entry.plaintext.as_ref().ok_or(MigrationError::Rejected)?;
                let mut object_id = [0u8; 32];
                entropy
                    .fill32(&mut object_id)
                    .map_err(|_| MigrationError::Rejected)?;
                let object = VentureProfileV1 {
                    object_id,
                    revision: 1,
                    chunks: vec![ZeroizingChunk::from_bytes(plaintext.as_slice().to_vec())],
                };
                txn.put_venture_profile_v1(&object)
                    .map_err(|_| MigrationError::Rejected)?;
            }
            LegacyRoleMapping::BlockedUntilIdentityHandoff
            | LegacyRoleMapping::UnsupportedLegacyRole => {}
        }
    }
    txn.commit().map_err(|_| MigrationError::Rejected)?;
    drop(hardened);

    let pwk =
        fixture_derive_pwk(recovery_password, &[0x01; 16]).map_err(|_| MigrationError::Rejected)?;
    let prepared = prepare_fixture_initial_slots(
        owner,
        &staging.db_path(),
        binding.workspace_id,
        binding.database_id,
        device_kek,
        recovery_kek,
        &pwk,
        &dbk,
        recovery_password,
    )
    .map_err(|_| MigrationError::Rejected)?;

    publish_initial_slots(
        owner,
        &staging,
        &staging.db_path(),
        prepared,
        ExpectedSidecar::Absent,
        &binding.workspace_id,
        &binding.database_id,
        device_kek,
        recovery_password,
    )
    .map_err(|_| MigrationError::Rejected)?;

    let db_bytes = std::fs::read(staging.db_path()).map_err(|_| MigrationError::Rejected)?;
    let slots_bytes = std::fs::read(staging.slots_path()).map_err(|_| MigrationError::Rejected)?;
    let db_commitment = Sha256::digest(&db_bytes).into();
    let sidecar_commitment = Sha256::digest(&slots_bytes).into();

    Ok(VerifiedV2Staging {
        db_commitment,
        sidecar_commitment,
        staging_path: staging.root,
        blockers,
    })
}

fn encode_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write;
        let _ = write!(out, "{byte:02x}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::process::bootstrap_crypto_process;
    use sovereign_vault::Vault;
    use std::fs;
    use tempfile::tempdir;

    fn owner() -> &'static CryptoProcessOwner {
        bootstrap_crypto_process().expect("openssl")
    }

    #[test]
    fn role_keys_block_activation() {
        let dir = tempdir().expect("tempdir");
        let legacy_root = dir.path().join("vault");
        let mut vault = Vault::init(&legacy_root).expect("init");
        vault.put("workspace_graph", b"graph").expect("graph");
        vault
            .put("runtime_authority_key", b"authority-secret")
            .expect("authority");
        drop(vault);

        let device_kek = DeviceKek::from_bytes([0x01; 32]);
        let recovery_kek = RecoveryKek::from_bytes([0x02; 32]);
        let binding = ExpectedVaultBinding {
            workspace_id: [0x10; 32],
            database_id: [0x20; 32],
            format_state: ExpectedFormatState::StagingOnly,
            activation_epoch: 0,
        };
        let staging = import_legacy_side_by_side(
            owner(),
            dir.path(),
            &legacy_root,
            binding,
            &device_kek,
            &recovery_kek,
            b"recovery-passphrase-32-chars-min!!",
        )
        .expect("import");
        assert!(staging
            .blockers
            .iter()
            .any(|b| b.state == ActivationBlockerState::LegacyRoleHandoffRequired));
        assert!(legacy_root.join("vault.key").exists());
        assert!(staging.staging_path.join("vault.db").exists());
    }
}

//! Read-only importer for the exact unversioned `workspace/vault/` layout (RFC 0005 Task 4).
//!
//! AES-256-GCM decrypt only — no encrypt, key generation, or legacy writes.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use cap_std::fs::Dir;
use std::collections::BTreeSet;
use std::io::Read;
use std::path::{Component, Path};

pub(crate) const LEGACY_MANIFEST_MAX_BYTES: usize = 8 * 1024 * 1024;
pub(crate) const LEGACY_ENTRY_JSON_MAX_BYTES: usize = 32 * 1024 * 1024;
pub(crate) const LEGACY_PLAINTEXT_MAX_BYTES: usize = 16 * 1024 * 1024;
const LEGACY_KEY_MAX_BYTES: usize = 256;
const LEGACY_NONCE_LEN: usize = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LegacyImportError {
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LegacyRoleMapping {
    BusinessStateV1,
    VentureProfileV1,
    BlockedUntilIdentityHandoff,
    UnsupportedLegacyRole,
}

pub(crate) fn map_legacy_entry_name(name: &str) -> LegacyRoleMapping {
    match name {
        "workspace_graph" => LegacyRoleMapping::BusinessStateV1,
        "venture_profile" => LegacyRoleMapping::VentureProfileV1,
        "owner_admission_key" | "owner_approval_key" | "runtime_authority_key" => {
            LegacyRoleMapping::BlockedUntilIdentityHandoff
        }
        _ => LegacyRoleMapping::UnsupportedLegacyRole,
    }
}

struct VaultManifest {
    version: u32,
    entries: Vec<String>,
}

struct EncryptedBlob {
    nonce_b64: String,
    ciphertext_b64: String,
}

fn parse_manifest_json(bytes: &[u8]) -> Result<VaultManifest, LegacyImportError> {
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| LegacyImportError::Rejected)?;
    let object = value.as_object().ok_or(LegacyImportError::Rejected)?;
    if object.len() != 2 || !object.contains_key("version") || !object.contains_key("entries") {
        return Err(LegacyImportError::Rejected);
    }
    let version = object
        .get("version")
        .and_then(|v| v.as_u64())
        .and_then(|v| u32::try_from(v).ok())
        .ok_or(LegacyImportError::Rejected)?;
    let entries_value = object.get("entries").ok_or(LegacyImportError::Rejected)?;
    let entries_array = entries_value
        .as_array()
        .ok_or(LegacyImportError::Rejected)?;
    let mut entries = Vec::with_capacity(entries_array.len());
    for item in entries_array {
        let name = item
            .as_str()
            .ok_or(LegacyImportError::Rejected)?
            .to_string();
        entries.push(name);
    }
    Ok(VaultManifest { version, entries })
}

fn parse_encrypted_blob(bytes: &[u8]) -> Result<EncryptedBlob, LegacyImportError> {
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| LegacyImportError::Rejected)?;
    let object = value.as_object().ok_or(LegacyImportError::Rejected)?;
    if object.len() != 2
        || !object.contains_key("nonce_b64")
        || !object.contains_key("ciphertext_b64")
    {
        return Err(LegacyImportError::Rejected);
    }
    let nonce_b64 = object
        .get("nonce_b64")
        .and_then(|v| v.as_str())
        .ok_or(LegacyImportError::Rejected)?
        .to_string();
    let ciphertext_b64 = object
        .get("ciphertext_b64")
        .and_then(|v| v.as_str())
        .ok_or(LegacyImportError::Rejected)?
        .to_string();
    Ok(EncryptedBlob {
        nonce_b64,
        ciphertext_b64,
    })
}

pub(crate) struct LegacyPlaintext(zeroize::Zeroizing<Vec<u8>>);

impl LegacyPlaintext {
    pub(crate) fn as_slice(&self) -> &[u8] {
        &self.0
    }
}

pub(crate) struct VerifiedLegacyEntry {
    pub(crate) name: String,
    pub(crate) role: LegacyRoleMapping,
    pub(crate) plaintext: Option<LegacyPlaintext>,
}

pub(crate) struct VerifiedLegacyVault {
    pub(crate) entries: Vec<VerifiedLegacyEntry>,
    pub(crate) has_identity_handoff_blockers: bool,
    pub(crate) has_unsupported_entries: bool,
}

/// Read and AEAD-verify the legacy root without mutating it.
pub(crate) fn read_legacy_vault_read_only(
    legacy_dir: &Dir,
) -> Result<VerifiedLegacyVault, LegacyImportError> {
    let key = load_vault_key(legacy_dir)?;
    let manifest = load_manifest(legacy_dir)?;
    let listed = manifest
        .as_ref()
        .map(|m| m.entries.clone())
        .unwrap_or_default();
    let on_disk = list_enc_entries(legacy_dir)?;
    verify_manifest_listing(&listed, &on_disk)?;
    let mut entries = Vec::new();
    let mut has_blockers = false;
    let mut has_unsupported = false;
    for name in listed {
        let role = map_legacy_entry_name(&name);
        match role {
            LegacyRoleMapping::UnsupportedLegacyRole => {
                has_unsupported = true;
                entries.push(VerifiedLegacyEntry {
                    name,
                    role,
                    plaintext: None,
                });
                continue;
            }
            LegacyRoleMapping::BlockedUntilIdentityHandoff => {
                has_blockers = true;
                let _ = decrypt_entry(legacy_dir, &key, &name)?;
                entries.push(VerifiedLegacyEntry {
                    name,
                    role,
                    plaintext: None,
                });
                continue;
            }
            LegacyRoleMapping::BusinessStateV1 | LegacyRoleMapping::VentureProfileV1 => {
                let plaintext = decrypt_entry(legacy_dir, &key, &name)?;
                entries.push(VerifiedLegacyEntry {
                    name,
                    role,
                    plaintext: Some(plaintext),
                });
            }
        }
    }
    Ok(VerifiedLegacyVault {
        entries,
        has_identity_handoff_blockers: has_blockers,
        has_unsupported_entries: has_unsupported,
    })
}

fn load_vault_key(legacy_dir: &Dir) -> Result<zeroize::Zeroizing<[u8; 32]>, LegacyImportError> {
    let file = legacy_dir
        .open("vault.key")
        .map_err(|_| LegacyImportError::Rejected)?;
    let mut bytes = Vec::new();
    file.take(LEGACY_KEY_MAX_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| LegacyImportError::Rejected)?;
    if bytes.len() > LEGACY_KEY_MAX_BYTES {
        return Err(LegacyImportError::Rejected);
    }
    let text = std::str::from_utf8(&bytes).map_err(|_| LegacyImportError::Rejected)?;
    let decoded = STANDARD
        .decode(text.trim())
        .map_err(|_| LegacyImportError::Rejected)?;
    let key: [u8; 32] = decoded
        .try_into()
        .map_err(|_| LegacyImportError::Rejected)?;
    Ok(zeroize::Zeroizing::new(key))
}

fn load_manifest(legacy_dir: &Dir) -> Result<Option<VaultManifest>, LegacyImportError> {
    match legacy_dir.open("manifest.json") {
        Ok(file) => {
            let mut bytes = Vec::new();
            file.take(LEGACY_MANIFEST_MAX_BYTES as u64 + 1)
                .read_to_end(&mut bytes)
                .map_err(|_| LegacyImportError::Rejected)?;
            if bytes.len() > LEGACY_MANIFEST_MAX_BYTES {
                return Err(LegacyImportError::Rejected);
            }
            let manifest = parse_manifest_json(&bytes)?;
            if manifest.version != 1 {
                return Err(LegacyImportError::Rejected);
            }
            for name in &manifest.entries {
                validate_entry_name(name)?;
            }
            Ok(Some(manifest))
        }
        Err(_) => Ok(None),
    }
}

fn list_enc_entries(legacy_dir: &Dir) -> Result<BTreeSet<String>, LegacyImportError> {
    let mut names = BTreeSet::new();
    for entry in legacy_dir
        .entries()
        .map_err(|_| LegacyImportError::Rejected)?
    {
        let entry = entry.map_err(|_| LegacyImportError::Rejected)?;
        let file_name = entry.file_name();
        let name = file_name.to_str().ok_or(LegacyImportError::Rejected)?;
        if name == "vault.key" || name == "manifest.json" {
            continue;
        }
        if name.ends_with(".enc") {
            let stem = name
                .strip_suffix(".enc")
                .ok_or(LegacyImportError::Rejected)?;
            validate_entry_name(stem)?;
            if !names.insert(stem.to_string()) {
                return Err(LegacyImportError::Rejected);
            }
        } else {
            return Err(LegacyImportError::Rejected);
        }
    }
    Ok(names)
}

fn verify_manifest_listing(
    listed: &[String],
    on_disk: &BTreeSet<String>,
) -> Result<(), LegacyImportError> {
    let listed_set: BTreeSet<&str> = listed.iter().map(String::as_str).collect();
    if listed_set.len() != listed.len() {
        return Err(LegacyImportError::Rejected);
    }
    let on_disk_set: BTreeSet<&str> = on_disk.iter().map(String::as_str).collect();
    if listed_set != on_disk_set {
        return Err(LegacyImportError::Rejected);
    }
    Ok(())
}

fn validate_entry_name(name: &str) -> Result<(), LegacyImportError> {
    let mut components = Path::new(name).components();
    let normal =
        matches!(components.next(), Some(Component::Normal(_))) && components.next().is_none();
    if !normal || name.contains(['/', '\\']) || name.chars().any(char::is_control) {
        return Err(LegacyImportError::Rejected);
    }
    Ok(())
}

fn decrypt_entry(
    legacy_dir: &Dir,
    key: &zeroize::Zeroizing<[u8; 32]>,
    name: &str,
) -> Result<LegacyPlaintext, LegacyImportError> {
    validate_entry_name(name)?;
    let path = format!("{name}.enc");
    let file = legacy_dir
        .open(&path)
        .map_err(|_| LegacyImportError::Rejected)?;
    let mut bytes = Vec::new();
    file.take(LEGACY_ENTRY_JSON_MAX_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| LegacyImportError::Rejected)?;
    if bytes.len() > LEGACY_ENTRY_JSON_MAX_BYTES {
        return Err(LegacyImportError::Rejected);
    }
    let blob = parse_encrypted_blob(&bytes)?;
    let plaintext = decrypt_blob(key, &blob)?;
    if plaintext.len() > LEGACY_PLAINTEXT_MAX_BYTES {
        return Err(LegacyImportError::Rejected);
    }
    Ok(LegacyPlaintext(zeroize::Zeroizing::new(plaintext)))
}

fn decrypt_blob(
    key: &zeroize::Zeroizing<[u8; 32]>,
    blob: &EncryptedBlob,
) -> Result<Vec<u8>, LegacyImportError> {
    let cipher =
        Aes256Gcm::new_from_slice(key.as_ref()).map_err(|_| LegacyImportError::Rejected)?;
    let nonce_bytes = STANDARD
        .decode(&blob.nonce_b64)
        .map_err(|_| LegacyImportError::Rejected)?;
    if nonce_bytes.len() != LEGACY_NONCE_LEN {
        return Err(LegacyImportError::Rejected);
    }
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = STANDARD
        .decode(&blob.ciphertext_b64)
        .map_err(|_| LegacyImportError::Rejected)?;
    cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|_| LegacyImportError::Rejected)
}

/// Open the legacy directory with no-follow semantics (ambient authority for internal tests only).
pub(crate) fn open_legacy_dir(path: &Path) -> Result<Dir, LegacyImportError> {
    if path.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(LegacyImportError::Rejected);
    }
    cap_std::fs::Dir::open_ambient_dir(path, cap_std::ambient_authority())
        .map_err(|_| LegacyImportError::Rejected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sovereign_vault::Vault;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn missing_key_never_regenerates() {
        let dir = tempdir().expect("tempdir");
        let legacy_root = dir.path().join("vault");
        fs::create_dir_all(&legacy_root).expect("mkdir");
        let mut vault = Vault::init(&legacy_root).expect("init");
        vault.put("workspace_graph", b"graph").expect("put");
        drop(vault);
        fs::remove_file(legacy_root.join("vault.key")).expect("remove key");
        let opened = open_legacy_dir(&legacy_root).expect("open dir");
        assert!(matches!(
            read_legacy_vault_read_only(&opened),
            Err(LegacyImportError::Rejected)
        ));
        assert!(!legacy_root.join("vault.key").exists());
    }

    #[test]
    fn empty_v1_key_only_is_valid() {
        let dir = tempdir().expect("tempdir");
        let legacy_root = dir.path().join("vault");
        fs::create_dir_all(&legacy_root).expect("mkdir");
        let _vault = Vault::init(&legacy_root).expect("init");
        let opened = open_legacy_dir(&legacy_root).expect("open dir");
        let verified = read_legacy_vault_read_only(&opened).expect("read");
        assert!(verified.entries.is_empty());
    }

    #[test]
    fn role_keys_are_verified_but_not_exported_as_business_plaintext() {
        let dir = tempdir().expect("tempdir");
        let legacy_root = dir.path().join("vault");
        let mut vault = Vault::init(&legacy_root).expect("init");
        vault.put("workspace_graph", b"g").expect("graph");
        vault
            .put("owner_admission_key", b"secret-admission")
            .expect("admission");
        drop(vault);
        let opened = open_legacy_dir(&legacy_root).expect("open dir");
        let verified = read_legacy_vault_read_only(&opened).expect("read");
        assert!(verified.has_identity_handoff_blockers);
        let admission = verified
            .entries
            .iter()
            .find(|e| e.name == "owner_admission_key")
            .expect("admission entry");
        assert_eq!(
            admission.role,
            LegacyRoleMapping::BlockedUntilIdentityHandoff
        );
        assert!(admission.plaintext.is_none());
    }

    #[test]
    fn closed_kinds_map_workspace_graph_and_venture_profile() {
        assert_eq!(
            map_legacy_entry_name("workspace_graph"),
            LegacyRoleMapping::BusinessStateV1
        );
        assert_eq!(
            map_legacy_entry_name("venture_profile"),
            LegacyRoleMapping::VentureProfileV1
        );
        assert_eq!(
            map_legacy_entry_name("owner_approval_key"),
            LegacyRoleMapping::BlockedUntilIdentityHandoff
        );
        assert_eq!(
            map_legacy_entry_name("unknown_entry"),
            LegacyRoleMapping::UnsupportedLegacyRole
        );
    }
}

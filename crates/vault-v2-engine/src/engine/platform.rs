//! Native device credential store adapter (`keyring` v1) and test injection.

use crate::engine::wrappers::{DeviceKek, ProtocolId};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use keyring::Entry;
use keyring::Error as KeyringError;
use std::marker::PhantomData;
use zeroize::Zeroize;

const SERVICE: &str = "com.sovereign-founder-os.vault";
const DEVICE_KEK_PREFIX: &[u8] = b"sfo-device-kek-v1\0";
const DEVICE_KEK_RECORD_LEN: usize = DEVICE_KEK_PREFIX.len() + 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeviceStoreError {
    DeviceStoreUnavailable,
    DeviceKeyMissing,
    DeviceRecordInvalid,
    DeviceProviderRecordInvalid,
    DeviceRecordAmbiguous,
    DeviceStoreConfigurationInvalid,
}

/// Production native store — private and `!Send + !Sync`.
pub(crate) struct NativeDeviceStore {
    entry: Entry,
    _thread_bound: PhantomData<*const ()>,
}

impl NativeDeviceStore {
    pub(crate) fn open(
        workspace_id: &ProtocolId,
        protector_id: &ProtocolId,
    ) -> Result<Self, DeviceStoreError> {
        let username = format!(
            "device-kek:{}:{}",
            URL_SAFE_NO_PAD.encode(workspace_id),
            URL_SAFE_NO_PAD.encode(protector_id)
        );
        let entry = Entry::new(SERVICE, &username).map_err(map_entry_error)?;
        Ok(Self {
            entry,
            _thread_bound: PhantomData,
        })
    }

    pub(crate) fn get_device_kek(&self) -> Result<DeviceKek, DeviceStoreError> {
        let secret = self.entry.get_secret().map_err(map_get_error)?;
        parse_device_kek_record(secret)
    }

    #[cfg(any(test, feature = "platform-qualifier"))]
    pub(crate) fn set_device_kek(&self, kek: &DeviceKek) -> Result<(), DeviceStoreError> {
        let mut record = Vec::with_capacity(DEVICE_KEK_RECORD_LEN);
        record.extend_from_slice(DEVICE_KEK_PREFIX);
        record.extend_from_slice(kek.expose());
        self.entry.set_secret(&record).map_err(map_set_error)?;
        record.zeroize();
        Ok(())
    }

    /// Platform qualification only — never used on a product enrollment path.
    #[cfg(feature = "platform-qualifier")]
    pub(crate) fn delete_device_kek(&self) -> Result<(), DeviceStoreError> {
        qualification_namespace_present()?;
        self.entry.delete_credential().map_err(map_set_error)
    }
}

#[cfg(feature = "platform-qualifier")]
pub(crate) fn open_qualification_store(
    workspace_id: &ProtocolId,
    protector_id: &ProtocolId,
) -> Result<NativeDeviceStore, DeviceStoreError> {
    qualification_namespace_present()?;
    NativeDeviceStore::open(workspace_id, protector_id)
}

#[cfg(feature = "platform-qualifier")]
fn qualification_namespace_present() -> Result<(), DeviceStoreError> {
    let namespace = std::env::var("SFO_VAULT_PLATFORM_NAMESPACE").map_err(|_| {
        DeviceStoreError::DeviceStoreConfigurationInvalid
    })?;
    if !namespace.starts_with("sfo-ci:") {
        return Err(DeviceStoreError::DeviceStoreConfigurationInvalid);
    }
    Ok(())
}

/// In-memory injected store for unit tests (crate-private).
#[cfg(test)]
pub(crate) struct TestOnlyDeviceStore {
    available: bool,
    secret: Option<Vec<u8>>,
}

#[cfg(test)]
impl TestOnlyDeviceStore {
    pub(crate) fn unavailable() -> Self {
        Self {
            available: false,
            secret: None,
        }
    }

    pub(crate) fn missing() -> Self {
        Self {
            available: true,
            secret: None,
        }
    }

    pub(crate) fn with_kek(kek: &DeviceKek) -> Self {
        let mut record = Vec::with_capacity(DEVICE_KEK_RECORD_LEN);
        record.extend_from_slice(DEVICE_KEK_PREFIX);
        record.extend_from_slice(kek.expose());
        Self {
            available: true,
            secret: Some(record),
        }
    }

    pub(crate) fn get_device_kek(&self) -> Result<DeviceKek, DeviceStoreError> {
        if !self.available {
            return Err(DeviceStoreError::DeviceStoreUnavailable);
        }
        let secret = self
            .secret
            .clone()
            .ok_or(DeviceStoreError::DeviceKeyMissing)?;
        parse_device_kek_record(secret)
    }
}

fn parse_device_kek_record(mut secret: Vec<u8>) -> Result<DeviceKek, DeviceStoreError> {
    if secret.len() != DEVICE_KEK_RECORD_LEN || !secret.starts_with(DEVICE_KEK_PREFIX) {
        secret.zeroize();
        return Err(DeviceStoreError::DeviceRecordInvalid);
    }
    let mut key = [0u8; 32];
    key.copy_from_slice(&secret[DEVICE_KEK_PREFIX.len()..]);
    secret.zeroize();
    Ok(DeviceKek::from_bytes(key))
}

fn map_entry_error(error: KeyringError) -> DeviceStoreError {
    map_keyring_error(error)
}

fn map_get_error(error: KeyringError) -> DeviceStoreError {
    match error {
        KeyringError::NoEntry => DeviceStoreError::DeviceKeyMissing,
        other => map_keyring_error(other),
    }
}

fn map_set_error(error: KeyringError) -> DeviceStoreError {
    map_keyring_error(error)
}

fn map_keyring_error(error: KeyringError) -> DeviceStoreError {
    match error {
        KeyringError::NoEntry => DeviceStoreError::DeviceKeyMissing,
        KeyringError::BadEncoding(mut bytes) => {
            bytes.zeroize();
            DeviceStoreError::DeviceProviderRecordInvalid
        }
        KeyringError::BadDataFormat(mut bytes, source) => {
            bytes.zeroize();
            drop(source);
            DeviceStoreError::DeviceProviderRecordInvalid
        }
        KeyringError::Ambiguous(_) => DeviceStoreError::DeviceRecordAmbiguous,
        KeyringError::TooLong(_, _) => DeviceStoreError::DeviceStoreConfigurationInvalid,
        KeyringError::Invalid(_, _) => DeviceStoreError::DeviceStoreConfigurationInvalid,
        KeyringError::PlatformFailure(_)
        | KeyringError::NoStorageAccess(_)
        | KeyringError::BadStoreFormat(_)
        | KeyringError::NoDefaultStore
        | KeyringError::NotSupportedByStore(_) => DeviceStoreError::DeviceStoreUnavailable,
        _ => DeviceStoreError::DeviceStoreUnavailable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unavailable_store_returns_without_side_effects() {
        let store = TestOnlyDeviceStore::unavailable();
        assert!(matches!(
            store.get_device_kek(),
            Err(DeviceStoreError::DeviceStoreUnavailable)
        ));
    }

    #[test]
    fn missing_record_returns_device_key_missing() {
        let store = TestOnlyDeviceStore::missing();
        assert!(matches!(
            store.get_device_kek(),
            Err(DeviceStoreError::DeviceKeyMissing)
        ));
    }

    #[test]
    fn valid_injected_record_round_trips() {
        let kek = DeviceKek::from_bytes([0x5a; 32]);
        let store = TestOnlyDeviceStore::with_kek(&kek);
        let loaded = store.get_device_kek().expect("load");
        assert_eq!(loaded.expose(), kek.expose());
    }
}

#[cfg(all(test, feature = "platform-qualifier"))]
mod platform_qualification {
    use super::*;
    use crate::engine::wrappers::ProtocolId;

    fn random_protocol_id() -> ProtocolId {
        let mut id = [0u8; 32];
        getrandom::fill(&mut id).expect("rng");
        id
    }

    #[test]
    fn invalid_namespace_prefix_is_rejected() {
        let saved = std::env::var("SFO_VAULT_PLATFORM_NAMESPACE").ok();
        std::env::set_var("SFO_VAULT_PLATFORM_NAMESPACE", "not-sfo-ci");
        let workspace = random_protocol_id();
        let protector = random_protocol_id();
        assert!(matches!(
            open_qualification_store(&workspace, &protector),
            Err(DeviceStoreError::DeviceStoreConfigurationInvalid)
        ));
        if let Some(value) = saved {
            std::env::set_var("SFO_VAULT_PLATFORM_NAMESPACE", value);
        }
    }

    #[test]
    fn native_store_set_get_delete_roundtrip() {
        let namespace = std::env::var("SFO_VAULT_PLATFORM_NAMESPACE")
            .expect("CI must set SFO_VAULT_PLATFORM_NAMESPACE");
        assert!(
            namespace.starts_with("sfo-ci:"),
            "namespace must be sfo-ci:<run>:<attempt>"
        );
        let workspace = random_protocol_id();
        let protector = random_protocol_id();
        let store = open_qualification_store(&workspace, &protector).expect("open native store");
        let kek = DeviceKek::from_bytes([0x42; 32]);
        store.set_device_kek(&kek).expect("set_secret");
        let loaded = store.get_device_kek().expect("get_secret");
        assert_eq!(loaded.expose(), kek.expose());
        store.delete_device_kek().expect("delete_credential");
        assert!(matches!(
            store.get_device_kek(),
            Err(DeviceStoreError::DeviceKeyMissing)
        ));
    }

    #[test]
    fn admitted_target_env_matches_plan_contract() {
        let admitted = std::env::var("SFO_VAULT_PLATFORM_ADMITTED_TARGET")
            .expect("workflow must set SFO_VAULT_PLATFORM_ADMITTED_TARGET");
        match admitted.as_str() {
            "x86_64-unknown-linux-gnu" | "aarch64-apple-darwin" | "x86_64-pc-windows-msvc" => {}
            other => panic!("unexpected admitted triple: {other}"),
        }
    }
}

use serde::{Deserialize, Serialize};
use sovereign_identity::DeviceIdentity;
use std::path::{Path, PathBuf};

use crate::{hash_bytes, AuditLedger, LedgerError, GENESIS_HASH};

/// v0.1 sidecar body: `{version, workspace_binding, event_count, last_event_hash}`.
pub const LEDGER_HEAD_VERSION_V1: u16 = 1;
/// Amendment 1: v1 fields plus signed `freshness_generation`.
pub const LEDGER_HEAD_VERSION_V2: u16 = 2;
/// Default writes without an enrolled generation stay on the v0.1 body.
pub const LEDGER_HEAD_VERSION: u16 = LEDGER_HEAD_VERSION_V1;

/// Path to the freshness anchor beside `ledger.json`.
pub fn ledger_head_path(ledger_path: &Path) -> PathBuf {
    ledger_path.with_file_name("ledger.head")
}

/// Signed freshness-anchor body (RFC 0007). Field order is load-bearing for hashing.
/// `freshness_generation` is omitted from the hashed v1 body (`skip_serializing_if`)
/// so existing v0.1 signatures stay valid.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LedgerHeadBody {
    pub version: u16,
    pub workspace_binding: String,
    pub event_count: u64,
    pub last_event_hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub freshness_generation: Option<u64>,
}

/// On-disk anchor: body fields plus a device signature over the body hash.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LedgerHead {
    pub version: u16,
    pub workspace_binding: String,
    pub event_count: u64,
    pub last_event_hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub freshness_generation: Option<u64>,
    pub device_signature: String,
}

impl LedgerHead {
    pub fn from_ledger(ledger: &AuditLedger) -> Result<Self, LedgerError> {
        let workspace_binding = ledger
            .trusted_device_public_key_b64()
            .ok_or(LedgerError::MissingAnchor)?
            .to_owned();
        let event_count = ledger.events().len() as u64;
        let last_event_hash = ledger.last_hash();
        Ok(Self {
            version: LEDGER_HEAD_VERSION_V1,
            workspace_binding,
            event_count,
            last_event_hash,
            freshness_generation: None,
            device_signature: String::new(),
        })
    }

    /// Amendment 1 v2 sidecar: v0.1 fields plus the enrolled generation.
    /// Does not change `AuditEventBody`.
    pub fn from_ledger_with_generation(
        ledger: &AuditLedger,
        freshness_generation: u64,
    ) -> Result<Self, LedgerError> {
        let mut head = Self::from_ledger(ledger)?;
        head.version = LEDGER_HEAD_VERSION_V2;
        head.freshness_generation = Some(freshness_generation);
        Ok(head)
    }

    pub fn body(&self) -> LedgerHeadBody {
        LedgerHeadBody {
            version: self.version,
            workspace_binding: self.workspace_binding.clone(),
            event_count: self.event_count,
            last_event_hash: self.last_event_hash.clone(),
            freshness_generation: self.freshness_generation,
        }
    }

    pub fn sign(&mut self, device: &DeviceIdentity) -> Result<(), LedgerError> {
        if device.public_key_b64() != self.workspace_binding {
            return Err(LedgerError::DeviceMismatch);
        }
        let hash = hash_head_body(&self.body());
        self.device_signature = device.sign_legacy_v1(hash.as_bytes());
        Ok(())
    }

    pub fn verify_device_signature(&self) -> Result<(), LedgerError> {
        match (self.version, self.freshness_generation) {
            (LEDGER_HEAD_VERSION_V1, None) | (LEDGER_HEAD_VERSION_V2, Some(_)) => {}
            _ => return Err(LedgerError::InvalidAnchor),
        }
        let hash = hash_head_body(&self.body());
        DeviceIdentity::verify_legacy_v1(
            &self.workspace_binding,
            hash.as_bytes(),
            &self.device_signature,
        )
        .map_err(|_| LedgerError::InvalidAnchor)
    }

    pub fn save(&self, path: &Path) -> Result<(), LedgerError> {
        let json = serde_json::to_vec_pretty(self)?;
        write_atomic_private(path, &json)?;
        Ok(())
    }

    pub fn load(path: &Path) -> Result<Self, LedgerError> {
        let bytes = std::fs::read(path)?;
        let head: Self = serde_json::from_slice(&bytes)?;
        head.verify_device_signature()?;
        Ok(head)
    }
}

pub fn hash_head_body(body: &LedgerHeadBody) -> String {
    let json = serde_json::to_vec(body).expect("ledger head body must serialize");
    hash_bytes(&json)
}

/// When `require_anchor` is true, a non-empty ledger without `ledger.head` fails
/// closed (a previously anchored ledger whose anchor was removed). Legacy roots
/// that were never anchored pass with `require_anchor` false.
pub fn verify_freshness(
    ledger: &AuditLedger,
    head: Option<&LedgerHead>,
    require_anchor: bool,
) -> Result<(), LedgerError> {
    if let Some(anchor) = head {
        return verify_against_anchor(ledger, anchor);
    }
    if ledger.events().is_empty() {
        return Ok(());
    }
    if require_anchor {
        return Err(LedgerError::MissingAnchor);
    }
    Ok(())
}

fn verify_against_anchor(ledger: &AuditLedger, anchor: &LedgerHead) -> Result<(), LedgerError> {
    anchor.verify_device_signature()?;
    let trusted = ledger
        .trusted_device_public_key_b64()
        .ok_or(LedgerError::DeviceMismatch)?;
    if anchor.workspace_binding != trusted {
        return Err(LedgerError::AnchorDeviceMismatch);
    }
    let len = ledger.events().len() as u64;
    if len < anchor.event_count {
        return Err(LedgerError::Rewound);
    }
    if anchor.event_count == 0 {
        if !ledger.events().is_empty() && ledger.events()[0].previous_event_hash != GENESIS_HASH {
            return Err(LedgerError::Forked);
        }
        return Ok(());
    }
    let idx = (anchor.event_count - 1) as usize;
    let at_anchor = ledger.events().get(idx).ok_or(LedgerError::Rewound)?;
    if at_anchor.event_hash != anchor.last_event_hash {
        return Err(LedgerError::Forked);
    }
    Ok(())
}

pub(crate) fn write_atomic_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let temp_path = path.with_extension("tmp");
    let result = (|| {
        #[cfg(unix)]
        {
            use std::fs::OpenOptions;
            use std::os::unix::fs::OpenOptionsExt;
            let mut file = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(&temp_path)?;
            file.write_all(bytes)?;
            file.sync_all()?;
            drop(file);
        }
        #[cfg(not(unix))]
        {
            let mut file = std::fs::File::create(&temp_path)?;
            file.write_all(bytes)?;
            file.sync_all()?;
            drop(file);
        }
        std::fs::rename(&temp_path, path)?;
        #[cfg(unix)]
        if let Some(directory) = path.parent() {
            std::fs::File::open(directory)?.sync_all()?;
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temp_path);
    }
    result
}

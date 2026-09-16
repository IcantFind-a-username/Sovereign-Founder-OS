//! Persist and reload labelled public trust records. Never the secret key.
//!
//! Every row written here is `historical_verify_only`. A restarted process
//! can verify existing attestations with those public keys; it cannot
//! activate the old signer epoch.

use redb::{ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize};
use sovereign_authority::broker::store::{OwnedStore, StoreError};
use sovereign_identity::{ApprovalRole, KeyValidity, RoleTrustStore};

use crate::approval_bridge::PublicTrustRecord;

const PUBLIC_TRUST: TableDefinition<&str, &[u8]> =
    TableDefinition::new("unqualified-fixture-public-trust-v1");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustError {
    Unavailable,
    VerificationFailed,
}

impl From<StoreError> for TrustError {
    fn from(_: StoreError) -> Self {
        Self::Unavailable
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Envelope {
    record: PublicTrustRecord,
    attestation_hex: String,
}

/// Public keys recovered from redb. Verify-only: there is no constructor
/// that turns a record into a live `ApprovalBridge`.
pub struct HistoricalTrust {
    envelopes: Vec<Envelope>,
}

impl HistoricalTrust {
    pub fn load(store: &OwnedStore<'_>) -> Result<Self, TrustError> {
        store.read(|transaction| {
            let table = match transaction.open_table(PUBLIC_TRUST) {
                Ok(table) => table,
                Err(redb::TableError::TableDoesNotExist(_)) => {
                    return Ok(Self {
                        envelopes: Vec::new(),
                    })
                }
                Err(_) => return Err(TrustError::Unavailable),
            };
            let mut envelopes = Vec::new();
            let iter = table.iter().map_err(|_| TrustError::Unavailable)?;
            for entry in iter {
                let (_, value) = entry.map_err(|_| TrustError::Unavailable)?;
                let envelope: Envelope =
                    serde_json::from_slice(value.value()).map_err(|_| TrustError::Unavailable)?;
                envelopes.push(envelope);
            }
            Ok(Self { envelopes })
        })
    }

    pub fn records(&self) -> impl Iterator<Item = &PublicTrustRecord> {
        self.envelopes.iter().map(|envelope| &envelope.record)
    }

    pub fn len(&self) -> usize {
        self.envelopes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.envelopes.is_empty()
    }

    pub fn attestation_bytes(&self, index: usize) -> Result<Vec<u8>, TrustError> {
        let hex = &self
            .envelopes
            .get(index)
            .ok_or(TrustError::Unavailable)?
            .attestation_hex;
        hex::decode(hex).map_err(|_| TrustError::Unavailable)
    }

    /// Verify stored attestations with historical public keys. Success does
    /// not activate the old epoch.
    pub fn verify_stored_attestations(&self, now_unix: i64) -> Result<(), TrustError> {
        let mut store = RoleTrustStore::<ApprovalRole>::new();
        let validity = KeyValidity::new(0, 4_102_444_800).map_err(|_| TrustError::Unavailable)?;
        for record in self.records() {
            if record.status() != crate::approval_bridge::HISTORICAL_VERIFY_ONLY {
                return Err(TrustError::VerificationFailed);
            }
            let public_key = record.public_key_bytes()?;
            store
                .add_key(record.issuer(), public_key, validity)
                .map_err(|_| TrustError::Unavailable)?;
        }
        for envelope in &self.envelopes {
            if envelope.attestation_hex.is_empty() {
                continue;
            }
            let cose =
                hex::decode(&envelope.attestation_hex).map_err(|_| TrustError::Unavailable)?;
            store
                .verify(&cose, envelope.record.issuer(), now_unix)
                .map_err(|_| TrustError::VerificationFailed)?;
        }
        Ok(())
    }
}

pub fn persist_public_trust(
    store: &OwnedStore<'_>,
    record: &PublicTrustRecord,
    attestation: &[u8],
) -> Result<(), TrustError> {
    let envelope = Envelope {
        record: record.clone(),
        attestation_hex: hex::encode(attestation),
    };
    let bytes = serde_json::to_vec(&envelope).map_err(|_| TrustError::Unavailable)?;
    let key = record.signer_epoch_hex().to_owned();
    store.write(|transaction| {
        let mut table = transaction
            .open_table(PUBLIC_TRUST)
            .map_err(|_| TrustError::Unavailable)?;
        table
            .insert(key.as_str(), bytes.as_slice())
            .map_err(|_| TrustError::Unavailable)?;
        Ok(())
    })
}

//! Ephemeral approval signer for the unqualified fixture.
//!
//! After the OS lock and before redb opens, the process generates a
//! `TypedSigner<ApprovalRole>` and a random signer epoch. This bridge owns
//! that signer privately: there is no getter, secret export, generic public
//! signing method, or serialization path. RFC 0003 `approve_invocation` is
//! D04 and is not present here.
//!
//! **Maturity:** Developer Preview fixture. Design Accept ≠ product Current.
//! The label is `unqualified_fixture`; this is not owner admission.

use std::fmt;

use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sovereign_authority::broker::store::OwnedStore;
use sovereign_identity::{ApprovalRole, TypedSigner};

use crate::trust_persist::{self, TrustError};
use crate::BoundaryError;

/// Distinctive so a product release-symbol scan can prove this crate was not
/// linked. Must not appear in `target/release/sovereign`.
pub const SIGNER_NEEDLE: &str = "synthetic-owner-effect-fixture-signer";

/// Honest disk label. Historical records with this label cannot activate a
/// live signer epoch.
pub const UNQUALIFIED_LABEL: &str = "unqualified_fixture";

/// Status written for every persisted public record. Disk never carries a
/// live-signing capability.
pub const HISTORICAL_VERIFY_ONLY: &str = "historical_verify_only";

pub const FIXTURE_ISSUER: &str = "fixture.unqualified.local";

/// Labelled public half of one process's approval signer.
///
/// This is public data. It cannot reconstruct the signer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicTrustRecord {
    label: String,
    signer_epoch: String,
    public_key: String,
    key_id: String,
    issuer: String,
    status: String,
}

impl PublicTrustRecord {
    pub fn new(
        signer_epoch: [u8; 16],
        public_key: [u8; 32],
        key_id: [u8; 32],
        issuer: impl Into<String>,
    ) -> Self {
        Self {
            label: UNQUALIFIED_LABEL.to_owned(),
            signer_epoch: hex::encode(signer_epoch),
            public_key: hex::encode(public_key),
            key_id: hex::encode(key_id),
            issuer: issuer.into(),
            status: HISTORICAL_VERIFY_ONLY.to_owned(),
        }
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn signer_epoch_hex(&self) -> &str {
        &self.signer_epoch
    }

    pub fn public_key_hex(&self) -> &str {
        &self.public_key
    }

    pub fn key_id_hex(&self) -> &str {
        &self.key_id
    }

    pub fn issuer(&self) -> &str {
        &self.issuer
    }

    pub fn status(&self) -> &str {
        &self.status
    }

    pub fn public_key_bytes(&self) -> Result<[u8; 32], TrustError> {
        decode_hex32(&self.public_key)
    }

    pub fn key_id_bytes(&self) -> Result<[u8; 32], TrustError> {
        decode_hex32(&self.key_id)
    }

    pub fn signer_epoch_bytes(&self) -> Result<[u8; 16], TrustError> {
        decode_hex16(&self.signer_epoch)
    }
}

fn decode_hex32(value: &str) -> Result<[u8; 32], TrustError> {
    let bytes = hex::decode(value).map_err(|_| TrustError::Unavailable)?;
    bytes.try_into().map_err(|_| TrustError::Unavailable)
}

fn decode_hex16(value: &str) -> Result<[u8; 16], TrustError> {
    let bytes = hex::decode(value).map_err(|_| TrustError::Unavailable)?;
    bytes.try_into().map_err(|_| TrustError::Unavailable)
}

/// Closed owner of the live approval signer.
///
/// Not `Clone`, not `Serialize`, not `Debug`-derived. The only public
/// cryptographic surface here is the labelled public trust record.
pub struct ApprovalBridge {
    signer: TypedSigner<ApprovalRole>,
    epoch: [u8; 16],
}

impl fmt::Debug for ApprovalBridge {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ApprovalBridge")
            .field("label", &UNQUALIFIED_LABEL)
            .field("needle", &SIGNER_NEEDLE)
            .field("signer", &"<redacted>")
            .field("epoch", &"<redacted>")
            .finish_non_exhaustive()
    }
}

impl ApprovalBridge {
    pub fn generate() -> Result<Self, BoundaryError> {
        let signer = TypedSigner::<ApprovalRole>::generate(FIXTURE_ISSUER)
            .map_err(|_| BoundaryError::SignerUnavailable)?;
        let mut epoch = [0u8; 16];
        OsRng.fill_bytes(&mut epoch);
        Ok(Self { signer, epoch })
    }

    pub fn signer_epoch(&self) -> [u8; 16] {
        self.epoch
    }

    pub fn public_trust_record(&self) -> PublicTrustRecord {
        PublicTrustRecord::new(
            self.epoch,
            self.signer.public_key_bytes(),
            *self.signer.key_id(),
            self.signer.issuer(),
        )
    }

    /// Persist only the labelled public trust record, plus a COSE
    /// attestation over those public bytes. The secret key stays in
    /// process memory.
    pub fn persist_into(&self, store: &OwnedStore<'_>) -> Result<PublicTrustRecord, TrustError> {
        let record = self.public_trust_record();
        let payload = serde_json::to_vec(&record).map_err(|_| TrustError::Unavailable)?;
        let attestation = self
            .signer
            .sign_cose(&payload)
            .map_err(|_| TrustError::Unavailable)?;
        trust_persist::persist_public_trust(store, &record, &attestation)?;
        Ok(record)
    }
}

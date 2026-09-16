//! Ephemeral approval signer for the unqualified fixture.
//!
//! After the OS lock and before redb opens, the process generates a
//! `TypedSigner<ApprovalRole>` and a random signer epoch. This bridge owns
//! that signer privately: there is no getter, secret export, generic public
//! signing method, or serialization path. RFC 0003 evidence is emitted only
//! by [`ApprovalBridge::approve_invocation`].
//!
//! **Maturity:** Developer Preview fixture. Design Accept ≠ product Current.
//! The label is `unqualified_fixture`; this is not owner admission.

use std::fmt;

use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sovereign_artifact::PreparedInvocation;
use sovereign_authority::broker::store::OwnedStore;
use sovereign_capability::approval::{approve_invocation, ApprovalGrantRequest, SignedApprovalV1};
use sovereign_identity::{ApprovalRole, TypedSigner};
use sovereign_policy::PolicyAuthorizationV2;

use crate::effect::{SessionBinding, UnixClock, FIXTURE_AUDIENCE, FIXTURE_VENTURE};
use crate::grant::FreshUvGrant;
use crate::owner_surface::OwnerError;
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

    /// The only signing entry point. Consumes a one-use [`FreshUvGrant`] and
    /// emits RFC 0003 COSE/Ed25519 approval evidence over the existing
    /// canonical claim. There is no generic public `sign` method.
    pub fn approve_invocation(
        &self,
        grant: FreshUvGrant,
        session: &SessionBinding,
        prepared: &PreparedInvocation,
        policy: &PolicyAuthorizationV2,
        now: std::time::Instant,
        now_unix: i64,
    ) -> Result<SignedApprovalV1, OwnerError> {
        if now >= grant.expires_at() {
            return Err(OwnerError::CeremonyExpired);
        }
        if grant.signer_epoch() != self.epoch {
            return Err(OwnerError::EpochMismatch);
        }
        if grant.logout_epoch() != session.logout_epoch {
            return Err(OwnerError::EpochMismatch);
        }
        if grant.session_id() != session.session_id {
            return Err(OwnerError::SessionUnknown);
        }
        if grant.credential_id() != session.credential_id.as_slice() {
            return Err(OwnerError::CredentialMismatch);
        }
        if grant.policy_decision_id() != policy.decision_id() {
            return Err(OwnerError::IntentMismatch);
        }
        if grant.fixture_generation() != session.fixture_generation {
            return Err(OwnerError::EpochMismatch);
        }
        let primary = prepared
            .primary_resource()
            .ok_or(OwnerError::UnknownIntent)?;
        if primary != grant.effect_intent_id().file_stem() {
            return Err(OwnerError::IntentMismatch);
        }
        if grant.challenge_id().is_nil() {
            return Err(OwnerError::UnknownCeremony);
        }
        approve_invocation(
            &self.signer,
            &UnixClock(now_unix),
            ApprovalGrantRequest {
                approver_subject_id: &session.subject.to_string(),
                audience: FIXTURE_AUDIENCE,
                venture_id: FIXTURE_VENTURE,
                subject_id: &session.subject.to_string(),
                session_id: session.session_id,
                policy_decision: policy,
                prepared_invocation: prepared,
                ttl_seconds: 300,
            },
        )
        .map_err(|_| OwnerError::CoordinatorUnavailable)
    }
}

//! Seal an intent, render a synthetic preview, and mint a FreshUvGrant.
//!
//! Dispatch and reservation are D05. This module never writes `.eml`.

use std::collections::{HashMap, HashSet};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};
use sovereign_artifact::PreparedInvocation;
use sovereign_capability::v2::TrustedClock;
use sovereign_contracts::{AutomationLevel, DataClass};
use sovereign_owner::config::CEREMONY_TIMEOUT;
use sovereign_policy::{AuthenticatedPolicyContextV2, PolicyAuthorizationV2, PolicyEngine};
use uuid::Uuid;

use crate::closed_profile::ClosedProfile;
use crate::grant::{FreshUvGrant, FreshUvGrantSpec};
use crate::owner_surface::OwnerError;
use crate::sealed::{EffectIntentId, FixturePreview, SealedPayload, COORDINATOR_REF};

pub const FIXTURE_AUDIENCE: &str = "sovereign-synthetic-owner-effect";
pub const FIXTURE_VENTURE: &str = "synthetic-fixture";

pub struct SessionBinding {
    pub session_id: Uuid,
    pub subject: Uuid,
    pub credential_id: Vec<u8>,
    pub logout_epoch: u64,
    pub fixture_generation: u64,
}

struct PreparedEffect {
    payload: SealedPayload,
    prepared: PreparedInvocation,
    policy: PolicyAuthorizationV2,
    session_id: Uuid,
}

struct PendingApproval {
    intent_id: EffectIntentId,
    session_id: Uuid,
    credential_id: Vec<u8>,
    issued_at: Instant,
}

/// In-process coordinator for D04: seal, preview, fresh UV grant.
pub struct EffectCoordinator {
    generation: u64,
    signer_epoch: [u8; 16],
    closed: ClosedProfile,
    prepared: HashMap<Uuid, PreparedEffect>,
    pending: HashMap<Uuid, PendingApproval>,
    consumed_challenges: HashSet<Uuid>,
}

impl std::fmt::Debug for EffectCoordinator {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("EffectCoordinator")
            .field("generation", &self.generation)
            .field("prepared", &self.prepared.len())
            .finish_non_exhaustive()
    }
}

impl EffectCoordinator {
    pub fn new(generation: u64, signer_epoch: [u8; 16]) -> Result<Self, OwnerError> {
        let closed = ClosedProfile::generate().map_err(|_| OwnerError::CoordinatorUnavailable)?;
        Ok(Self {
            generation,
            signer_epoch,
            closed,
            prepared: HashMap::new(),
            pending: HashMap::new(),
            consumed_challenges: HashSet::new(),
        })
    }

    pub fn bind_signer_epoch(&mut self, signer_epoch: [u8; 16]) {
        self.signer_epoch = signer_epoch;
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn prepare(
        &mut self,
        session: &SessionBinding,
        body: &[u8],
        now_unix: i64,
    ) -> Result<(EffectIntentId, FixturePreview), OwnerError> {
        reject_injected_content(body)?;
        let intent_id = EffectIntentId::allocate();
        let payload = SealedPayload::compose(intent_id);
        let preview = payload.preview();
        let prepared = self
            .closed
            .prepare(intent_id, self.generation, now_unix)
            .map_err(|_| OwnerError::CoordinatorUnavailable)?;
        let subject_id = session.subject.to_string();
        let policy = PolicyEngine::with_clock(UnixClock(now_unix))
            .evaluate_prepared(
                &prepared,
                AuthenticatedPolicyContextV2::new(
                    FIXTURE_AUDIENCE,
                    FIXTURE_VENTURE,
                    &subject_id,
                    session.session_id,
                    DataClass::Green,
                    AutomationLevel::L2ApproveExecute,
                    intent_id.as_uuid(),
                )
                .map_err(|_| OwnerError::CoordinatorUnavailable)?,
            )
            .map_err(|_| OwnerError::CoordinatorUnavailable)?;
        if !policy.requires_approval() {
            return Err(OwnerError::CoordinatorUnavailable);
        }
        self.prepared.insert(
            intent_id.as_uuid(),
            PreparedEffect {
                payload,
                prepared,
                policy,
                session_id: session.session_id,
            },
        );
        Ok((intent_id, preview))
    }

    pub fn preview(&self, intent_id: EffectIntentId) -> Result<FixturePreview, OwnerError> {
        self.prepared
            .get(&intent_id.as_uuid())
            .map(|entry| entry.payload.preview())
            .ok_or(OwnerError::UnknownIntent)
    }

    pub fn bindings(
        &self,
        intent_id: EffectIntentId,
    ) -> Result<(&PreparedInvocation, &PolicyAuthorizationV2), OwnerError> {
        let entry = self
            .prepared
            .get(&intent_id.as_uuid())
            .ok_or(OwnerError::UnknownIntent)?;
        Ok((&entry.prepared, &entry.policy))
    }

    pub fn start_approval(
        &mut self,
        session: &SessionBinding,
        intent_id: EffectIntentId,
        now: Instant,
    ) -> Result<Uuid, OwnerError> {
        let entry = self
            .prepared
            .get(&intent_id.as_uuid())
            .ok_or(OwnerError::UnknownIntent)?;
        if entry.session_id != session.session_id {
            return Err(OwnerError::SessionUnknown);
        }
        let challenge_id = Uuid::new_v4();
        self.pending.insert(
            challenge_id,
            PendingApproval {
                intent_id,
                session_id: session.session_id,
                credential_id: session.credential_id.clone(),
                issued_at: now,
            },
        );
        Ok(challenge_id)
    }

    pub fn finish_approval(
        &mut self,
        session: &SessionBinding,
        body: &[u8],
        now: Instant,
    ) -> Result<FreshUvGrant, OwnerError> {
        let facts = parse_approval_facts(body)?;
        if self.consumed_challenges.contains(&facts.ceremony_id) {
            return Err(OwnerError::GrantReplay);
        }
        let pending = self
            .pending
            .remove(&facts.ceremony_id)
            .ok_or(OwnerError::UnknownCeremony)?;
        self.consumed_challenges.insert(facts.ceremony_id);
        if now.duration_since(pending.issued_at) >= CEREMONY_TIMEOUT {
            return Err(OwnerError::CeremonyExpired);
        }
        if pending.session_id != session.session_id {
            return Err(OwnerError::SessionUnknown);
        }
        if facts.credential_id != pending.credential_id
            || facts.credential_id != session.credential_id
        {
            return Err(OwnerError::CredentialMismatch);
        }
        if !facts.user_verified {
            return Err(OwnerError::UserVerificationMissing);
        }
        let entry = self
            .prepared
            .get(&pending.intent_id.as_uuid())
            .ok_or(OwnerError::UnknownIntent)?;
        Ok(FreshUvGrant::mint(FreshUvGrantSpec {
            session_id: session.session_id,
            logout_epoch: session.logout_epoch,
            credential_id: session.credential_id.clone(),
            challenge_id: facts.ceremony_id,
            signer_epoch: self.signer_epoch,
            effect_intent_id: pending.intent_id,
            policy_decision_id: entry.policy.decision_id(),
            fixture_generation: self.generation,
            expires_at: now + CEREMONY_TIMEOUT,
        }))
    }
}

/// Clock that Capability V2 and policy share.
pub struct UnixClock(pub i64);

impl TrustedClock for UnixClock {
    fn now_unix(&self) -> i64 {
        self.0
    }
}

impl sovereign_policy::TrustedClock for UnixClock {
    fn now(&self) -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::<chrono::Utc>::from_timestamp(self.0, 0)
            .unwrap_or(chrono::DateTime::<chrono::Utc>::UNIX_EPOCH)
    }
}

pub fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

fn reject_injected_content(body: &[u8]) -> Result<(), OwnerError> {
    if body.is_empty() || body == b"{}" {
        return Ok(());
    }
    let value: Value = serde_json::from_slice(body).map_err(|_| OwnerError::BadBody)?;
    let Some(object) = value.as_object() else {
        return Err(OwnerError::GenericInput);
    };
    if object.is_empty() {
        return Ok(());
    }
    for key in object.keys() {
        if matches!(
            key.as_str(),
            "to" | "from"
                | "subject"
                | "recipient"
                | "body"
                | "headers"
                | "rfc5322"
                | "payload"
                | "content"
                | "cc"
                | "bcc"
        ) {
            return Err(OwnerError::HeaderInjection);
        }
    }
    Err(OwnerError::GenericInput)
}

struct ApprovalFacts {
    ceremony_id: Uuid,
    credential_id: Vec<u8>,
    user_verified: bool,
}

fn parse_approval_facts(body: &[u8]) -> Result<ApprovalFacts, OwnerError> {
    let value: Value = serde_json::from_slice(body).map_err(|_| OwnerError::BadBody)?;
    let ceremony_id = value
        .get("ceremony_id")
        .and_then(Value::as_str)
        .and_then(|text| Uuid::parse_str(text).ok())
        .ok_or(OwnerError::BadBody)?;
    let credential_hex = value
        .get("credential_id")
        .and_then(Value::as_str)
        .ok_or(OwnerError::BadBody)?;
    let credential_id = hex::decode(credential_hex).map_err(|_| OwnerError::BadBody)?;
    let user_verified = value
        .get("user_verified")
        .and_then(Value::as_bool)
        .ok_or(OwnerError::BadBody)?;
    Ok(ApprovalFacts {
        ceremony_id,
        credential_id,
        user_verified,
    })
}

pub fn json_preview(preview: FixturePreview) -> Value {
    json!({
        "intent_id": preview.intent_id.file_stem(),
        "header_count": preview.header_count,
        "is_synthetic": preview.is_synthetic,
        "coordinator": COORDINATOR_REF,
    })
}

//! One-use fresh UV grant. Session presence is not approval.
//!
//! Private fields, no public constructor, neither `Clone`, `Serialize`, nor
//! `Debug`. Only a successful fresh UV ceremony in this crate can mint one,
//! and only [`crate::ApprovalBridge::approve_invocation`] may consume it.

use std::time::Instant;

use uuid::Uuid;

use crate::sealed::EffectIntentId;

/// Opaque one-use proof that a fresh UV ceremony bound this exact intent.
pub struct FreshUvGrant {
    session_id: Uuid,
    logout_epoch: u64,
    credential_id: Vec<u8>,
    challenge_id: Uuid,
    signer_epoch: [u8; 16],
    effect_intent_id: EffectIntentId,
    policy_decision_id: Uuid,
    fixture_generation: u64,
    expires_at: Instant,
}

pub(crate) struct FreshUvGrantSpec {
    pub(crate) session_id: Uuid,
    pub(crate) logout_epoch: u64,
    pub(crate) credential_id: Vec<u8>,
    pub(crate) challenge_id: Uuid,
    pub(crate) signer_epoch: [u8; 16],
    pub(crate) effect_intent_id: EffectIntentId,
    pub(crate) policy_decision_id: Uuid,
    pub(crate) fixture_generation: u64,
    pub(crate) expires_at: Instant,
}

impl FreshUvGrant {
    pub(crate) fn mint(spec: FreshUvGrantSpec) -> Self {
        Self {
            session_id: spec.session_id,
            logout_epoch: spec.logout_epoch,
            credential_id: spec.credential_id,
            challenge_id: spec.challenge_id,
            signer_epoch: spec.signer_epoch,
            effect_intent_id: spec.effect_intent_id,
            policy_decision_id: spec.policy_decision_id,
            fixture_generation: spec.fixture_generation,
            expires_at: spec.expires_at,
        }
    }

    pub(crate) fn session_id(&self) -> Uuid {
        self.session_id
    }

    pub(crate) fn logout_epoch(&self) -> u64 {
        self.logout_epoch
    }

    pub(crate) fn credential_id(&self) -> &[u8] {
        &self.credential_id
    }

    pub(crate) fn challenge_id(&self) -> Uuid {
        self.challenge_id
    }

    pub(crate) fn signer_epoch(&self) -> [u8; 16] {
        self.signer_epoch
    }

    pub(crate) fn effect_intent_id(&self) -> EffectIntentId {
        self.effect_intent_id
    }

    pub(crate) fn policy_decision_id(&self) -> Uuid {
        self.policy_decision_id
    }

    pub(crate) fn fixture_generation(&self) -> u64 {
        self.fixture_generation
    }

    pub(crate) fn expires_at(&self) -> Instant {
        self.expires_at
    }
}

#[cfg(test)]
mod proof_invariants {
    use super::FreshUvGrant;

    static_assertions::assert_not_impl_any!(
        FreshUvGrant: Clone,
        std::fmt::Debug,
        serde::Serialize
    );
}

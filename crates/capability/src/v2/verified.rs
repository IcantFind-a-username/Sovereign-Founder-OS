//! Opaque, side-effect-free Capability V2 / RFC 0003 approval proofs.
//!
//! These types exist so a later coordinator can verify once and consume the
//! proofs by value. They have private fields, no public constructor, and no
//! `Clone` / `Serialize` / `Debug` implementation.

use sovereign_artifact::{ArtifactBackend, RiskClass};
use uuid::Uuid;

use super::{
    canonical_claims, compare_invocation_claims, invocation_fingerprint, map_identity_error,
    policy_decision_digest, validate_claim_lifetime, validate_policy_authorization,
    validate_policy_freshness, validate_supported_invocation, AuthorizedCapabilityV2,
    CapabilityClaimsV2, CapabilityTokenV2, CapabilityV2Error, CapabilityV2ValidationContext,
    CapabilityValidatorV2, TrustedClock, CAPABILITY_V2_MAX_PAYLOAD_BYTES,
    CAPABILITY_V2_MAX_POLICY_AGE_SECONDS, CAPABILITY_V2_MAX_TOKEN_BYTES, CAPABILITY_V2_TYPE,
    CAPABILITY_V2_VERSION,
};
use crate::approval::{ApprovalClaimsV1, SignedApprovalV1};

/// Proof that cryptographic, temporal, policy, artifact, and invocation
/// checks succeeded. Process-local replay and durable store consumption are
/// separate steps; constructing this type does not occupy either.
pub struct VerifiedCapabilityV2 {
    claims: CapabilityClaimsV2,
}

impl VerifiedCapabilityV2 {
    fn from_claims(claims: CapabilityClaimsV2) -> Self {
        Self { claims }
    }

    pub fn claims(&self) -> &CapabilityClaimsV2 {
        &self.claims
    }

    pub fn token_id(&self) -> Uuid {
        self.claims.token_id
    }

    pub fn idempotency_key(&self) -> Uuid {
        self.claims.idempotency_key
    }

    pub fn expires_at_unix(&self) -> i64 {
        self.claims.expires_at_unix
    }

    pub fn approval_id(&self) -> Option<Uuid> {
        self.claims
            .approval_evidence
            .as_ref()
            .map(|evidence| evidence.approval_id)
    }

    fn into_claims(self) -> CapabilityClaimsV2 {
        self.claims
    }
}

/// Proof that RFC 0003 approval evidence matched the verified capability.
/// Present only when the decision requires approval.
pub struct VerifiedApprovalV1 {
    claims: ApprovalClaimsV1,
}

impl VerifiedApprovalV1 {
    fn from_claims(claims: ApprovalClaimsV1) -> Self {
        Self { claims }
    }

    pub fn claims(&self) -> &ApprovalClaimsV1 {
        &self.claims
    }

    pub fn approval_id(&self) -> Uuid {
        self.claims.approval_id
    }

    pub fn expires_at_unix(&self) -> i64 {
        self.claims.expires_at_unix
    }
}

impl<C: TrustedClock> CapabilityValidatorV2<C> {
    /// Side-effect-free cryptographic and context verification. Does not
    /// occupy process-local replay state or write any store.
    pub fn verify(
        &self,
        token: &CapabilityTokenV2,
        context: CapabilityV2ValidationContext<'_>,
    ) -> Result<VerifiedCapabilityV2, CapabilityV2Error> {
        let (verified, approval) = self.verify_approved(token, context, None)?;
        debug_assert!(approval.is_none());
        Ok(verified)
    }

    /// Like [`Self::verify`], additionally presenting RFC 0003 signed
    /// approval whenever the token carries approval evidence. Still does not
    /// consume the token, idempotency key, or approval id.
    pub fn verify_approved(
        &self,
        token: &CapabilityTokenV2,
        context: CapabilityV2ValidationContext<'_>,
        approval: Option<&SignedApprovalV1>,
    ) -> Result<(VerifiedCapabilityV2, Option<VerifiedApprovalV1>), CapabilityV2Error> {
        let now_unix = self.clock.now_unix();
        let verified = self.verify_capability(token, &context, now_unix)?;
        let verified_approval =
            self.verify_required_approval(&verified, &context, approval, now_unix)?;
        Ok((verified, verified_approval))
    }

    pub(super) fn consume_verified(
        &mut self,
        verified: VerifiedCapabilityV2,
        approval: Option<VerifiedApprovalV1>,
    ) -> Result<AuthorizedCapabilityV2, CapabilityV2Error> {
        if self.consumed_tokens.contains(&verified.token_id()) {
            return Err(CapabilityV2Error::Replay);
        }
        if let Some(ref approval) = approval {
            if self.consumed_approvals.contains(&approval.approval_id()) {
                return Err(CapabilityV2Error::ApprovalReused);
            }
        }

        let fingerprint = invocation_fingerprint(verified.claims())?;
        if let Some(existing) = self.idempotency.get(&verified.idempotency_key()) {
            if *existing == fingerprint {
                return Err(CapabilityV2Error::IdempotencyReplay);
            }
            return Err(CapabilityV2Error::IdempotencyConflict);
        }

        // Process-local only. Durable token / idempotency / optional
        // approval claims live on the inverted authority plane
        // (`claim_verified`; two-part when this arm is `None`). This crate
        // must not depend on that plane.
        self.consumed_tokens.insert(verified.token_id());
        self.idempotency
            .insert(verified.idempotency_key(), fingerprint);
        if let Some(approval) = approval {
            self.consumed_approvals.insert(approval.approval_id());
        }
        Ok(AuthorizedCapabilityV2 {
            claims: verified.into_claims(),
        })
    }

    fn verify_capability(
        &self,
        token: &CapabilityTokenV2,
        context: &CapabilityV2ValidationContext<'_>,
        now_unix: i64,
    ) -> Result<VerifiedCapabilityV2, CapabilityV2Error> {
        if token.as_bytes().len() > CAPABILITY_V2_MAX_TOKEN_BYTES {
            return Err(CapabilityV2Error::TokenTooLarge);
        }
        let verified = self
            .trust_store
            .verify(token.as_bytes(), &self.expected_issuer, now_unix)
            .map_err(map_identity_error)?;
        if verified.payload().len() > CAPABILITY_V2_MAX_PAYLOAD_BYTES {
            return Err(CapabilityV2Error::ClaimsTooLarge);
        }
        let claims: CapabilityClaimsV2 = serde_json::from_slice(verified.payload())
            .map_err(|_| CapabilityV2Error::InvalidClaims)?;
        let canonical = canonical_claims(&claims)?;
        if canonical != verified.payload() {
            return Err(CapabilityV2Error::NonCanonicalPayload);
        }

        if claims.typ != CAPABILITY_V2_TYPE {
            return Err(CapabilityV2Error::TypeMismatch);
        }
        if claims.version != CAPABILITY_V2_VERSION {
            return Err(CapabilityV2Error::UnsupportedVersion);
        }
        if claims.issuer != self.expected_issuer || verified.issuer() != self.expected_issuer {
            return Err(CapabilityV2Error::IssuerMismatch);
        }
        if claims.issuer_key_id.as_bytes() != verified.key_id() {
            return Err(CapabilityV2Error::IssuerKeyMismatch);
        }
        if claims.audience != self.expected_audience {
            return Err(CapabilityV2Error::AudienceMismatch);
        }
        validate_claim_lifetime(&claims, now_unix)?;
        if claims.max_uses != 1 {
            return Err(CapabilityV2Error::InvalidUseLimit);
        }
        #[cfg(feature = "owner-effect-fixture")]
        let closed_fixture = context
            .prepared_invocation
            .artifact()
            .manifest()
            .is_closed_fixture_profile()
            && claims.risk_class == RiskClass::LowRiskEffectful
            && claims.backend == ArtifactBackend::CoreWasm;
        #[cfg(not(feature = "owner-effect-fixture"))]
        let closed_fixture = false;
        if !closed_fixture {
            if claims.risk_class != RiskClass::PureCompute {
                return Err(CapabilityV2Error::UnsupportedRiskClass);
            }
            if !matches!(
                claims.backend,
                ArtifactBackend::CoreWasm | ArtifactBackend::ComponentWasm
            ) {
                return Err(CapabilityV2Error::BackendDowngradeDenied);
            }
        }
        if claims.venture_id != context.venture_id {
            return Err(CapabilityV2Error::VentureMismatch);
        }
        if claims.subject_id != context.subject_id {
            return Err(CapabilityV2Error::SubjectMismatch);
        }
        if claims.session_id != context.session_id {
            return Err(CapabilityV2Error::SessionMismatch);
        }

        validate_supported_invocation(context.prepared_invocation)?;
        compare_invocation_claims(&claims, context.prepared_invocation)?;
        validate_policy_authorization(
            context.policy_decision,
            &self.expected_audience,
            context.venture_id,
            context.subject_id,
            context.session_id,
            claims.idempotency_key,
            context.prepared_invocation,
        )?;
        let max_policy_age = if claims.approval_evidence.is_some() {
            crate::approval::APPROVAL_MAX_TTL_SECONDS
        } else {
            CAPABILITY_V2_MAX_POLICY_AGE_SECONDS
        };
        validate_policy_freshness(context.policy_decision, now_unix, max_policy_age)?;
        if !context.policy_decision.allowed() {
            return Err(CapabilityV2Error::PolicyDenied);
        }
        if claims.policy_decision_id != context.policy_decision.decision_id()
            || claims.policy_decision_digest != policy_decision_digest(context.policy_decision)?
        {
            return Err(CapabilityV2Error::PolicyAuthorizationMismatch(
                "decision_digest",
            ));
        }
        Ok(VerifiedCapabilityV2::from_claims(claims))
    }

    fn verify_required_approval(
        &self,
        capability: &VerifiedCapabilityV2,
        context: &CapabilityV2ValidationContext<'_>,
        approval: Option<&SignedApprovalV1>,
        now_unix: i64,
    ) -> Result<Option<VerifiedApprovalV1>, CapabilityV2Error> {
        match (
            context.policy_decision.requires_approval(),
            &capability.claims.approval_evidence,
            approval,
        ) {
            (false, None, None) => Ok(None),
            (true, None, _) => Err(CapabilityV2Error::ApprovalEvidenceUnavailable),
            (false, _, Some(_)) | (false, Some(_), None) => {
                Err(CapabilityV2Error::UnexpectedApprovalEvidence)
            }
            (true, Some(_), None) => Err(CapabilityV2Error::ApprovalEvidenceMismatch),
            (true, Some(evidence), Some(approval)) => {
                let trust = self
                    .approvals
                    .as_ref()
                    .ok_or(CapabilityV2Error::ApprovalTrustUnavailable)?;
                let verified = crate::approval::verify_approval(
                    approval,
                    crate::approval::ApprovalVerificationContext {
                        trust: &trust.trust,
                        expected_issuer: &trust.expected_issuer,
                        audience: &self.expected_audience,
                        venture_id: context.venture_id,
                        subject_id: context.subject_id,
                        session_id: context.session_id,
                        policy_decision: context.policy_decision,
                        prepared: context.prepared_invocation,
                        now_unix,
                    },
                )?;
                if verified.approval_id != evidence.approval_id
                    || verified.approver_subject_id != evidence.approver_subject_id
                    || verified.approved_at_unix != evidence.approved_at_unix
                {
                    return Err(CapabilityV2Error::ApprovalEvidenceMismatch);
                }
                Ok(Some(VerifiedApprovalV1::from_claims(verified)))
            }
        }
    }
}

#[cfg(test)]
mod proof_invariants {
    use super::{VerifiedApprovalV1, VerifiedCapabilityV2};

    static_assertions::assert_not_impl_any!(
        VerifiedCapabilityV2: Clone,
        std::fmt::Debug,
        serde::Serialize
    );
    static_assertions::assert_not_impl_any!(
        VerifiedApprovalV1: Clone,
        std::fmt::Debug,
        serde::Serialize
    );
}

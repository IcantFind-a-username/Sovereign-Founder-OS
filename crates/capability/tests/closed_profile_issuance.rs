//! v01-D04: fixture-feature issuance of the closed RFC 0002 Amendment 1 profile.
//!
//! Product `verify()` and default issuance stay `pure_compute`. This file is
//! compiled only with `owner-effect-fixture`.

#![cfg(feature = "owner-effect-fixture")]

use chrono::{DateTime, Duration, Utc};
use sovereign_artifact::{
    AdmissionLimits, ArtifactVerificationIntent, ArtifactVerifier, Digest, OperationSelector,
    PreparedInvocation, RawResourceGrant, TrustedClock as ArtifactClock,
    CLOSED_FIXTURE_OPERATION_ID, CLOSED_FIXTURE_TOOL_ID, CLOSED_FIXTURE_TOOL_VERSION,
    CORE_WASM_ENTRYPOINT, MANIFEST_PROTOCOL_VERSION,
};
use sovereign_capability::approval::{approve_invocation, ApprovalGrantRequest};
use sovereign_capability::v2::{
    CapabilityIssuerV2, CapabilityV2IssueOptions, CapabilityV2IssueRequest,
    CapabilityV2ValidationContext, CapabilityValidatorV2, TrustedClock,
};
use sovereign_contracts::{AutomationLevel, DataClass};
use sovereign_identity::{
    ApprovalRole, AuthorityRole, KeyValidity, PublisherRole, RoleTrustStore, TypedSigner,
};
use sovereign_policy::{AuthenticatedPolicyContextV2, PolicyEngine};
use uuid::Uuid;

const NOW: i64 = 1_800_000_000;
const AUTHORITY_ISSUER: &str = "authority.local";
const APPROVER_ISSUER: &str = "owner-approval.local";
const PUBLISHER_ISSUER: &str = "publisher.local";
const AUDIENCE: &str = "sovereign-runtime";
const VENTURE: &str = "synthetic-fixture";
const SUBJECT: &str = "fixture-subject";
const APPROVER: &str = "unqualified-fixture";
const AUTHORITY_SECRET: [u8; 32] = [0x41; 32];
const APPROVER_SECRET: [u8; 32] = [0x42; 32];
const PUBLISHER_SECRET: [u8; 32] = [0x50; 32];
const SESSION: Uuid = Uuid::from_u128(7);
const INTENT: &str = "0123456789abcdef0123456789abcdef";
const COMPONENT: &[u8] = b"\0asm\x01\0\0\0closed-profile-fixture";

#[derive(Debug, Clone, Copy)]
struct FixedClock(i64);

impl TrustedClock for FixedClock {
    fn now_unix(&self) -> i64 {
        self.0
    }
}

impl ArtifactClock for FixedClock {
    fn now_unix(&self) -> i64 {
        self.0
    }
}

impl sovereign_policy::TrustedClock for FixedClock {
    fn now(&self) -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(self.0, 0).unwrap()
    }
}

fn selector() -> OperationSelector {
    OperationSelector::new(
        CLOSED_FIXTURE_TOOL_ID,
        CLOSED_FIXTURE_TOOL_VERSION,
        CLOSED_FIXTURE_OPERATION_ID,
    )
    .unwrap()
}

fn prepared() -> PreparedInvocation {
    let publisher =
        TypedSigner::<PublisherRole>::from_secret_bytes(PUBLISHER_ISSUER, PUBLISHER_SECRET)
            .unwrap();
    let manifest = serde_json::json!({
        "protocol_version": MANIFEST_PROTOCOL_VERSION,
        "publisher_issuer": PUBLISHER_ISSUER,
        "publisher_key_id": Digest::from_bytes(*publisher.key_id()),
        "component_digest": Digest::of_bytes(COMPONENT),
        "backend": "core_wasm",
        "risk_class": "low_risk_effectful",
        "abi": "sovereign_core_wasm_v2",
        "entrypoint": CORE_WASM_ENTRYPOINT,
        "requested_host_capabilities": [],
        "operations": [{
            "selector": {
                "tool_id": CLOSED_FIXTURE_TOOL_ID,
                "tool_version": CLOSED_FIXTURE_TOOL_VERSION,
                "operation_id": CLOSED_FIXTURE_OPERATION_ID
            },
            "input_limits": { "max_bytes": 4096, "max_depth": 8 },
            "input_schema": {
                "type": "object",
                "properties": {
                    "effect_intent_id": { "type": "string", "max_utf8_bytes": 64 },
                    "fixture_generation": { "type": "string", "max_utf8_bytes": 32 },
                    "operation": { "type": "string", "max_utf8_bytes": 64 },
                    "coordinator": { "type": "string", "max_utf8_bytes": 128 }
                },
                "required": ["effect_intent_id", "fixture_generation", "operation", "coordinator"],
                "max_properties": 4
            },
            "resource_bindings": [{
                "binding_id": "intent",
                "json_pointer": "/effect_intent_id",
                "normalization": "exact_utf8_v1",
                "primary": true
            }]
        }]
    });
    let canonical = serde_json_canonicalizer::to_vec(&manifest).unwrap();
    let signed = publisher.sign_cose(&canonical).unwrap();
    let mut publishers = RoleTrustStore::<PublisherRole>::new();
    publishers
        .trust_signer(&publisher, KeyValidity::new(NOW - 60, NOW + 7_200).unwrap())
        .unwrap();
    let intent = ArtifactVerificationIntent::new(
        PUBLISHER_ISSUER,
        Digest::of_bytes(&signed),
        Digest::of_bytes(COMPONENT),
    )
    .unwrap();
    let artifact =
        ArtifactVerifier::with_clock(&publishers, AdmissionLimits::default(), FixedClock(NOW))
            .verify_closed_fixture_profile(&intent, &signed, COMPONENT)
            .unwrap();
    let input = serde_json::to_vec(&serde_json::json!({
        "effect_intent_id": INTENT,
        "fixture_generation": "1",
        "operation": CLOSED_FIXTURE_OPERATION_ID,
        "coordinator": "synthetic-owner-effect-v2"
    }))
    .unwrap();
    PreparedInvocation::prepare(
        &artifact,
        &selector(),
        &input,
        vec![RawResourceGrant::new("intent", INTENT)],
    )
    .unwrap()
}

#[test]
fn closed_profile_claims_use_exact_wire_tokens() {
    let invocation = prepared();
    assert!(invocation.artifact().manifest().is_closed_fixture_profile());
    let policy = PolicyEngine::with_clock(FixedClock(NOW))
        .evaluate_prepared(
            &invocation,
            AuthenticatedPolicyContextV2::new(
                AUDIENCE,
                VENTURE,
                SUBJECT,
                SESSION,
                DataClass::Green,
                AutomationLevel::L2ApproveExecute,
                Uuid::from_u128(2),
            )
            .unwrap(),
        )
        .unwrap();
    assert!(policy.requires_approval());

    let approver =
        TypedSigner::<ApprovalRole>::from_secret_bytes(APPROVER_ISSUER, APPROVER_SECRET).unwrap();
    let mut approval_trust = RoleTrustStore::<ApprovalRole>::new();
    approval_trust
        .trust_signer(&approver, KeyValidity::new(NOW - 60, NOW + 7_200).unwrap())
        .unwrap();
    let mut issuance_trust = RoleTrustStore::<ApprovalRole>::new();
    issuance_trust
        .trust_signer(&approver, KeyValidity::new(NOW - 60, NOW + 7_200).unwrap())
        .unwrap();
    let approval = approve_invocation(
        &approver,
        &FixedClock(NOW),
        ApprovalGrantRequest {
            approver_subject_id: APPROVER,
            audience: AUDIENCE,
            venture_id: VENTURE,
            subject_id: SUBJECT,
            session_id: SESSION,
            policy_decision: &policy,
            prepared_invocation: &invocation,
            ttl_seconds: 60,
        },
    )
    .unwrap();

    let issuer = CapabilityIssuerV2::new(
        TypedSigner::<AuthorityRole>::from_secret_bytes(AUTHORITY_ISSUER, AUTHORITY_SECRET)
            .unwrap(),
        AUDIENCE,
        FixedClock(NOW),
    )
    .unwrap()
    .with_approval_trust(issuance_trust, APPROVER_ISSUER)
    .unwrap();
    let token = issuer
        .issue_approved(
            CapabilityV2IssueRequest {
                venture_id: VENTURE,
                subject_id: SUBJECT,
                session_id: SESSION,
                policy_decision: &policy,
                prepared_invocation: &invocation,
                options: CapabilityV2IssueOptions {
                    ttl: Duration::seconds(60),
                    idempotency_key: policy.idempotency_key(),
                },
            },
            &approval,
        )
        .unwrap();

    let mut authority_trust = RoleTrustStore::<AuthorityRole>::new();
    authority_trust
        .trust_signer(
            &TypedSigner::<AuthorityRole>::from_secret_bytes(AUTHORITY_ISSUER, AUTHORITY_SECRET)
                .unwrap(),
            KeyValidity::new(NOW - 60, NOW + 7_200).unwrap(),
        )
        .unwrap();
    let validator =
        CapabilityValidatorV2::new(authority_trust, AUTHORITY_ISSUER, AUDIENCE, FixedClock(NOW))
            .unwrap()
            .with_approval_trust(approval_trust, APPROVER_ISSUER)
            .unwrap();
    let (verified, _) = validator
        .verify_approved(
            &token,
            CapabilityV2ValidationContext {
                venture_id: VENTURE,
                subject_id: SUBJECT,
                session_id: SESSION,
                policy_decision: &policy,
                prepared_invocation: &invocation,
            },
            Some(&approval),
        )
        .unwrap();
    let json = serde_json::to_value(verified.claims()).unwrap();
    assert_eq!(json["risk_class"], "low_risk_effectful");
    assert_eq!(json["backend"], "core_wasm");
    assert_eq!(json["tool"]["tool_id"], "local_outbox");
    assert_eq!(json["tool"]["tool_version"], "1.0.0");
    assert_eq!(json["tool"]["operation"], "write_rfc5322");
}

#[test]
fn product_verify_still_rejects_the_closed_profile_bytes() {
    let publisher =
        TypedSigner::<PublisherRole>::from_secret_bytes(PUBLISHER_ISSUER, PUBLISHER_SECRET)
            .unwrap();
    let manifest = serde_json::json!({
        "protocol_version": MANIFEST_PROTOCOL_VERSION,
        "publisher_issuer": PUBLISHER_ISSUER,
        "publisher_key_id": Digest::from_bytes(*publisher.key_id()),
        "component_digest": Digest::of_bytes(COMPONENT),
        "backend": "core_wasm",
        "risk_class": "low_risk_effectful",
        "abi": "sovereign_core_wasm_v2",
        "entrypoint": CORE_WASM_ENTRYPOINT,
        "requested_host_capabilities": [],
        "operations": [{
            "selector": {
                "tool_id": CLOSED_FIXTURE_TOOL_ID,
                "tool_version": CLOSED_FIXTURE_TOOL_VERSION,
                "operation_id": CLOSED_FIXTURE_OPERATION_ID
            },
            "input_limits": { "max_bytes": 4096, "max_depth": 8 },
            "input_schema": {
                "type": "object",
                "properties": {
                    "effect_intent_id": { "type": "string", "max_utf8_bytes": 64 }
                },
                "required": ["effect_intent_id"],
                "max_properties": 1
            },
            "resource_bindings": [{
                "binding_id": "intent",
                "json_pointer": "/effect_intent_id",
                "normalization": "exact_utf8_v1",
                "primary": true
            }]
        }]
    });
    let canonical = serde_json_canonicalizer::to_vec(&manifest).unwrap();
    let signed = publisher.sign_cose(&canonical).unwrap();
    let mut publishers = RoleTrustStore::<PublisherRole>::new();
    publishers
        .trust_signer(&publisher, KeyValidity::new(NOW - 60, NOW + 7_200).unwrap())
        .unwrap();
    let intent = ArtifactVerificationIntent::new(
        PUBLISHER_ISSUER,
        Digest::of_bytes(&signed),
        Digest::of_bytes(COMPONENT),
    )
    .unwrap();
    let err =
        ArtifactVerifier::with_clock(&publishers, AdmissionLimits::default(), FixedClock(NOW))
            .verify(&intent, &signed, COMPONENT)
            .unwrap_err();
    assert_eq!(err, sovereign_artifact::ArtifactError::UnsupportedRiskClass);
}

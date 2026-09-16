//! Named tests for v01-D01: pure Capability V2 verification must not consume.

use std::fs;

use chrono::{DateTime, Duration, Utc};
use sovereign_artifact::{
    AdmissionLimits, ArtifactVerificationIntent, ArtifactVerifier, Digest, OperationSelector,
    PreparedInvocation, RawResourceGrant, TrustedClock as ArtifactClock, CORE_WASM_ENTRYPOINT,
    MANIFEST_PROTOCOL_VERSION,
};
use sovereign_capability::approval::{approve_invocation, ApprovalGrantRequest, SignedApprovalV1};
use sovereign_capability::v2::{
    CapabilityIssuerV2, CapabilityTokenV2, CapabilityV2Error, CapabilityV2IssueOptions,
    CapabilityV2IssueRequest, CapabilityV2ValidationContext, CapabilityValidatorV2, TrustedClock,
};
use sovereign_contracts::{AutomationLevel, DataClass};
use sovereign_identity::{
    ApprovalRole, AuthorityRole, KeyValidity, PublisherRole, RoleTrustStore, TypedSigner,
};
use sovereign_policy::{AuthenticatedPolicyContextV2, PolicyAuthorizationV2, PolicyEngine};
use uuid::Uuid;

const NOW: i64 = 1_800_000_000;
const ISSUER: &str = "authority.local";
const APPROVER_ISSUER: &str = "owner-approval.local";
const AUDIENCE: &str = "sovereign-runtime";
const VENTURE: &str = "venture-alpha";
const SUBJECT: &str = "founder-session-subject";
const PUBLISHER: &str = "publisher.local";
const APPROVER: &str = "human-owner";
const RESOURCE: &str = "draft:alpha";
const AUTHORITY_SECRET: [u8; 32] = [0x41; 32];
const APPROVER_SECRET: [u8; 32] = [0x42; 32];
const PUBLISHER_SECRET: [u8; 32] = [0x50; 32];
const COMPONENT: &[u8] = b"\0asm\x01\0\0\0capability-verify-fixture";
const SESSION: Uuid = Uuid::from_u128(7);

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

fn prepared(content: &str) -> PreparedInvocation {
    let publisher =
        TypedSigner::<PublisherRole>::from_secret_bytes(PUBLISHER, PUBLISHER_SECRET).unwrap();
    let manifest = serde_json::json!({
        "protocol_version": MANIFEST_PROTOCOL_VERSION,
        "publisher_issuer": PUBLISHER,
        "publisher_key_id": Digest::from_bytes(*publisher.key_id()),
        "component_digest": Digest::of_bytes(COMPONENT),
        "backend": "core_wasm",
        "risk_class": "pure_compute",
        "abi": "sovereign_core_wasm_v1",
        "entrypoint": CORE_WASM_ENTRYPOINT,
        "requested_host_capabilities": [],
        "operations": [{
            "selector": {
                "tool_id": "document.transform",
                "tool_version": "1.0.0",
                "operation_id": "render"
            },
            "input_limits": { "max_bytes": 4096, "max_depth": 8 },
            "input_schema": {
                "type": "object",
                "properties": {
                    "content": { "type": "string", "max_utf8_bytes": 2048 },
                    "resource": { "type": "string", "max_utf8_bytes": 256 }
                },
                "required": ["content", "resource"],
                "max_properties": 2
            },
            "resource_bindings": [{
                "binding_id": "primary",
                "json_pointer": "/resource",
                "normalization": "exact_utf8_v1",
                "primary": true
            }]
        }]
    });
    let canonical_manifest = serde_json_canonicalizer::to_vec(&manifest).unwrap();
    let signed_manifest = publisher.sign_cose(&canonical_manifest).unwrap();
    let mut publishers = RoleTrustStore::<PublisherRole>::new();
    publishers
        .trust_signer(&publisher, KeyValidity::new(NOW - 60, NOW + 3_600).unwrap())
        .unwrap();
    let intent = ArtifactVerificationIntent::new(
        PUBLISHER,
        Digest::of_bytes(&signed_manifest),
        Digest::of_bytes(COMPONENT),
    )
    .unwrap();
    let artifact =
        ArtifactVerifier::with_clock(&publishers, AdmissionLimits::default(), FixedClock(NOW))
            .verify(&intent, &signed_manifest, COMPONENT)
            .unwrap();
    let selector = OperationSelector::new("document.transform", "1.0.0", "render").unwrap();
    let input = serde_json::to_vec(&serde_json::json!({
        "content": content,
        "resource": RESOURCE
    }))
    .unwrap();
    PreparedInvocation::prepare(
        &artifact,
        &selector,
        &input,
        vec![RawResourceGrant::new("primary", RESOURCE)],
    )
    .unwrap()
}

fn authorization(
    prepared: &PreparedInvocation,
    automation_level: AutomationLevel,
    session_id: Uuid,
    idempotency_key: Uuid,
) -> PolicyAuthorizationV2 {
    PolicyEngine::with_clock(FixedClock(NOW))
        .evaluate_prepared(
            prepared,
            AuthenticatedPolicyContextV2::new(
                AUDIENCE,
                VENTURE,
                SUBJECT,
                session_id,
                DataClass::Green,
                automation_level,
                idempotency_key,
            )
            .unwrap(),
        )
        .unwrap()
}

fn authority_signer() -> TypedSigner<AuthorityRole> {
    TypedSigner::<AuthorityRole>::from_secret_bytes(ISSUER, AUTHORITY_SECRET).unwrap()
}

fn validator_at(now: i64) -> CapabilityValidatorV2<FixedClock> {
    let mut trust_store = RoleTrustStore::<AuthorityRole>::new();
    trust_store
        .trust_signer(
            &authority_signer(),
            KeyValidity::new(NOW - 60, NOW + 3_600).unwrap(),
        )
        .unwrap();
    CapabilityValidatorV2::new(trust_store, ISSUER, AUDIENCE, FixedClock(now)).unwrap()
}

fn issuer_at(now: i64) -> CapabilityIssuerV2<FixedClock> {
    CapabilityIssuerV2::new(authority_signer(), AUDIENCE, FixedClock(now)).unwrap()
}

fn issue_token(
    issuer: &CapabilityIssuerV2<FixedClock>,
    prepared: &PreparedInvocation,
    decision: &PolicyAuthorizationV2,
) -> CapabilityTokenV2 {
    issuer
        .issue(CapabilityV2IssueRequest {
            venture_id: VENTURE,
            subject_id: SUBJECT,
            session_id: decision.session_id(),
            policy_decision: decision,
            prepared_invocation: prepared,
            options: CapabilityV2IssueOptions {
                ttl: Duration::seconds(60),
                idempotency_key: decision.idempotency_key(),
            },
        })
        .unwrap()
}

fn context<'a>(
    prepared: &'a PreparedInvocation,
    decision: &'a PolicyAuthorizationV2,
) -> CapabilityV2ValidationContext<'a> {
    CapabilityV2ValidationContext {
        venture_id: VENTURE,
        subject_id: SUBJECT,
        session_id: decision.session_id(),
        policy_decision: decision,
        prepared_invocation: prepared,
    }
}

fn approver() -> TypedSigner<ApprovalRole> {
    TypedSigner::<ApprovalRole>::from_secret_bytes(APPROVER_ISSUER, APPROVER_SECRET).unwrap()
}

fn approval_trust() -> RoleTrustStore<ApprovalRole> {
    let mut trust = RoleTrustStore::<ApprovalRole>::new();
    trust
        .trust_signer(
            &approver(),
            KeyValidity::new(NOW - 60, NOW + 7_200).unwrap(),
        )
        .unwrap();
    trust
}

fn approved_issuer_at(now: i64) -> CapabilityIssuerV2<FixedClock> {
    issuer_at(now)
        .with_approval_trust(approval_trust(), APPROVER_ISSUER)
        .unwrap()
}

fn approved_validator_at(now: i64) -> CapabilityValidatorV2<FixedClock> {
    validator_at(now)
        .with_approval_trust(approval_trust(), APPROVER_ISSUER)
        .unwrap()
}

fn approve_at(
    now: i64,
    invocation: &PreparedInvocation,
    policy_decision: &PolicyAuthorizationV2,
) -> SignedApprovalV1 {
    approve_invocation(
        &approver(),
        &FixedClock(now),
        ApprovalGrantRequest {
            approver_subject_id: APPROVER,
            audience: AUDIENCE,
            venture_id: VENTURE,
            subject_id: SUBJECT,
            session_id: SESSION,
            policy_decision,
            prepared_invocation: invocation,
            ttl_seconds: 300,
        },
    )
    .unwrap()
}

fn empty_dir_entries(path: &std::path::Path) -> usize {
    fs::read_dir(path).unwrap().count()
}

#[test]
fn repeated_pure_verification_does_not_mutate_process_local_replay_state() {
    let prepared = prepared("verify-only input");
    let decision = authorization(
        &prepared,
        AutomationLevel::L1Draft,
        Uuid::from_u128(1),
        Uuid::from_u128(2),
    );
    let token = issue_token(&issuer_at(NOW), &prepared, &decision);
    let mut validator = validator_at(NOW);
    let before = format!("{validator:?}");

    let first = validator
        .verify(&token, context(&prepared, &decision))
        .unwrap();
    let token_id = first.token_id();
    for _ in 0..4 {
        let again = validator
            .verify(&token, context(&prepared, &decision))
            .unwrap();
        assert_eq!(again.token_id(), token_id);
    }

    let after_verify = format!("{validator:?}");
    assert_eq!(
        after_verify, before,
        "pure verification must not change process-local validator state"
    );

    let authorized = validator
        .authorize_and_consume(&token, context(&prepared, &decision))
        .unwrap();
    assert_eq!(authorized.token_id(), token_id);
    assert_ne!(
        format!("{validator:?}"),
        before,
        "consume must still occupy process-local replay state, or the snapshot is vacuous"
    );
    assert!(matches!(
        validator.authorize_and_consume(&token, context(&prepared, &decision)),
        Err(CapabilityV2Error::Replay)
    ));
}

#[test]
fn repeated_pure_verification_does_not_mutate_any_attached_store() {
    let prepared = prepared("store-isolation input");
    let decision = authorization(
        &prepared,
        AutomationLevel::L1Draft,
        Uuid::from_u128(3),
        Uuid::from_u128(4),
    );
    let token = issue_token(&issuer_at(NOW), &prepared, &decision);
    let validator = validator_at(NOW);

    let store_root = std::env::temp_dir().join(format!(
        "sovereign-capability-verify-store-{}",
        Uuid::new_v4()
    ));
    fs::create_dir_all(&store_root).unwrap();
    assert_eq!(empty_dir_entries(&store_root), 0);

    for _ in 0..3 {
        validator
            .verify(&token, context(&prepared, &decision))
            .unwrap();
    }

    assert_eq!(
        empty_dir_entries(&store_root),
        0,
        "pure verification must not create store files"
    );
    let debug = format!("{validator:?}");
    assert!(
        !debug.contains(&store_root.display().to_string()),
        "validator must not attach a filesystem store path"
    );
    fs::remove_dir_all(&store_root).unwrap();

    let manifest = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));
    assert!(
        !manifest.contains("sovereign-authority"),
        "capability must not grow an authority dependency that could attach a store"
    );
}

#[test]
fn repeated_pure_approved_verification_does_not_consume_approval() {
    let invocation = prepared("approved verify-only");
    let policy_decision = authorization(
        &invocation,
        AutomationLevel::L3BoundedAuto,
        SESSION,
        Uuid::from_u128(11),
    );
    assert!(policy_decision.requires_approval());
    let approval = approve_at(NOW + 120, &invocation, &policy_decision);
    let token = approved_issuer_at(NOW + 125)
        .issue_approved(
            CapabilityV2IssueRequest {
                venture_id: VENTURE,
                subject_id: SUBJECT,
                session_id: SESSION,
                policy_decision: &policy_decision,
                prepared_invocation: &invocation,
                options: CapabilityV2IssueOptions {
                    ttl: Duration::seconds(60),
                    idempotency_key: policy_decision.idempotency_key(),
                },
            },
            &approval,
        )
        .unwrap();

    let mut validator = approved_validator_at(NOW + 130);
    let before = format!("{validator:?}");
    for _ in 0..3 {
        let (capability, verified_approval) = validator
            .verify_approved(
                &token,
                context(&invocation, &policy_decision),
                Some(&approval),
            )
            .unwrap();
        assert_eq!(
            capability.approval_id(),
            verified_approval
                .as_ref()
                .map(|approval| approval.approval_id())
        );
        assert!(capability.approval_id().is_some());
    }
    assert_eq!(format!("{validator:?}"), before);

    validator
        .authorize_and_consume_approved(
            &token,
            context(&invocation, &policy_decision),
            Some(&approval),
        )
        .unwrap();
    assert!(matches!(
        validator.authorize_and_consume_approved(
            &token,
            context(&invocation, &policy_decision),
            Some(&approval),
        ),
        Err(CapabilityV2Error::Replay)
    ));
}

#[test]
fn capability_package_does_not_depend_on_authority() {
    let manifest = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));
    let dependencies = manifest
        .split("[dependencies]")
        .nth(1)
        .and_then(|rest| rest.split('[').next())
        .expect("dependencies table");
    assert!(
        !dependencies.contains("sovereign-authority"),
        "v01-D01 must not add a capability → authority edge"
    );
}

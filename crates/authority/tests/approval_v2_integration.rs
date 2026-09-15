//! Durable approval/store integration that used to hang off
//! `CapabilityValidatorV2::with_authority_store`.
//!
//! Capability verification is pure. This crate claims the authorized
//! capability through [`AuthorityStore::claim_verified`].

use chrono::{DateTime, Duration, Utc};
use sovereign_artifact::{
    AdmissionLimits, ArtifactVerificationIntent, ArtifactVerifier, Digest, OperationSelector,
    PreparedInvocation, RawResourceGrant, TrustedClock as ArtifactClock, CORE_WASM_ENTRYPOINT,
    MANIFEST_PROTOCOL_VERSION,
};
use sovereign_authority::{AuthorityError, AuthorityStore};
use sovereign_capability::approval::{approve_invocation, ApprovalGrantRequest, SignedApprovalV1};
use sovereign_capability::v2::{
    AuthorizedCapabilityV2, CapabilityIssuerV2, CapabilityTokenV2, CapabilityV2IssueOptions,
    CapabilityV2IssueRequest, CapabilityV2ValidationContext, CapabilityValidatorV2, TrustedClock,
};
use sovereign_contracts::{AutomationLevel, DataClass};
use sovereign_fault_testing::BlockedPath;
use sovereign_identity::{
    ApprovalRole, AuthorityRole, KeyValidity, PublisherRole, RoleTrustStore, TypedSigner,
};
use sovereign_policy::{AuthenticatedPolicyContextV2, PolicyAuthorizationV2, PolicyEngine};
use uuid::Uuid;

const NOW: i64 = 1_800_000_000;
const AUTHORITY_ISSUER: &str = "authority.local";
const APPROVER_ISSUER: &str = "owner-approval.local";
const PUBLISHER_ISSUER: &str = "publisher.local";
const AUDIENCE: &str = "sovereign-runtime";
const VENTURE: &str = "venture-alpha";
const SUBJECT: &str = "founder-session-subject";
const APPROVER: &str = "human-owner";
const RESOURCE: &str = "draft:alpha";
const AUTHORITY_SECRET: [u8; 32] = [0x41; 32];
const APPROVER_SECRET: [u8; 32] = [0x42; 32];
const PUBLISHER_SECRET: [u8; 32] = [0x50; 32];
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

fn selector() -> OperationSelector {
    OperationSelector::new("document.transform", "1.0.0", "render").unwrap()
}

fn prepared(content: &str) -> PreparedInvocation {
    let publisher =
        TypedSigner::<PublisherRole>::from_secret_bytes(PUBLISHER_ISSUER, PUBLISHER_SECRET)
            .unwrap();
    let component: &[u8] = b"\0asm\x01\0\0\0approval-fixture";
    let manifest = serde_json::json!({
        "protocol_version": MANIFEST_PROTOCOL_VERSION,
        "publisher_issuer": PUBLISHER_ISSUER,
        "publisher_key_id": Digest::from_bytes(*publisher.key_id()),
        "component_digest": Digest::of_bytes(component),
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
    let canonical = serde_json_canonicalizer::to_vec(&manifest).unwrap();
    let signed = publisher.sign_cose(&canonical).unwrap();
    let mut publishers = RoleTrustStore::<PublisherRole>::new();
    publishers
        .trust_signer(&publisher, KeyValidity::new(NOW - 60, NOW + 7_200).unwrap())
        .unwrap();
    let intent = ArtifactVerificationIntent::new(
        PUBLISHER_ISSUER,
        Digest::of_bytes(&signed),
        Digest::of_bytes(component),
    )
    .unwrap();
    let artifact =
        ArtifactVerifier::with_clock(&publishers, AdmissionLimits::default(), FixedClock(NOW))
            .verify(&intent, &signed, component)
            .unwrap();
    let input = serde_json::to_vec(&serde_json::json!({
        "content": content,
        "resource": RESOURCE
    }))
    .unwrap();
    PreparedInvocation::prepare(
        &artifact,
        &selector(),
        &input,
        vec![RawResourceGrant::new("primary", RESOURCE)],
    )
    .unwrap()
}

fn decision(
    invocation: &PreparedInvocation,
    level: AutomationLevel,
    idempotency: Uuid,
) -> PolicyAuthorizationV2 {
    PolicyEngine::with_clock(FixedClock(NOW))
        .evaluate_prepared(
            invocation,
            AuthenticatedPolicyContextV2::new(
                AUDIENCE,
                VENTURE,
                SUBJECT,
                SESSION,
                DataClass::Green,
                level,
                idempotency,
            )
            .unwrap(),
        )
        .unwrap()
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

fn issuer_at(now: i64) -> CapabilityIssuerV2<FixedClock> {
    CapabilityIssuerV2::new(
        TypedSigner::<AuthorityRole>::from_secret_bytes(AUTHORITY_ISSUER, AUTHORITY_SECRET)
            .unwrap(),
        AUDIENCE,
        FixedClock(now),
    )
    .unwrap()
    .with_approval_trust(approval_trust(), APPROVER_ISSUER)
    .unwrap()
}

fn validator_at(now: i64) -> CapabilityValidatorV2<FixedClock> {
    let trusted =
        TypedSigner::<AuthorityRole>::from_secret_bytes(AUTHORITY_ISSUER, AUTHORITY_SECRET)
            .unwrap();
    let mut authority_trust = RoleTrustStore::<AuthorityRole>::new();
    authority_trust
        .trust_signer(&trusted, KeyValidity::new(NOW - 60, NOW + 7_200).unwrap())
        .unwrap();
    CapabilityValidatorV2::new(authority_trust, AUTHORITY_ISSUER, AUDIENCE, FixedClock(now))
        .unwrap()
        .with_approval_trust(approval_trust(), APPROVER_ISSUER)
        .unwrap()
}

fn approve_at(
    now: i64,
    invocation: &PreparedInvocation,
    policy_decision: &PolicyAuthorizationV2,
    ttl_seconds: i64,
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
            ttl_seconds,
        },
    )
    .unwrap()
}

fn request_with_ttl<'a>(
    invocation: &'a PreparedInvocation,
    policy_decision: &'a PolicyAuthorizationV2,
    idempotency: Uuid,
    ttl_seconds: i64,
) -> CapabilityV2IssueRequest<'a> {
    CapabilityV2IssueRequest {
        venture_id: VENTURE,
        subject_id: SUBJECT,
        session_id: SESSION,
        policy_decision,
        prepared_invocation: invocation,
        options: CapabilityV2IssueOptions {
            ttl: Duration::seconds(ttl_seconds),
            idempotency_key: idempotency,
        },
    }
}

fn request<'a>(
    invocation: &'a PreparedInvocation,
    policy_decision: &'a PolicyAuthorizationV2,
    idempotency: Uuid,
) -> CapabilityV2IssueRequest<'a> {
    request_with_ttl(invocation, policy_decision, idempotency, 60)
}

fn context<'a>(
    invocation: &'a PreparedInvocation,
    policy_decision: &'a PolicyAuthorizationV2,
) -> CapabilityV2ValidationContext<'a> {
    CapabilityV2ValidationContext {
        venture_id: VENTURE,
        subject_id: SUBJECT,
        session_id: SESSION,
        policy_decision,
        prepared_invocation: invocation,
    }
}

fn authorize(
    now: i64,
    token: &CapabilityTokenV2,
    invocation: &PreparedInvocation,
    policy_decision: &PolicyAuthorizationV2,
    approval: Option<&SignedApprovalV1>,
) -> AuthorizedCapabilityV2 {
    validator_at(now)
        .authorize_and_consume_approved(token, context(invocation, policy_decision), approval)
        .unwrap()
}

fn minted_ids(
    token: &CapabilityTokenV2,
    invocation: &PreparedInvocation,
    policy_decision: &PolicyAuthorizationV2,
    approval: &SignedApprovalV1,
) -> (Uuid, Uuid, i64) {
    let authorized = authorize(NOW + 1, token, invocation, policy_decision, Some(approval));
    let approval_id = authorized.approval_id().expect("approved token");
    (
        authorized.token_id(),
        approval_id,
        authorized.expires_at_unix(),
    )
}

#[test]
fn durable_approval_survives_token_expiry_purge_until_approval_expiry() {
    let dir = tempfile::tempdir().unwrap();
    let invocation = prepared("durable approval retention");
    let idempotency = Uuid::from_u128(23);
    let policy_decision = decision(&invocation, AutomationLevel::L3BoundedAuto, idempotency);
    let approval = approve_at(NOW, &invocation, &policy_decision, 120);
    let first_token = issuer_at(NOW)
        .issue_approved(
            request_with_ttl(&invocation, &policy_decision, idempotency, 30),
            &approval,
        )
        .unwrap();

    let store = AuthorityStore::open(dir.path()).unwrap();
    let first = authorize(
        NOW + 1,
        &first_token,
        &invocation,
        &policy_decision,
        Some(&approval),
    );
    store
        .claim_verified(&first, Some(NOW + 120), NOW + 1)
        .unwrap();

    store.purge_expired(NOW + 31).unwrap();
    drop(store);

    let second_token = issuer_at(NOW + 31)
        .issue_approved(
            request_with_ttl(&invocation, &policy_decision, idempotency, 30),
            &approval,
        )
        .unwrap();
    let reopened = AuthorityStore::open(dir.path()).unwrap();
    let second = authorize(
        NOW + 31,
        &second_token,
        &invocation,
        &policy_decision,
        Some(&approval),
    );
    match reopened.claim_verified(&second, Some(NOW + 120), NOW + 31) {
        Err(AuthorityError::ApprovalAlreadyConsumed) => {}
        Err(error) => panic!("unexpected approval replay result: {error:?}"),
        Ok(()) => panic!("approval replay incorrectly reopened after token-expiry purge"),
    }
}

#[test]
fn expired_approval_purges_at_approval_expiry() {
    let dir = tempfile::tempdir().unwrap();
    let invocation = prepared("durable approval expiry");
    let idempotency = Uuid::from_u128(24);
    let policy_decision = decision(&invocation, AutomationLevel::L3BoundedAuto, idempotency);
    let approval = approve_at(NOW, &invocation, &policy_decision, 120);
    let token = issuer_at(NOW)
        .issue_approved(
            request_with_ttl(&invocation, &policy_decision, idempotency, 30),
            &approval,
        )
        .unwrap();
    let store = AuthorityStore::open(dir.path()).unwrap();
    let authorized = authorize(
        NOW + 1,
        &token,
        &invocation,
        &policy_decision,
        Some(&approval),
    );
    store
        .claim_verified(&authorized, Some(NOW + 120), NOW + 1)
        .unwrap();

    assert_eq!(store.purge_expired(NOW + 31).unwrap(), 2);
    assert_eq!(store.purge_expired(NOW + 119).unwrap(), 0);
    assert_eq!(store.purge_expired(NOW + 120).unwrap(), 3);
}

#[test]
fn durable_store_denies_replay_across_validator_restarts() {
    let dir = tempfile::tempdir().unwrap();
    let invocation = prepared("durable input");
    let idempotency = Uuid::from_u128(21);
    let policy_decision = decision(&invocation, AutomationLevel::L3BoundedAuto, idempotency);
    let approval = approve_at(NOW + 5, &invocation, &policy_decision, 300);
    let token = issuer_at(NOW + 10)
        .issue_approved(
            request(&invocation, &policy_decision, idempotency),
            &approval,
        )
        .unwrap();

    let store = AuthorityStore::open(dir.path()).unwrap();
    let first = authorize(
        NOW + 20,
        &token,
        &invocation,
        &policy_decision,
        Some(&approval),
    );
    store
        .claim_verified(&first, Some(NOW + 5 + 300), NOW + 20)
        .unwrap();

    let second = authorize(
        NOW + 25,
        &token,
        &invocation,
        &policy_decision,
        Some(&approval),
    );
    assert_eq!(
        store.claim_verified(&second, Some(NOW + 5 + 300), NOW + 25),
        Err(AuthorityError::AlreadyConsumed)
    );

    let second_token = issuer_at(NOW + 10)
        .issue_approved(
            request(&invocation, &policy_decision, idempotency),
            &approval,
        )
        .unwrap();
    let third = authorize(
        NOW + 30,
        &second_token,
        &invocation,
        &policy_decision,
        Some(&approval),
    );
    let error = store
        .claim_verified(&third, Some(NOW + 5 + 300), NOW + 30)
        .unwrap_err();
    assert!(
        matches!(
            error,
            AuthorityError::IdempotencyReplay | AuthorityError::ApprovalAlreadyConsumed
        ),
        "durable idempotency or approval consumption must deny: {error:?}"
    );
}

#[cfg(unix)]
#[test]
fn broken_authority_store_fails_closed() {
    let dir = tempfile::tempdir().unwrap();
    let store = AuthorityStore::open(dir.path()).unwrap();
    std::fs::remove_dir_all(dir.path().join("tokens")).unwrap();
    std::fs::write(dir.path().join("tokens"), b"broken").unwrap();

    let invocation = prepared("plain input");
    let idempotency = Uuid::from_u128(22);
    let policy_decision = decision(&invocation, AutomationLevel::L1Draft, idempotency);
    let token = issuer_at(NOW + 5)
        .issue(request(&invocation, &policy_decision, idempotency))
        .unwrap();
    let authorized = authorize(NOW + 10, &token, &invocation, &policy_decision, None);
    assert!(matches!(
        store.claim_verified(&authorized, None, NOW + 10),
        Err(AuthorityError::Unavailable(_))
    ));
}

#[test]
fn a_revoked_token_is_refused_as_revoked_not_as_a_replay() {
    let dir = tempfile::tempdir().unwrap();
    let invocation = prepared("revoked token");
    let idempotency = Uuid::from_u128(40);
    let policy_decision = decision(&invocation, AutomationLevel::L3BoundedAuto, idempotency);
    let approval = approve_at(NOW, &invocation, &policy_decision, 120);
    let token = issuer_at(NOW)
        .issue_approved(
            request_with_ttl(&invocation, &policy_decision, idempotency, 30),
            &approval,
        )
        .unwrap();

    let (token_id, _, expires_at_unix) =
        minted_ids(&token, &invocation, &policy_decision, &approval);
    let store = AuthorityStore::open(dir.path()).unwrap();
    store
        .revoke_token(token_id, NOW + 1, expires_at_unix)
        .unwrap();

    let authorized = authorize(
        NOW + 1,
        &token,
        &invocation,
        &policy_decision,
        Some(&approval),
    );
    assert_eq!(
        store.claim_verified(&authorized, Some(NOW + 120), NOW + 1),
        Err(AuthorityError::Revoked)
    );
}

#[test]
fn a_revoked_approval_is_refused_as_revoked() {
    let dir = tempfile::tempdir().unwrap();
    let invocation = prepared("revoked approval");
    let idempotency = Uuid::from_u128(41);
    let policy_decision = decision(&invocation, AutomationLevel::L3BoundedAuto, idempotency);
    let approval = approve_at(NOW, &invocation, &policy_decision, 120);
    let token = issuer_at(NOW)
        .issue_approved(
            request_with_ttl(&invocation, &policy_decision, idempotency, 30),
            &approval,
        )
        .unwrap();
    let (_, approval_id, _) = minted_ids(&token, &invocation, &policy_decision, &approval);

    let store = AuthorityStore::open(dir.path()).unwrap();
    store
        .revoke_approval(approval_id, NOW + 1, NOW + 120)
        .unwrap();

    let authorized = authorize(
        NOW + 1,
        &token,
        &invocation,
        &policy_decision,
        Some(&approval),
    );
    assert_eq!(
        store.claim_verified(&authorized, Some(NOW + 120), NOW + 1),
        Err(AuthorityError::Revoked)
    );
}

#[test]
fn a_bundle_interrupted_after_the_token_claim_resumes_on_retry() {
    let dir = tempfile::tempdir().unwrap();
    let invocation = prepared("interrupted bundle");
    let idempotency = Uuid::from_u128(42);
    let policy_decision = decision(&invocation, AutomationLevel::L3BoundedAuto, idempotency);
    let approval = approve_at(NOW, &invocation, &policy_decision, 120);
    let token = issuer_at(NOW)
        .issue_approved(
            request_with_ttl(&invocation, &policy_decision, idempotency, 30),
            &approval,
        )
        .unwrap();

    let store = AuthorityStore::open(dir.path()).unwrap();
    let blocked = BlockedPath::block(dir.path().join("approvals")).unwrap();
    let first = authorize(
        NOW + 1,
        &token,
        &invocation,
        &policy_decision,
        Some(&approval),
    );
    assert!(matches!(
        store.claim_verified(&first, Some(NOW + 120), NOW + 1),
        Err(AuthorityError::Unavailable(_))
    ));
    drop(blocked);

    let retry = authorize(
        NOW + 1,
        &token,
        &invocation,
        &policy_decision,
        Some(&approval),
    );
    assert_eq!(
        AuthorityStore::open(dir.path())
            .unwrap()
            .claim_verified(&retry, Some(NOW + 120), NOW + 1),
        Ok(())
    );
}

use chrono::{DateTime, Utc};
use sovereign_artifact::{
    AdmissionLimits, ArtifactVerificationIntent, ArtifactVerifier, Digest, OperationSelector,
    PreparedInvocation, RawResourceGrant, TrustedClock as ArtifactClock, CORE_WASM_ENTRYPOINT,
    MANIFEST_PROTOCOL_VERSION,
};
use sovereign_contracts::{AutomationLevel, DataClass};
use sovereign_identity::{KeyValidity, PublisherRole, RoleTrustStore, TypedSigner};
use sovereign_policy::{AuthenticatedPolicyContextV2, PolicyEngine, PolicyV2Error};
use uuid::Uuid;

const NOW: i64 = 1_800_000_000;
const AUDIENCE: &str = "sovereign-runtime";
const VENTURE: &str = "venture-alpha";
const SUBJECT: &str = "founder-session-subject";
const PUBLISHER: &str = "publisher.local";
const COMPONENT: &[u8] = b"\0asm\x01\0\0\0policy-v2-fixture";

fn session_id() -> Uuid {
    Uuid::from_u128(0x1111_1111_1111_1111_1111_1111_1111_1111)
}

fn idempotency_key() -> Uuid {
    Uuid::from_u128(0x2222_2222_2222_2222_2222_2222_2222_2222)
}

struct FixedClock(i64);

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

/// A `PreparedInvocation` built from a manifest with no resource bindings at
/// all, so `primary_resource()` is always `None` regardless of the input.
fn prepared_without_primary_resource() -> PreparedInvocation {
    let publisher = TypedSigner::<PublisherRole>::from_secret_bytes(PUBLISHER, [0x50; 32]).unwrap();
    let publisher_key_id = Digest::from_bytes(*publisher.key_id());
    let manifest = serde_json::json!({
        "protocol_version": MANIFEST_PROTOCOL_VERSION,
        "publisher_issuer": PUBLISHER,
        "publisher_key_id": publisher_key_id,
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
            "input_limits": {
                "max_bytes": 4096,
                "max_depth": 8
            },
            "input_schema": {
                "type": "object",
                "properties": {},
                "required": [],
                "max_properties": 1
            },
            "resource_bindings": []
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
    let input = serde_json::to_vec(&serde_json::json!({})).unwrap();
    PreparedInvocation::prepare(&artifact, &selector, &input, Vec::<RawResourceGrant>::new())
        .unwrap()
}

struct ContextCase {
    name: &'static str,
    audience: String,
    venture_id: String,
    subject_id: String,
    session_id: Uuid,
    idempotency_key: Uuid,
    expected: Result<(), PolicyV2Error>,
}

#[test]
fn authenticated_policy_context_v2_rejection_paths() {
    let too_long = "a".repeat(513);
    let cases = vec![
        ContextCase {
            name: "valid context constructs",
            audience: AUDIENCE.to_string(),
            venture_id: VENTURE.to_string(),
            subject_id: SUBJECT.to_string(),
            session_id: session_id(),
            idempotency_key: idempotency_key(),
            expected: Ok(()),
        },
        ContextCase {
            name: "empty audience",
            audience: "".to_string(),
            venture_id: VENTURE.to_string(),
            subject_id: SUBJECT.to_string(),
            session_id: session_id(),
            idempotency_key: idempotency_key(),
            expected: Err(PolicyV2Error::InvalidContext("audience")),
        },
        ContextCase {
            name: "audience with untrimmed whitespace",
            audience: " sovereign-runtime".to_string(),
            venture_id: VENTURE.to_string(),
            subject_id: SUBJECT.to_string(),
            session_id: session_id(),
            idempotency_key: idempotency_key(),
            expected: Err(PolicyV2Error::InvalidContext("audience")),
        },
        ContextCase {
            name: "audience exceeds 512 chars",
            audience: too_long.clone(),
            venture_id: VENTURE.to_string(),
            subject_id: SUBJECT.to_string(),
            session_id: session_id(),
            idempotency_key: idempotency_key(),
            expected: Err(PolicyV2Error::InvalidContext("audience")),
        },
        ContextCase {
            name: "audience contains a control character",
            audience: "sovereign\truntime".to_string(),
            venture_id: VENTURE.to_string(),
            subject_id: SUBJECT.to_string(),
            session_id: session_id(),
            idempotency_key: idempotency_key(),
            expected: Err(PolicyV2Error::InvalidContext("audience")),
        },
        ContextCase {
            name: "empty venture_id",
            audience: AUDIENCE.to_string(),
            venture_id: "".to_string(),
            subject_id: SUBJECT.to_string(),
            session_id: session_id(),
            idempotency_key: idempotency_key(),
            expected: Err(PolicyV2Error::InvalidContext("venture_id")),
        },
        ContextCase {
            name: "venture_id contains a control character",
            audience: AUDIENCE.to_string(),
            venture_id: "venture\u{0000}alpha".to_string(),
            subject_id: SUBJECT.to_string(),
            session_id: session_id(),
            idempotency_key: idempotency_key(),
            expected: Err(PolicyV2Error::InvalidContext("venture_id")),
        },
        ContextCase {
            name: "empty subject_id",
            audience: AUDIENCE.to_string(),
            venture_id: VENTURE.to_string(),
            subject_id: "".to_string(),
            session_id: session_id(),
            idempotency_key: idempotency_key(),
            expected: Err(PolicyV2Error::InvalidContext("subject_id")),
        },
        ContextCase {
            name: "subject_id exceeds 512 chars",
            audience: AUDIENCE.to_string(),
            venture_id: VENTURE.to_string(),
            subject_id: too_long.clone(),
            session_id: session_id(),
            idempotency_key: idempotency_key(),
            expected: Err(PolicyV2Error::InvalidContext("subject_id")),
        },
        ContextCase {
            name: "nil session_id",
            audience: AUDIENCE.to_string(),
            venture_id: VENTURE.to_string(),
            subject_id: SUBJECT.to_string(),
            session_id: Uuid::nil(),
            idempotency_key: idempotency_key(),
            expected: Err(PolicyV2Error::InvalidContext("session_id")),
        },
        ContextCase {
            name: "nil idempotency_key",
            audience: AUDIENCE.to_string(),
            venture_id: VENTURE.to_string(),
            subject_id: SUBJECT.to_string(),
            session_id: session_id(),
            idempotency_key: Uuid::nil(),
            expected: Err(PolicyV2Error::InvalidContext("idempotency_key")),
        },
    ];

    for case in cases {
        let result = AuthenticatedPolicyContextV2::new(
            case.audience,
            case.venture_id,
            case.subject_id,
            case.session_id,
            DataClass::Green,
            AutomationLevel::L1Draft,
            case.idempotency_key,
        );
        match case.expected {
            Ok(()) => assert!(
                result.is_ok(),
                "case `{}` expected Ok, got {result:?}",
                case.name
            ),
            Err(expected_err) => assert_eq!(
                result.err(),
                Some(expected_err),
                "case `{}` produced the wrong error",
                case.name
            ),
        }
    }
}

#[test]
fn evaluate_prepared_rejects_invocation_with_no_primary_resource() {
    let prepared = prepared_without_primary_resource();
    let context = AuthenticatedPolicyContextV2::new(
        AUDIENCE,
        VENTURE,
        SUBJECT,
        session_id(),
        DataClass::Green,
        AutomationLevel::L1Draft,
        idempotency_key(),
    )
    .unwrap();

    let result = PolicyEngine::new().evaluate_prepared(&prepared, context);

    assert_eq!(result.err(), Some(PolicyV2Error::MissingPrimaryResource));
}

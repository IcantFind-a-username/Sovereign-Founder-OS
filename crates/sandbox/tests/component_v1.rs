//! Adversarial and end-to-end tests for the verified component backend
//! (`sovereign:tool/pure-tool` world): full authorization chain, exact input
//! delivery and output lifting, default-deny imports, resource ceilings,
//! cross-backend substitution, and worker/cache boundaries.

use chrono::{DateTime, Duration, Utc};
use sovereign_artifact::{
    AdmissionLimits, AdmittedArtifact, ArtifactStore, ArtifactVerificationIntent, ArtifactVerifier,
    Digest, OperationSelector, PreparedInvocation, RawResourceGrant, TrustedClock as ArtifactClock,
    COMPONENT_ENTRYPOINT, CORE_WASM_ENTRYPOINT, MANIFEST_PROTOCOL_VERSION,
    SOVEREIGN_TOOL_WIT_WORLD,
};
use sovereign_capability::v2::{
    CapabilityIssuerV2, CapabilityTokenV2, CapabilityV2Error, CapabilityV2IssueOptions,
    CapabilityV2IssueRequest, CapabilityValidatorV2, TrustedClock as CapabilityClock,
};
use sovereign_contracts::{AutomationLevel, DataClass};
use sovereign_identity::{
    AdmissionRole, AuthorityRole, KeyValidity, PublisherRole, RoleTrustStore, TypedSigner,
};
use sovereign_policy::{AuthenticatedPolicyContextV2, PolicyAuthorizationV2, PolicyEngine};
use sovereign_sandbox::{
    ExecutionRuntime, SandboxError, VerifiedExecutionRequest, VerifiedSandboxExecutor,
};
use uuid::Uuid;

const NOW: i64 = 1_800_000_000;
const PUBLISHER_ISSUER: &str = "publisher.local";
const AUTHORITY_ISSUER: &str = "authority.local";
const AUDIENCE: &str = "sovereign-runtime";
const VENTURE: &str = "venture-alpha";
const SUBJECT: &str = "founder-session-subject";
const RESOURCE: &str = "draft:alpha";
const PUBLISHER_SECRET: [u8; 32] = [0x50; 32];
const AUTHORITY_SECRET: [u8; 32] = [0x41; 32];
const ADMISSION_ISSUER: &str = "device.local";
const ADMISSION_SECRET: [u8; 32] = [0x44; 32];

#[derive(Debug, Clone, Copy)]
struct FixedClock(i64);

impl ArtifactClock for FixedClock {
    fn now_unix(&self) -> i64 {
        self.0
    }
}

impl CapabilityClock for FixedClock {
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

/// A well-formed `pure-tool` guest whose `run` body is provided by the
/// caller. The body receives the lowered canonical input as `(param $ptr
/// $len)` and must return an i32 pointer to the lifted
/// `result<list<u8>, string>` representation.
fn component_with_run_body(body: &str) -> Vec<u8> {
    wat::parse_str(format!(
        r#"(component
  (core module $m
    (memory (export "mem") 6)
    (data (i32.const 8) "boom")
    (global $next (mut i32) (i32.const 65536))
    (func (export "cabi_realloc") (param $old i32) (param $old_size i32) (param $align i32) (param $new_size i32) (result i32)
      (local $ptr i32)
      (local.set $ptr
        (i32.and
          (i32.add (global.get $next) (i32.sub (local.get $align) (i32.const 1)))
          (i32.xor (i32.sub (local.get $align) (i32.const 1)) (i32.const -1))))
      (global.set $next (i32.add (local.get $ptr) (local.get $new_size)))
      (local.get $ptr))
    (func (export "run") (param $ptr i32) (param $len i32) (result i32)
      {body})
  )
  (core instance $i (instantiate $m))
  (func (export "run") (param "input" (list u8)) (result (result (list u8) (error string)))
    (canon lift (core func $i "run") (memory $i "mem") (realloc (func $i "cabi_realloc"))))
)"#
    ))
    .unwrap()
}

/// Echoes the exact delivered input back as the success payload.
fn component_echo() -> Vec<u8> {
    component_with_run_body(
        r#"(i32.store (i32.const 1024) (i32.const 0))
           (i32.store (i32.const 1028) (local.get $ptr))
           (i32.store (i32.const 1032) (local.get $len))
           (i32.const 1024)"#,
    )
}

/// Reports the guest-level error string "boom".
fn component_guest_error() -> Vec<u8> {
    component_with_run_body(
        r#"(i32.store (i32.const 1024) (i32.const 1))
           (i32.store (i32.const 1028) (i32.const 8))
           (i32.store (i32.const 1032) (i32.const 4))
           (i32.const 1024)"#,
    )
}

/// Spins forever: fuel must terminate it.
fn component_looping() -> Vec<u8> {
    component_with_run_body(
        r#"(loop $spin (br $spin))
           (i32.const 1024)"#,
    )
}

/// Grows linear memory until the host ceiling refuses it.
fn component_memory_bomb() -> Vec<u8> {
    component_with_run_body(
        r#"(loop $grow
             (drop (memory.grow (i32.const 4)))
             (br $grow))
           (i32.const 1024)"#,
    )
}

/// Returns a success payload larger than the host output ceiling
/// (256 KiB + 1 bytes of guest memory starting at offset 0).
fn component_oversized_output() -> Vec<u8> {
    component_with_run_body(
        r#"(i32.store (i32.const 262148) (i32.const 0))
           (i32.store (i32.const 262152) (i32.const 0))
           (i32.store (i32.const 262156) (i32.const 262145))
           (i32.const 262148)"#,
    )
}

/// Declares an import: refused before instantiation, whatever it exports.
fn component_with_import() -> Vec<u8> {
    wat::parse_str(r#"(component (import "hostile" (func)))"#).unwrap()
}

/// Exports `run` with a non-world type (`func() -> s32`).
fn component_wrong_run_type() -> Vec<u8> {
    wat::parse_str(
        r#"(component
  (core module $m (func (export "run") (result i32) (i32.const 0)))
  (core instance $i (instantiate $m))
  (func (export "run") (result s32) (canon lift (core func $i "run")))
)"#,
    )
    .unwrap()
}

/// Exports nothing named `run`.
fn component_missing_run() -> Vec<u8> {
    wat::parse_str(
        r#"(component
  (core module $m (func (export "walk") (result i32) (i32.const 0)))
  (core instance $i (instantiate $m))
  (func (export "walk") (result s32) (canon lift (core func $i "walk")))
)"#,
    )
    .unwrap()
}

/// A plain core module (valid for the core backend, not a component).
fn core_module() -> Vec<u8> {
    wat::parse_str(r#"(module (func (export "sovereign_run") (result i32) i32.const 7))"#).unwrap()
}

fn manifest_json(
    component: &[u8],
    publisher: &TypedSigner<PublisherRole>,
    backend: &str,
) -> serde_json::Value {
    let mut value = serde_json::json!({
        "protocol_version": MANIFEST_PROTOCOL_VERSION,
        "publisher_issuer": PUBLISHER_ISSUER,
        "publisher_key_id": Digest::from_bytes(*publisher.key_id()),
        "component_digest": Digest::of_bytes(component),
        "backend": backend,
        "risk_class": "pure_compute",
        "abi": "sovereign_component_v1",
        "entrypoint": COMPONENT_ENTRYPOINT,
        "wit_world": SOVEREIGN_TOOL_WIT_WORLD,
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
    if backend == "core_wasm" {
        let object = value.as_object_mut().unwrap();
        object.remove("wit_world");
        object.insert("abi".into(), serde_json::json!("sovereign_core_wasm_v1"));
        object.insert("entrypoint".into(), serde_json::json!(CORE_WASM_ENTRYPOINT));
    }
    value
}

fn admit_and_prepare_backend(
    component: &[u8],
    content: &str,
    backend: &str,
) -> (PreparedInvocation, AdmittedArtifact) {
    let publisher =
        TypedSigner::<PublisherRole>::from_secret_bytes(PUBLISHER_ISSUER, PUBLISHER_SECRET)
            .unwrap();
    let manifest = manifest_json(component, &publisher, backend);
    let canonical_manifest = serde_json_canonicalizer::to_vec(&manifest).unwrap();
    let signed_manifest = publisher.sign_cose(&canonical_manifest).unwrap();
    let mut publishers = RoleTrustStore::<PublisherRole>::new();
    publishers
        .trust_signer(&publisher, KeyValidity::new(NOW - 60, NOW + 3_600).unwrap())
        .unwrap();
    let intent = ArtifactVerificationIntent::new(
        PUBLISHER_ISSUER,
        Digest::of_bytes(&signed_manifest),
        Digest::of_bytes(component),
    )
    .unwrap();
    let artifact =
        ArtifactVerifier::with_clock(&publishers, AdmissionLimits::default(), FixedClock(NOW))
            .verify(&intent, &signed_manifest, component)
            .unwrap();
    let input = serde_json::to_vec(&serde_json::json!({
        "content": content,
        "resource": RESOURCE
    }))
    .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let admission_signer =
        TypedSigner::<AdmissionRole>::from_secret_bytes(ADMISSION_ISSUER, ADMISSION_SECRET)
            .unwrap();
    let store = ArtifactStore::open(dir.path()).unwrap();
    let admitted = store
        .admit(&artifact, &admission_signer, &FixedClock(NOW))
        .unwrap();
    let invocation = PreparedInvocation::prepare(
        &artifact,
        &selector(),
        &input,
        vec![RawResourceGrant::new("primary", RESOURCE)],
    )
    .unwrap();
    (invocation, admitted)
}

fn admit_and_prepare(component: &[u8], content: &str) -> (PreparedInvocation, AdmittedArtifact) {
    admit_and_prepare_backend(component, content, "component_wasm")
}

fn decision(invocation: &PreparedInvocation) -> PolicyAuthorizationV2 {
    PolicyEngine::with_clock(FixedClock(NOW))
        .evaluate_prepared(
            invocation,
            AuthenticatedPolicyContextV2::new(
                AUDIENCE,
                VENTURE,
                SUBJECT,
                Uuid::from_u128(1),
                DataClass::Green,
                AutomationLevel::L1Draft,
                Uuid::from_u128(2),
            )
            .unwrap(),
        )
        .unwrap()
}

fn authority() -> (
    CapabilityIssuerV2<FixedClock>,
    CapabilityValidatorV2<FixedClock>,
) {
    let trusted =
        TypedSigner::<AuthorityRole>::from_secret_bytes(AUTHORITY_ISSUER, AUTHORITY_SECRET)
            .unwrap();
    let mut trust_store = RoleTrustStore::<AuthorityRole>::new();
    trust_store
        .trust_signer(&trusted, KeyValidity::new(NOW - 60, NOW + 3_600).unwrap())
        .unwrap();
    let issuer = CapabilityIssuerV2::new(
        TypedSigner::<AuthorityRole>::from_secret_bytes(AUTHORITY_ISSUER, AUTHORITY_SECRET)
            .unwrap(),
        AUDIENCE,
        FixedClock(NOW),
    )
    .unwrap();
    let validator =
        CapabilityValidatorV2::new(trust_store, AUTHORITY_ISSUER, AUDIENCE, FixedClock(NOW))
            .unwrap();
    (issuer, validator)
}

fn issue(
    issuer: &CapabilityIssuerV2<FixedClock>,
    invocation: &PreparedInvocation,
    policy_decision: &PolicyAuthorizationV2,
) -> CapabilityTokenV2 {
    issuer
        .issue(CapabilityV2IssueRequest {
            venture_id: VENTURE,
            subject_id: SUBJECT,
            session_id: policy_decision.session_id(),
            policy_decision,
            prepared_invocation: invocation,
            options: CapabilityV2IssueOptions {
                ttl: Duration::seconds(60),
                idempotency_key: policy_decision.idempotency_key(),
            },
        })
        .unwrap()
}

fn request<'a>(
    token: &'a CapabilityTokenV2,
    invocation: &'a PreparedInvocation,
    admitted: &'a AdmittedArtifact,
    policy_decision: &'a PolicyAuthorizationV2,
) -> VerifiedExecutionRequest<'a> {
    VerifiedExecutionRequest {
        token,
        invocation,
        admitted,
        venture_id: VENTURE,
        subject_id: SUBJECT,
        session_id: policy_decision.session_id(),
        policy_decision,
    }
}

/// Full chain, one call: admit, prepare, decide, issue, execute.
fn run_component(component: &[u8]) -> Result<sovereign_sandbox::WasmExecutionResult, SandboxError> {
    let (invocation, admitted) = admit_and_prepare(component, "component input");
    let policy_decision = decision(&invocation);
    let (issuer, validator) = authority();
    let token = issue(&issuer, &invocation, &policy_decision);
    let mut executor = VerifiedSandboxExecutor::new(vec![selector()], validator).unwrap();
    executor.execute(request(&token, &invocation, &admitted, &policy_decision))
}

#[test]
fn component_receives_the_exact_canonical_input_and_returns_bounded_output() {
    let (invocation, admitted) = admit_and_prepare(&component_echo(), "hello component");
    let policy_decision = decision(&invocation);
    let (issuer, validator) = authority();
    let token = issue(&issuer, &invocation, &policy_decision);
    let mut executor = VerifiedSandboxExecutor::new(vec![selector()], validator).unwrap();

    let result = executor
        .execute(request(&token, &invocation, &admitted, &policy_decision))
        .unwrap();
    assert_eq!(
        result.runtime,
        ExecutionRuntime::WasmtimeVerifiedComponentV1
    );
    assert!(result.runtime.is_isolated());
    assert!(!result.runtime.is_production_ready());
    // The echoed payload proves the exact authenticated canonical input was
    // delivered through the canonical ABI — not a path, not a caller buffer.
    assert_eq!(
        result.output.as_deref(),
        Some(invocation.canonical_input()),
        "component output must be the exact canonical input it echoed"
    );
}

#[test]
fn component_declaring_any_import_is_refused_before_instantiation() {
    assert!(matches!(
        run_component(&component_with_import()),
        Err(SandboxError::ForbiddenImport { .. })
    ));
}

#[test]
fn component_infinite_loop_is_terminated_by_fuel() {
    assert!(matches!(
        run_component(&component_looping()),
        Err(SandboxError::FuelExhausted)
    ));
}

#[test]
fn component_memory_bomb_hits_the_host_ceiling() {
    assert!(matches!(
        run_component(&component_memory_bomb()),
        Err(SandboxError::ResourceLimitExceeded(_))
    ));
}

#[test]
fn component_output_above_the_host_ceiling_is_rejected() {
    assert!(matches!(
        run_component(&component_oversized_output()),
        Err(SandboxError::OutputLimitExceeded { actual, .. }) if actual == 262_145
    ));
}

#[test]
fn component_missing_or_mistyped_run_export_fails_closed() {
    assert!(matches!(
        run_component(&component_missing_run()),
        Err(SandboxError::MissingEntrypoint(name)) if name == COMPONENT_ENTRYPOINT
    ));
    assert!(matches!(
        run_component(&component_wrong_run_type()),
        Err(SandboxError::InvalidEntrypoint { .. })
    ));
}

#[test]
fn guest_reported_error_is_bounded_and_still_consumes_the_capability() {
    let (invocation, admitted) = admit_and_prepare(&component_guest_error(), "component input");
    let policy_decision = decision(&invocation);
    let (issuer, validator) = authority();
    let token = issue(&issuer, &invocation, &policy_decision);
    let mut executor = VerifiedSandboxExecutor::new(vec![selector()], validator).unwrap();

    assert!(matches!(
        executor.execute(request(&token, &invocation, &admitted, &policy_decision)),
        Err(SandboxError::GuestReportedError(message)) if message == "boom"
    ));
    // The failed run spent the one-use capability: a replay is refused.
    assert!(matches!(
        executor.execute(request(&token, &invocation, &admitted, &policy_decision)),
        Err(SandboxError::CapabilityV2(CapabilityV2Error::Replay))
    ));
}

#[test]
fn cross_backend_substitution_fails_closed_in_both_directions() {
    // Core-module bytes admitted under a component manifest cannot execute.
    assert!(matches!(
        run_component(&core_module()),
        Err(SandboxError::InvalidModule(_))
    ));

    // Component bytes admitted under a core-Wasm manifest cannot execute.
    let (invocation, admitted) =
        admit_and_prepare_backend(&component_echo(), "component input", "core_wasm");
    let policy_decision = decision(&invocation);
    let (issuer, validator) = authority();
    let token = issue(&issuer, &invocation, &policy_decision);
    let mut executor = VerifiedSandboxExecutor::new(vec![selector()], validator).unwrap();
    assert!(matches!(
        executor.execute(request(&token, &invocation, &admitted, &policy_decision)),
        Err(SandboxError::InvalidModule(_))
    ));
}

#[test]
fn component_token_cannot_authorize_a_different_component() {
    let (invocation, _admitted) = admit_and_prepare(&component_echo(), "component input");
    let (other_invocation, other_admitted) =
        admit_and_prepare(&component_guest_error(), "component input");
    let policy_decision = decision(&invocation);
    let (issuer, validator) = authority();
    let token = issue(&issuer, &invocation, &policy_decision);
    let mut executor = VerifiedSandboxExecutor::new(vec![selector()], validator).unwrap();

    assert!(matches!(
        executor.execute(request(
            &token,
            &other_invocation,
            &other_admitted,
            &policy_decision
        )),
        Err(SandboxError::CapabilityV2(
            CapabilityV2Error::InvocationMismatch("component_digest")
        ))
    ));
}

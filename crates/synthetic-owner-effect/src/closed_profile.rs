//! Build a publisher-verified closed-profile invocation for one sealed intent.
//!
//! Canonical input binds the random intent id, fixture generation, fixed
//! operation, and coordinator reference — never recipient or RFC 5322 bytes.

use std::sync::OnceLock;

use sovereign_artifact::{
    AdmissionLimits, ArtifactVerificationIntent, ArtifactVerifier, Digest, OperationSelector,
    PreparedInvocation, RawResourceGrant, CLOSED_FIXTURE_OPERATION_ID, CLOSED_FIXTURE_TOOL_ID,
    CLOSED_FIXTURE_TOOL_VERSION, CORE_WASM_ENTRYPOINT, MANIFEST_PROTOCOL_VERSION,
};
use sovereign_identity::{KeyValidity, PublisherRole, RoleTrustStore, TypedSigner};

use crate::sealed::{EffectIntentId, COORDINATOR_REF};

const PUBLISHER: &str = "fixture.unqualified.publisher";

/// Import-free Core Wasm v2 guest: sums delivered canonical input bytes.
///
/// No WASI, filesystem, clock, or host-call imports. The coordinator compares
/// the exit code against the expected closed checksum and owns publication.
pub const FIXED_GUEST_WAT: &str = r#"(module
    (memory (export "memory") 1)
    (func (export "sovereign_run") (param $ptr i32) (param $len i32) (result i32)
        (local $i i32) (local $sum i32)
        (block $done
            (loop $loop
                (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
                (local.set $sum
                    (i32.add (local.get $sum)
                        (i32.load8_u (i32.add (local.get $ptr) (local.get $i)))))
                (local.set $i (i32.add (local.get $i) (i32.const 1)))
                (br $loop)))
        (i32.and (local.get $sum) (i32.const 255))))"#;

pub fn fixed_core_wasm() -> &'static [u8] {
    static WASM: OnceLock<Vec<u8>> = OnceLock::new();
    WASM.get_or_init(|| wat::parse_str(FIXED_GUEST_WAT).expect("fixed import-free Core Wasm guest"))
}

pub fn canonical_guest_input(intent_id: EffectIntentId, fixture_generation: u64) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "effect_intent_id": intent_id.file_stem(),
        "fixture_generation": fixture_generation.to_string(),
        "operation": CLOSED_FIXTURE_OPERATION_ID,
        "coordinator": COORDINATOR_REF
    }))
    .expect("canonical guest input is JSON")
}

pub fn expected_closed_exit(input: &[u8]) -> i32 {
    input
        .iter()
        .fold(0i32, |sum, byte| (sum + i32::from(*byte)) & 255)
}

pub struct ClosedProfile {
    publisher: TypedSigner<PublisherRole>,
    publishers: RoleTrustStore<PublisherRole>,
}

impl ClosedProfile {
    pub fn generate() -> Result<Self, ClosedProfileError> {
        let publisher = TypedSigner::<PublisherRole>::generate(PUBLISHER)
            .map_err(|_| ClosedProfileError::Unavailable)?;
        let mut publishers = RoleTrustStore::<PublisherRole>::new();
        publishers
            .trust_signer(&publisher, KeyValidity::new(1, 4_000_000_000).unwrap())
            .map_err(|_| ClosedProfileError::Unavailable)?;
        Ok(Self {
            publisher,
            publishers,
        })
    }

    pub fn prepare(
        &self,
        intent_id: EffectIntentId,
        fixture_generation: u64,
        now_unix: i64,
    ) -> Result<PreparedInvocation, ClosedProfileError> {
        let resource = intent_id.file_stem();
        let manifest = serde_json::json!({
            "protocol_version": MANIFEST_PROTOCOL_VERSION,
            "publisher_issuer": PUBLISHER,
            "publisher_key_id": Digest::from_bytes(*self.publisher.key_id()),
            "component_digest": Digest::of_bytes(fixed_core_wasm()),
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
        let canonical = serde_json_canonicalizer::to_vec(&manifest)
            .map_err(|_| ClosedProfileError::Unavailable)?;
        let signed = self
            .publisher
            .sign_cose(&canonical)
            .map_err(|_| ClosedProfileError::Unavailable)?;
        let intent = ArtifactVerificationIntent::new(
            PUBLISHER,
            Digest::of_bytes(&signed),
            Digest::of_bytes(fixed_core_wasm()),
        )
        .map_err(|_| ClosedProfileError::Unavailable)?;
        let artifact = ArtifactVerifier::with_clock(
            &self.publishers,
            AdmissionLimits::default(),
            UnixClock(now_unix),
        )
        .verify_closed_fixture_profile(&intent, &signed, fixed_core_wasm())
        .map_err(|_| ClosedProfileError::Unavailable)?;
        let input = canonical_guest_input(intent_id, fixture_generation);
        let selector = OperationSelector::new(
            CLOSED_FIXTURE_TOOL_ID,
            CLOSED_FIXTURE_TOOL_VERSION,
            CLOSED_FIXTURE_OPERATION_ID,
        )
        .map_err(|_| ClosedProfileError::Unavailable)?;
        PreparedInvocation::prepare(
            &artifact,
            &selector,
            &input,
            vec![RawResourceGrant::new("intent", resource)],
        )
        .map_err(|_| ClosedProfileError::Unavailable)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClosedProfileError {
    Unavailable,
}

struct UnixClock(i64);

impl sovereign_artifact::TrustedClock for UnixClock {
    fn now_unix(&self) -> i64 {
        self.0
    }
}

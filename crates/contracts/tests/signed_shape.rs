//! Wire-shape locks for the signed contract types.
//!
//! Signatures and hashes are taken over `serde_json::to_vec` of these bodies
//! (`sovereign_capability::token_signing_bytes`,
//! `sovereign_audit_ledger::hash_event_body`), and plain `serde_json` emits
//! struct fields in declaration order. The serialized bytes — field names *and*
//! their order — are therefore part of the verified payload, not an
//! implementation detail: every golden string below is a compatibility
//! boundary, and changing one invalidates tokens and audit chains signed under
//! the old shape.
//!
//! A field may still be renamed in Rust; keeping `#[serde(rename = "...")]` on
//! it preserves the wire shape and these tests stay green.

use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;
use sovereign_contracts::{
    ActionRequest, AuditEvent, AuditEventBody, AutomationLevel, CapabilityToken,
    CapabilityTokenBody, DataClass, PolicyDecision,
};
use uuid::Uuid;

/// 2026-01-01T00:00:00Z
const ISSUED_AT: i64 = 1_767_225_600;
/// 2026-01-01T01:00:00Z
const EXPIRES_AT: i64 = 1_767_229_200;

const TOKEN_ID: &str = "11111111-2222-3333-4444-555555555555";
const DECISION_ID: &str = "66666666-7777-8888-9999-aaaaaaaaaaaa";
const EVENT_ID: &str = "bbbbbbbb-cccc-dddd-eeee-ffffffffffff";

// Distinct values per field: a transposed mapping in the hand-written `From`
// impls (both carry several same-typed `String` fields) shows up as a golden
// mismatch instead of passing silently.
const VENTURE_ID: &str = "venture-alpha";
const ACTOR_ID: &str = "agent-drafting";
const TOOL: &str = "outbox";
const OPERATION: &str = "compose";
const RESOURCE: &str = "draft:alpha";

fn uuid(text: &str) -> Uuid {
    Uuid::parse_str(text).expect("fixture uuid must parse")
}

fn at(unix: i64) -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp(unix, 0).expect("fixture timestamp must be in range")
}

/// Asserts that dropping any non-optional key makes deserialization fail, and
/// that the listed optional keys are absent-tolerant. Signed bodies must never
/// silently default a field: a verifier that accepts a truncated body would
/// check a signature over content the issuer never authorized.
fn assert_absent_field_handling<T: DeserializeOwned>(golden: &str, optional: &[&str], label: &str) {
    let value: serde_json::Value = serde_json::from_str(golden).expect("golden must parse");
    let object = value.as_object().expect("golden must be a JSON object");

    for key in object.keys() {
        let mut reduced = object.clone();
        reduced.remove(key);
        let outcome = serde_json::from_value::<T>(serde_json::Value::Object(reduced));

        if optional.contains(&key.as_str()) {
            assert!(
                outcome.is_ok(),
                "{label}: `{key}` is declared optional but dropping it failed to deserialize"
            );
        } else {
            assert!(
                outcome.is_err(),
                "{label}: dropping `{key}` still deserialized — a signed body must never \
                 silently default a field"
            );
        }
    }
}

/// Asserts a key is absent from the serialized body. Checked structurally
/// rather than by substring: `event_hash` is a substring of the legitimate
/// `previous_event_hash` key, so `contains` would report a false positive.
fn assert_key_absent(json: &str, key: &str) {
    let value: serde_json::Value = serde_json::from_str(json).expect("body must parse");
    let object = value.as_object().expect("body must be a JSON object");

    assert!(
        !object.contains_key(key),
        "`{key}` must not appear in the signed body: {json}"
    );
}

fn capability_token() -> CapabilityToken {
    CapabilityToken {
        token_id: uuid(TOKEN_ID),
        venture_id: VENTURE_ID.to_owned(),
        actor_id: ACTOR_ID.to_owned(),
        tool: TOOL.to_owned(),
        operation: OPERATION.to_owned(),
        resource: RESOURCE.to_owned(),
        max_uses: 1,
        issued_at: at(ISSUED_AT),
        expires_at: at(EXPIRES_AT),
        policy_decision_id: uuid(DECISION_ID),
        issuer_public_key_b64: "aXNzdWVyLWtleQ==".to_owned(),
        // Excluded from the signed body by construction — the golden string
        // below is what proves it.
        signature_b64: "c2lnbmF0dXJl".to_owned(),
    }
}

const CAPABILITY_TOKEN_BODY_GOLDEN: &str = concat!(
    r#"{"token_id":"11111111-2222-3333-4444-555555555555","#,
    r#""venture_id":"venture-alpha","#,
    r#""actor_id":"agent-drafting","#,
    r#""tool":"outbox","#,
    r#""operation":"compose","#,
    r#""resource":"draft:alpha","#,
    r#""max_uses":1,"#,
    r#""issued_at":"2026-01-01T00:00:00Z","#,
    r#""expires_at":"2026-01-01T01:00:00Z","#,
    r#""policy_decision_id":"66666666-7777-8888-9999-aaaaaaaaaaaa","#,
    r#""issuer_public_key_b64":"aXNzdWVyLWtleQ=="}"#,
);

#[test]
fn capability_token_body_serializes_to_the_signed_golden_bytes() {
    let body = CapabilityTokenBody::from(&capability_token());
    let json = serde_json::to_string(&body).expect("body must serialize");

    assert_eq!(
        json, CAPABILITY_TOKEN_BODY_GOLDEN,
        "the capability token signing payload changed shape; existing tokens no longer verify"
    );
}

#[test]
fn capability_token_body_excludes_the_signature() {
    let json = serde_json::to_string(&CapabilityTokenBody::from(&capability_token()))
        .expect("body must serialize");

    assert_key_absent(&json, "signature_b64");
}

#[test]
fn capability_token_body_round_trips_byte_identically() {
    let parsed: CapabilityTokenBody =
        serde_json::from_str(CAPABILITY_TOKEN_BODY_GOLDEN).expect("golden must deserialize");

    assert_eq!(parsed.token_id, uuid(TOKEN_ID));
    assert_eq!(parsed.venture_id, VENTURE_ID);
    assert_eq!(parsed.actor_id, ACTOR_ID);
    assert_eq!(parsed.tool, TOOL);
    assert_eq!(parsed.operation, OPERATION);
    assert_eq!(parsed.resource, RESOURCE);
    assert_eq!(parsed.max_uses, 1);
    assert_eq!(parsed.issued_at, at(ISSUED_AT));
    assert_eq!(parsed.expires_at, at(EXPIRES_AT));
    assert_eq!(parsed.policy_decision_id, uuid(DECISION_ID));
    assert_eq!(parsed.issuer_public_key_b64, "aXNzdWVyLWtleQ==");

    let reserialized = serde_json::to_string(&parsed).expect("body must serialize");
    assert_eq!(
        reserialized, CAPABILITY_TOKEN_BODY_GOLDEN,
        "a decode/encode cycle must reproduce the exact signed bytes"
    );
}

#[test]
fn capability_token_body_rejects_a_truncated_payload() {
    assert_absent_field_handling::<CapabilityTokenBody>(
        CAPABILITY_TOKEN_BODY_GOLDEN,
        &[],
        "CapabilityTokenBody",
    );
}

fn policy_decision() -> PolicyDecision {
    PolicyDecision {
        decision_id: uuid(DECISION_ID),
        allowed: true,
        reason: "outbox.compose permitted at L2".to_owned(),
        requires_approval: true,
        evaluated_at: at(ISSUED_AT),
        request: ActionRequest {
            actor_id: ACTOR_ID.to_owned(),
            venture_id: VENTURE_ID.to_owned(),
            tool: TOOL.to_owned(),
            operation: OPERATION.to_owned(),
            resource: RESOURCE.to_owned(),
            data_class: DataClass::Amber,
            automation_level: AutomationLevel::L2ApproveExecute,
        },
    }
}

const POLICY_DECISION_GOLDEN: &str = concat!(
    r#"{"decision_id":"66666666-7777-8888-9999-aaaaaaaaaaaa","#,
    r#""allowed":true,"#,
    r#""reason":"outbox.compose permitted at L2","#,
    r#""requires_approval":true,"#,
    r#""evaluated_at":"2026-01-01T00:00:00Z","#,
    r#""request":{"actor_id":"agent-drafting","#,
    r#""venture_id":"venture-alpha","#,
    r#""tool":"outbox","#,
    r#""operation":"compose","#,
    r#""resource":"draft:alpha","#,
    r#""data_class":"amber","#,
    r#""automation_level":"L2ApproveExecute"}}"#,
);

#[test]
fn policy_decision_serializes_to_the_golden_shape() {
    let json = serde_json::to_string(&policy_decision()).expect("decision must serialize");

    assert_eq!(
        json, POLICY_DECISION_GOLDEN,
        "the policy decision wire shape changed; decision digests no longer match"
    );
}

#[test]
fn policy_decision_round_trips() {
    let parsed: PolicyDecision =
        serde_json::from_str(POLICY_DECISION_GOLDEN).expect("golden must deserialize");

    assert_eq!(parsed, policy_decision());
}

#[test]
fn policy_decision_rejects_a_truncated_payload() {
    assert_absent_field_handling::<PolicyDecision>(POLICY_DECISION_GOLDEN, &[], "PolicyDecision");
}

#[test]
fn data_class_wire_tokens_are_snake_case() {
    for (variant, expected) in [
        (DataClass::Red, r#""red""#),
        (DataClass::Amber, r#""amber""#),
        (DataClass::Green, r#""green""#),
    ] {
        let json = serde_json::to_string(&variant).expect("class must serialize");
        assert_eq!(json, expected, "{variant:?} changed its wire token");

        let parsed: DataClass = serde_json::from_str(expected).expect("token must deserialize");
        assert_eq!(parsed, variant);
    }
}

#[test]
fn automation_level_serializes_by_name_not_by_discriminant() {
    // The variants carry explicit `= 0..3` discriminants, which makes a numeric
    // encoding look plausible; serde emits the variant name, and that name is
    // what a signed `ActionRequest` commits to.
    for (variant, expected) in [
        (AutomationLevel::L0Suggest, r#""L0Suggest""#),
        (AutomationLevel::L1Draft, r#""L1Draft""#),
        (AutomationLevel::L2ApproveExecute, r#""L2ApproveExecute""#),
        (AutomationLevel::L3BoundedAuto, r#""L3BoundedAuto""#),
    ] {
        let json = serde_json::to_string(&variant).expect("level must serialize");
        assert_eq!(json, expected, "{variant:?} changed its wire token");

        let parsed: AutomationLevel =
            serde_json::from_str(expected).expect("token must deserialize");
        assert_eq!(parsed, variant);
    }
}

fn audit_event(capability_id: Option<Uuid>, policy_decision_hash: Option<String>) -> AuditEvent {
    AuditEvent {
        event_id: uuid(EVENT_ID),
        venture_id: VENTURE_ID.to_owned(),
        actor_id: ACTOR_ID.to_owned(),
        action: "effect.written".to_owned(),
        resource: "outbox/alpha.eml".to_owned(),
        capability_id,
        timestamp: at(ISSUED_AT),
        payload_hash: "payload-hash".to_owned(),
        previous_event_hash: "previous-hash".to_owned(),
        policy_decision_hash,
        device_public_key_b64: "ZGV2aWNlLWtleQ==".to_owned(),
        // Both are derived from the body, so neither may appear inside it.
        event_hash: "event-hash".to_owned(),
        device_signature: Some("ZGV2aWNlLXNpZw==".to_owned()),
    }
}

const AUDIT_EVENT_BODY_GOLDEN: &str = concat!(
    r#"{"event_id":"bbbbbbbb-cccc-dddd-eeee-ffffffffffff","#,
    r#""venture_id":"venture-alpha","#,
    r#""actor_id":"agent-drafting","#,
    r#""action":"effect.written","#,
    r#""resource":"outbox/alpha.eml","#,
    r#""capability_id":"11111111-2222-3333-4444-555555555555","#,
    r#""timestamp":"2026-01-01T00:00:00Z","#,
    r#""payload_hash":"payload-hash","#,
    r#""previous_event_hash":"previous-hash","#,
    r#""policy_decision_hash":"decision-hash","#,
    r#""device_public_key_b64":"ZGV2aWNlLWtleQ=="}"#,
);

#[test]
fn audit_event_body_serializes_to_the_hashed_golden_bytes() {
    let event = audit_event(Some(uuid(TOKEN_ID)), Some("decision-hash".to_owned()));
    let json = serde_json::to_string(&AuditEventBody::from(&event)).expect("body must serialize");

    assert_eq!(
        json, AUDIT_EVENT_BODY_GOLDEN,
        "the audit event hash input changed shape; every existing chain fails to re-verify"
    );
}

#[test]
fn audit_event_body_excludes_the_hash_and_signature() {
    let event = audit_event(Some(uuid(TOKEN_ID)), Some("decision-hash".to_owned()));
    let json = serde_json::to_string(&AuditEventBody::from(&event)).expect("body must serialize");

    assert_key_absent(&json, "event_hash");
    assert_key_absent(&json, "device_signature");
}

#[test]
fn audit_event_body_writes_absent_options_as_null() {
    let event = audit_event(None, None);
    let json = serde_json::to_string(&AuditEventBody::from(&event)).expect("body must serialize");

    // No `skip_serializing_if`: the keys stay present with a null value. Making
    // them disappear instead would change the hash of every unattributed event.
    assert!(
        json.contains(r#""capability_id":null"#) && json.contains(r#""policy_decision_hash":null"#),
        "absent options must hash as explicit nulls, not vanish: {json}"
    );
}

#[test]
fn audit_event_body_round_trips_byte_identically() {
    let parsed: AuditEventBody =
        serde_json::from_str(AUDIT_EVENT_BODY_GOLDEN).expect("golden must deserialize");

    assert_eq!(parsed.event_id, uuid(EVENT_ID));
    assert_eq!(parsed.capability_id, Some(uuid(TOKEN_ID)));
    assert_eq!(parsed.timestamp, at(ISSUED_AT));
    assert_eq!(
        parsed.policy_decision_hash.as_deref(),
        Some("decision-hash")
    );

    let reserialized = serde_json::to_string(&parsed).expect("body must serialize");
    assert_eq!(
        reserialized, AUDIT_EVENT_BODY_GOLDEN,
        "a decode/encode cycle must reproduce the exact hashed bytes"
    );
}

#[test]
fn audit_event_body_rejects_a_truncated_payload() {
    assert_absent_field_handling::<AuditEventBody>(
        AUDIT_EVENT_BODY_GOLDEN,
        &["capability_id", "policy_decision_hash"],
        "AuditEventBody",
    );
}

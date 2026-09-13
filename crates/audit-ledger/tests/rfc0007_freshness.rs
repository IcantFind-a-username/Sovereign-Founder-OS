//! RFC 0007 conformance tests (`ledger.head` freshness anchor).

use chrono::{DateTime, Utc};
use sovereign_audit_ledger::{
    hash_event_body, ledger_head_path, verify_freshness, AppendInput, AuditLedger, LedgerError,
    LedgerHead,
};
use sovereign_contracts::{AuditEvent, AuditEventBody};
use sovereign_identity::DeviceIdentity;
use tempfile::tempdir;
use uuid::Uuid;

fn append_n(ledger: &mut AuditLedger, device: &DeviceIdentity, n: usize) {
    for i in 0..n {
        ledger
            .append(
                AppendInput {
                    venture_id: "ven".into(),
                    actor_id: "actor".into(),
                    action: format!("act-{i}"),
                    resource: "r".into(),
                    capability_id: None,
                    payload: serde_json::json!({}),
                    policy_decision_hash: None,
                },
                device,
            )
            .unwrap();
    }
}

fn save_legacy_ledger_only(ledger: &AuditLedger, path: &std::path::Path) {
    let json = serde_json::to_vec_pretty(ledger.events()).unwrap();
    std::fs::write(path, json).unwrap();
}

#[test]
fn an_old_prefix_ledger_is_rejected_against_a_current_anchor() {
    let device = DeviceIdentity::generate();
    let dir = tempdir().unwrap();
    let ledger_path = dir.path().join("ledger.json");

    let mut ledger = AuditLedger::new();
    append_n(&mut ledger, &device, 3);
    ledger.save(&ledger_path, &device).unwrap();
    let anchor = LedgerHead::load(&ledger_head_path(&ledger_path)).unwrap();
    assert_eq!(anchor.event_count, 3);

    let prefix_events = ledger.events()[..2].to_vec();
    save_legacy_ledger_only(
        &AuditLedger::from_events(prefix_events, device.public_key_b64()).unwrap(),
        &ledger_path,
    );

    let loaded = AuditLedger::load(&ledger_path, device.public_key_b64()).unwrap();
    let err = verify_freshness(&loaded, Some(&anchor), true).unwrap_err();
    assert!(matches!(err, LedgerError::Rewound));
}

#[test]
fn a_forked_chain_at_the_anchored_index_is_rejected() {
    let device = DeviceIdentity::generate();
    let dir = tempdir().unwrap();
    let ledger_path = dir.path().join("ledger.json");

    let mut ledger = AuditLedger::new();
    append_n(&mut ledger, &device, 2);
    ledger.save(&ledger_path, &device).unwrap();
    let anchor = LedgerHead::load(&ledger_head_path(&ledger_path)).unwrap();

    let mut forked = ledger.events().to_vec();
    forked[1].action = "forked-action".into();
    forked[1].event_hash = hash_event_body(&AuditEventBody::from(&forked[1]));
    forked[1].device_signature = Some(device.sign_legacy_v1(forked[1].event_hash.as_bytes()));

    let forked_ledger = AuditLedger::from_events(forked, device.public_key_b64()).unwrap();
    forked_ledger.verify_chain().unwrap();

    let err = verify_freshness(&forked_ledger, Some(&anchor), true).unwrap_err();
    assert!(matches!(err, LedgerError::Forked));
}

#[test]
fn a_forward_extension_of_the_anchored_head_is_accepted() {
    let device = DeviceIdentity::generate();
    let dir = tempdir().unwrap();
    let ledger_path = dir.path().join("ledger.json");

    let mut ledger = AuditLedger::new();
    append_n(&mut ledger, &device, 2);
    ledger.save(&ledger_path, &device).unwrap();
    let anchor = LedgerHead::load(&ledger_head_path(&ledger_path)).unwrap();

    append_n(&mut ledger, &device, 1);
    save_legacy_ledger_only(&ledger, &ledger_path);

    let loaded = AuditLedger::load(&ledger_path, device.public_key_b64()).unwrap();
    verify_freshness(&loaded, Some(&anchor), true).unwrap();
}

#[test]
fn a_missing_anchor_over_a_previously_anchored_ledger_fails_closed() {
    let device = DeviceIdentity::generate();
    let dir = tempdir().unwrap();
    let ledger_path = dir.path().join("ledger.json");

    let mut ledger = AuditLedger::new();
    append_n(&mut ledger, &device, 1);
    ledger.save(&ledger_path, &device).unwrap();
    assert!(ledger_head_path(&ledger_path).is_file());

    std::fs::remove_file(ledger_head_path(&ledger_path)).unwrap();
    let loaded = AuditLedger::load(&ledger_path, device.public_key_b64()).unwrap();
    let err = verify_freshness(&loaded, None, true).unwrap_err();
    assert!(matches!(err, LedgerError::MissingAnchor));
}

#[test]
fn a_first_run_and_a_never_anchored_legacy_ledger_initialize() {
    let device = DeviceIdentity::generate();
    let dir = tempdir().unwrap();
    let ledger_path = dir.path().join("ledger.json");

    let empty = AuditLedger::new();
    verify_freshness(&empty, None, false).unwrap();

    let mut legacy = AuditLedger::new();
    append_n(&mut legacy, &device, 2);
    save_legacy_ledger_only(&legacy, &ledger_path);
    assert!(!ledger_head_path(&ledger_path).exists());

    let loaded = AuditLedger::load(&ledger_path, device.public_key_b64()).unwrap();
    verify_freshness(&loaded, None, false).unwrap();
}

#[test]
#[allow(non_snake_case)]
fn the_signed_AuditEventBody_shape_is_unchanged() {
    const TOKEN_ID: &str = "11111111-2222-3333-4444-555555555555";
    const EVENT_ID: &str = "bbbbbbbb-cccc-dddd-eeee-ffffffffffff";
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

    fn at(secs: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(secs, 0).expect("valid timestamp")
    }

    let event = AuditEvent {
        event_id: Uuid::parse_str(EVENT_ID).unwrap(),
        venture_id: "venture-alpha".to_owned(),
        actor_id: "agent-drafting".to_owned(),
        action: "effect.written".to_owned(),
        resource: "outbox/alpha.eml".to_owned(),
        capability_id: Some(Uuid::parse_str(TOKEN_ID).unwrap()),
        timestamp: at(1_767_225_600),
        payload_hash: "payload-hash".to_owned(),
        previous_event_hash: "previous-hash".to_owned(),
        policy_decision_hash: Some("decision-hash".to_owned()),
        device_public_key_b64: "ZGV2aWNlLWtleQ==".to_owned(),
        event_hash: "event-hash".to_owned(),
        device_signature: Some("ZGV2aWNlLXNpZw==".to_owned()),
    };

    let json = serde_json::to_string(&AuditEventBody::from(&event)).expect("body must serialize");
    assert_eq!(
        json, AUDIT_EVENT_BODY_GOLDEN,
        "AuditEventBody wire shape changed; RFC 0007 forbids that"
    );
}

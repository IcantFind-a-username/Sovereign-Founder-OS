//! RFC 0007 workspace open-time freshness (ledger.head).

use super::*;
use sovereign_audit_ledger::{ledger_head_path, AuditLedger};
use sovereign_identity::DeviceIdentity;
use tempfile::tempdir;

#[test]
fn a_reverted_ledger_is_refused_at_open() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    store.set_venture("Acme", "consulting").unwrap();
    store
        .add_customer("Dr. Tan", "dr.tan@example.com", "met at expo")
        .unwrap();

    let ledger_path = dir.path().join("ledger.json");
    let device = DeviceIdentity::load(&dir.path().join("device.json")).unwrap();
    let ledger = AuditLedger::load(&ledger_path, device.public_key_b64()).unwrap();
    assert!(ledger.events().len() >= 2);
    assert!(
        ledger_head_path(&ledger_path).is_file(),
        "anchor must exist after commits"
    );

    // Prefix restore of ledger.json only; anchor still reflects the longer chain.
    let prefix = ledger.events()[..1].to_vec();
    let json = serde_json::to_vec_pretty(&prefix).unwrap();
    std::fs::write(&ledger_path, json).unwrap();

    match Store::open(dir.path()) {
        Err(err) => {
            let msg = err.to_string();
            assert!(
                msg.contains("rewound") || msg.contains("freshness anchor"),
                "expected a rewound-ledger refusal, got: {msg}"
            );
        }
        Ok(_) => panic!("open must refuse a prefix-restored ledger with a current anchor"),
    }
}

#[test]
fn a_whole_directory_rollback_is_not_detected_documented_boundary() {
    // Honest boundary (RFC 0007, THREAT_MODEL T10 Research/deferred): an actor
    // who can write the whole workspace directory can re-sign ledger.head for
    // any prefix. This mechanism detects prefix restore only when the anchor
    // outlives the ledger — not whole-device rollback.
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    store.set_venture("Acme", "consulting").unwrap();
    store
        .add_customer("Dr. Tan", "dr.tan@example.com", "met at expo")
        .unwrap();

    let ledger_path = dir.path().join("ledger.json");
    let device = DeviceIdentity::load(&dir.path().join("device.json")).unwrap();
    let full = AuditLedger::load(&ledger_path, device.public_key_b64()).unwrap();
    assert!(full.events().len() >= 2);

    let prefix_events = full.events()[..1].to_vec();
    let prefix =
        AuditLedger::from_events(prefix_events, device.public_key_b64()).unwrap();
    prefix.save(&ledger_path, &device).unwrap();

    Store::open(dir.path()).expect(
        "whole-directory rollback with a re-signed ledger.head must not be \
         treated as detected — that would overstate the v0.1 boundary",
    );
}

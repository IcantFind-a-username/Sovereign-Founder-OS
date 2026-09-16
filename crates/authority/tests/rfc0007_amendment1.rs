//! RFC 0007 Amendment 1 §d: audit-chain verify is not execute authority.

use sovereign_audit_ledger::{AppendInput, AuditLedger};
use sovereign_authority::{AuthorityError, AuthorityStore, BundlePart};
use sovereign_identity::DeviceIdentity;
use tempfile::tempdir;
use uuid::Uuid;

const NOW: i64 = 1_700_000_000;

fn part(expires_at_unix: i64) -> BundlePart {
    BundlePart {
        id: Uuid::new_v4(),
        expires_at_unix,
    }
}

fn signed_ledger(device: &DeviceIdentity) -> AuditLedger {
    let mut ledger = AuditLedger::new();
    ledger
        .append(
            AppendInput {
                venture_id: "ven".into(),
                actor_id: "actor".into(),
                action: "effect.prepared".into(),
                resource: "outbox/demo.eml".into(),
                capability_id: None,
                payload: serde_json::json!({}),
                policy_decision_hash: None,
            },
            device,
        )
        .unwrap();
    ledger.verify_chain().unwrap();
    ledger
}

#[test]
fn authority_consume_state_not_assumed_fresh_from_audit_chain_alone() {
    let device = DeviceIdentity::generate();
    let ledger = signed_ledger(&device);
    let dir = tempdir().unwrap();
    let store = AuthorityStore::open(dir.path()).unwrap();

    store.bind_freshness_generation(1).unwrap();
    let token = part(NOW + 60);
    let approval = part(NOW + 60);
    let idempotency = part(NOW + 60);
    let fingerprint = [0x42u8; 32];
    store
        .consume_bundle(token, Some(approval), idempotency, &fingerprint, NOW)
        .unwrap();

    store.advance_freshness_generation(2).unwrap();

    // The restored (or still-present) chain still verifies. That fact must
    // not be treated as fresh consume / dispatch / session authority.
    ledger.verify_chain().unwrap();
    assert_eq!(
        store.authorize_new_execute(1),
        Err(AuthorityError::StaleGeneration),
        "generation 1 consume markers are not execute authority after advance"
    );
    assert_eq!(
        store.consume_bundle(token, Some(approval), idempotency, &fingerprint, NOW + 1),
        Err(AuthorityError::StaleGeneration),
        "a pre-restore committed bundle must not authorize a new execute path"
    );

    store.authorize_new_execute(2).unwrap();
    let fresh_token = part(NOW + 60);
    let fresh_approval = part(NOW + 60);
    let fresh_idempotency = part(NOW + 60);
    store
        .consume_bundle(
            fresh_token,
            Some(fresh_approval),
            fresh_idempotency,
            &[0x43u8; 32],
            NOW + 2,
        )
        .expect("new owner-ceremony grants at the recovered generation must still consume");
}

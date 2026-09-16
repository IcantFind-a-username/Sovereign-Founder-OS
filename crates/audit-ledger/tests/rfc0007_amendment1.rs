//! RFC 0007 Amendment 1 conformance tests (enrolled generation).

use sovereign_audit_ledger::{
    claim_enrollment, enrolled_freshness_path, persist_enrolled, recover_enrolled_generation,
    verify_freshness, verify_freshness_and_generation, verify_generation, AppendInput, AuditLedger,
    EnrolledFreshness, FreshnessDisposition, LedgerError, LedgerHead, LEDGER_HEAD_VERSION_V2,
};
use sovereign_identity::DeviceIdentity;
use tempfile::tempdir;

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

fn enroll_and_save(
    ledger: &AuditLedger,
    device: &DeviceIdentity,
    ledger_path: &std::path::Path,
    enrollment_dir: &std::path::Path,
    generation: u64,
    now_unix: u64,
) -> EnrolledFreshness {
    let enrolled =
        EnrolledFreshness::enroll(generation, device.public_key_b64(), now_unix).unwrap();
    persist_enrolled(enrollment_dir, &enrolled).unwrap();
    ledger
        .save_with_generation(ledger_path, device, generation)
        .unwrap();
    enrolled
}

#[test]
fn paired_old_ledger_and_old_anchor_rejected_when_enrolled_generation_survives() {
    let device = DeviceIdentity::generate();
    let dir = tempdir().unwrap();
    let tree = dir.path().join("tree");
    let enrollment = dir.path().join("enrollment");
    std::fs::create_dir_all(&tree).unwrap();
    let ledger_path = tree.join("ledger.json");

    let mut ledger = AuditLedger::new();
    append_n(&mut ledger, &device, 2);
    let old = enroll_and_save(
        &ledger,
        &device,
        &ledger_path,
        &enrollment,
        3,
        1_700_000_000,
    );
    let old_ledger = std::fs::read(&ledger_path).unwrap();
    let old_head = std::fs::read(sovereign_audit_ledger::ledger_head_path(&ledger_path)).unwrap();

    append_n(&mut ledger, &device, 1);
    enroll_and_save(
        &ledger,
        &device,
        &ledger_path,
        &enrollment,
        4,
        1_700_000_100,
    );

    std::fs::write(&ledger_path, old_ledger).unwrap();
    std::fs::write(
        sovereign_audit_ledger::ledger_head_path(&ledger_path),
        old_head,
    )
    .unwrap();

    let restored = AuditLedger::load(&ledger_path, device.public_key_b64()).unwrap();
    restored.verify_chain().unwrap();
    let paired_head =
        LedgerHead::load(&sovereign_audit_ledger::ledger_head_path(&ledger_path)).unwrap();
    verify_freshness(&restored, Some(&paired_head), true).unwrap();

    let enrolled = EnrolledFreshness::load(&enrolled_freshness_path(&enrollment)).unwrap();
    assert_eq!(enrolled.freshness_generation, 4);
    assert_eq!(old.freshness_generation, 3);

    let err =
        verify_freshness_and_generation(&restored, Some(&paired_head), true, Some(&enrolled), true)
            .unwrap_err();
    assert!(
        matches!(err, LedgerError::GenerationDowngrade),
        "paired old ledger+anchor must not open when enrolled generation survives: {err:?}"
    );
}

#[test]
fn a_generation_downgrade_is_rejected_even_with_valid_signatures() {
    let device = DeviceIdentity::generate();
    let dir = tempdir().unwrap();
    let ledger_path = dir.path().join("ledger.json");
    let enrollment = dir.path().join("enrollment");

    let mut newer = AuditLedger::new();
    append_n(&mut newer, &device, 3);
    enroll_and_save(&newer, &device, &ledger_path, &enrollment, 7, 10);

    let mut older = AuditLedger::new();
    append_n(&mut older, &device, 2);
    older
        .save_with_generation(&ledger_path, &device, 4)
        .unwrap();
    let older_head =
        LedgerHead::load(&sovereign_audit_ledger::ledger_head_path(&ledger_path)).unwrap();
    older_head.verify_device_signature().unwrap();
    assert_eq!(older_head.version, LEDGER_HEAD_VERSION_V2);
    assert_eq!(older_head.freshness_generation, Some(4));

    let enrolled = EnrolledFreshness::load(&enrolled_freshness_path(&enrollment)).unwrap();
    assert_eq!(enrolled.freshness_generation, 7);

    older.verify_chain().unwrap();
    let err = verify_generation(&older, Some(&older_head), Some(&enrolled), true).unwrap_err();
    assert!(
        matches!(err, LedgerError::GenerationDowngrade),
        "valid signatures must not authorize a generation downgrade: {err:?}"
    );
}

#[test]
fn missing_enrolled_generation_after_enrollment_enters_limited_recovery() {
    let device = DeviceIdentity::generate();
    let dir = tempdir().unwrap();
    let ledger_path = dir.path().join("ledger.json");
    let enrollment = dir.path().join("enrollment");

    let mut ledger = AuditLedger::new();
    append_n(&mut ledger, &device, 1);
    enroll_and_save(&ledger, &device, &ledger_path, &enrollment, 2, 20);
    claim_enrollment(&enrollment).unwrap();
    assert!(enrolled_freshness_path(&enrollment).is_file());

    std::fs::remove_file(enrolled_freshness_path(&enrollment)).unwrap();
    assert!(sovereign_audit_ledger::enrollment_is_claimed(&enrollment));

    let loaded = AuditLedger::load(&ledger_path, device.public_key_b64()).unwrap();
    let head = LedgerHead::load(&sovereign_audit_ledger::ledger_head_path(&ledger_path)).unwrap();
    loaded.verify_chain().unwrap();
    verify_freshness(&loaded, Some(&head), true).unwrap();

    let disposition = verify_generation(&loaded, Some(&head), None, true).unwrap();
    assert_eq!(
        disposition,
        FreshnessDisposition::LimitedRecovery,
        "missing enrolled state after a claim must not silently accept the on-disk anchor"
    );

    let recovered =
        recover_enrolled_generation(device.public_key_b64(), 30, &[&head], None).unwrap();
    assert!(
        recovered.freshness_generation > head.freshness_generation.unwrap(),
        "limited recovery must bump strictly above restored artifacts"
    );
}

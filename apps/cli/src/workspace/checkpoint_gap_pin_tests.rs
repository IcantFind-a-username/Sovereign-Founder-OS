//! Pins fail-closed behavior for the workflow checkpoint gap (v0.1 wave B).

use super::*;
use sovereign_audit_ledger::AuditLedger;

fn store() -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    (dir, store)
}

/// Pin: a kill after step 1 finishes (outbox + persisted record) but before
/// the workflow checkpoint is written makes the retry re-run execute with a
/// fresh capability — **two** consumed token records on disk — while orphan
/// pre-clean keeps exactly **one** `.eml`. If either count changes, the
/// double-burn behavior changed and needs a recorded product decision.
#[test]
fn a_kill_between_outbox_write_and_checkpoint_burns_fresh_authority_but_never_double_sends() {
    use super::send_workflow::ExecuteSendStep;
    use sovereign_workflow::{StepContext, WorkflowRunner, WorkflowStep};

    let (dir, store) = store();
    store.set_venture("Acme", "Landing pages").unwrap();
    let workspace = store
        .add_customer("Dr. Tan", "dr.tan@example.com", "")
        .unwrap();
    let customer_id = workspace.customers[0].id;
    let workspace = store
        .create_document(DocumentKind::Invoice, customer_id, Some(250_000), "en")
        .unwrap();
    let document_id = workspace.documents[0].id;
    let workspace = store.request_send(document_id).unwrap();
    let approval_id = workspace.approvals[0].id;

    let workflow_id = format!("send-{approval_id}");
    let wf_dir = dir.path().join("workflows").join(&workflow_id);
    let record_path = wf_dir.join("record.json");
    std::fs::create_dir_all(&wf_dir).unwrap();
    let _runner = WorkflowRunner::open(&wf_dir, &workflow_id).unwrap();

    // Step 1 without a workflow checkpoint: outbox + record.json land, then
    // we "die" before persist().
    let execute = ExecuteSendStep {
        root: dir.path().to_path_buf(),
        record_path: record_path.clone(),
        approval_id,
        document_id,
    };
    let context = StepContext {
        workflow_id: &workflow_id,
        prior: &[],
    };
    execute.run(&context).unwrap();
    assert!(
        !wf_dir.join("checkpoint.json").exists(),
        "simulated crash must leave no workflow checkpoint"
    );
    assert_eq!(
        store.load().unwrap().approvals[0].status,
        ApprovalStatus::Pending,
        "state must stay pending until commit"
    );
    assert_eq!(
        std::fs::read_dir(dir.path().join("outbox"))
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().extension().is_some_and(|x| x == "eml"))
            .count(),
        1,
        "first execute wrote one message before the kill"
    );

    let token_claims = || -> usize {
        std::fs::read_dir(dir.path().join("authority/tokens"))
            .map(|entries| entries.count())
            .unwrap_or(0)
    };
    assert_eq!(token_claims(), 1, "first execute consumed one token");

    // Owner retries approval: no checkpoint → full execute again, then commit.
    let workspace = store.decide(approval_id, true).unwrap();
    assert_eq!(workspace.approvals[0].status, ApprovalStatus::Approved);
    assert_eq!(
        workspace.documents[0].status,
        DocumentStatus::ApprovedPendingDelivery
    );

    assert_eq!(
        std::fs::read_dir(dir.path().join("outbox"))
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().extension().is_some_and(|x| x == "eml"))
            .count(),
        1,
        "orphan pre-clean must prevent a second on-disk send"
    );
    assert_eq!(
        token_claims(),
        2,
        "pin: two consumed token records after the checkpoint-gap retry"
    );

    let evidence = workspace.approvals[0].evidence.as_ref().unwrap();
    let outbox = evidence.outbox.as_ref().unwrap();
    let eml_path = dir.path().join("outbox").join(&outbox.relative_path);
    let on_disk = std::fs::read(&eml_path).unwrap();
    assert_eq!(
        sovereign_audit_ledger::hash_bytes(&on_disk),
        outbox.content_sha256,
        "the committed delivery record must match the sole .eml"
    );

    let recovered = sovereign_execution::ExecutionJournal::open(dir.path().join("executions"))
        .unwrap()
        .recover()
        .unwrap();
    assert_eq!(
        recovered.len(),
        2,
        "each execute pass left its own journal record"
    );
    assert!(
        recovered.iter().all(|record| matches!(
            record.state,
            sovereign_execution::ExecutionState::Completed { .. }
        )),
        "both runs completed sandbox execution before the kill/retry"
    );

    let device = DeviceIdentity::load(&dir.path().join("device.json")).unwrap();
    let ledger =
        AuditLedger::load(&dir.path().join("ledger.json"), device.public_key_b64()).unwrap();
    assert_eq!(
        ledger
            .events()
            .iter()
            .filter(|event| event.action == "capability.executed")
            .count(),
        1,
        "audit chain records one authorized effect, not two sends"
    );
}

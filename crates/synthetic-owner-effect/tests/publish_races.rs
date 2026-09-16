//! v01-D07: same-process concurrent dispatch reconcile race.
//!
//! Design Accept ≠ product Current. Full cross-process validator race is Target.
//! Not 1C0, Exact Effect, ActiveV2, or RP1.

#![cfg(feature = "owner-effect-fixture")]

use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

use sovereign_synthetic_owner_effect::{
    inspect_intent_state, listed_evidence, publish_once, reconcile_without_writing,
    reserve_exact_authority, with_publish_failpoint, ClosedOutcome, IntentState, PublishError,
    PublishFailpoint,
};

#[path = "support/proofs.rs"]
mod proofs;
#[path = "support/root.rs"]
mod root;

#[test]
fn same_process_concurrent_dispatch_reconcile_has_one_terminal_winner() {
    let dir = tempfile::tempdir().unwrap();
    let root = root::marked_root(dir.path());
    let mut harness = proofs::Harness::boot(&root);
    let issued = harness.issue_for_new_intent();
    let (capability, approval) = harness.verify(
        &issued.token,
        &issued.signed_approval,
        issued.intent_id,
        issued.context.now_unix,
    );
    let live = proofs::live_context(&issued);
    let store = harness.owner.open_store().unwrap();
    let reserved = reserve_exact_authority(&store, &issued.context, capability, approval).unwrap();
    let error = with_publish_failpoint(PublishFailpoint::AfterDispatchingCommit, || {
        publish_once(&store, &root, reserved, &live)
    });
    assert_eq!(
        error,
        Err(PublishError::Failpoint(
            PublishFailpoint::AfterDispatchingCommit
        ))
    );
    assert_eq!(
        inspect_intent_state(&store, issued.intent_id).unwrap(),
        Some(IntentState::Dispatching)
    );

    let wins = AtomicUsize::new(0);
    thread::scope(|scope| {
        for _ in 0..2 {
            scope.spawn(|| {
                match reconcile_without_writing(&store, &root, issued.intent_id, live.signer_epoch)
                {
                    Ok(ClosedOutcome::Indeterminate) => {
                        wins.fetch_add(1, Ordering::SeqCst);
                    }
                    Err(PublishError::Unavailable) => {}
                    other => panic!("unexpected dispatch race outcome: {other:?}"),
                }
            });
        }
    });
    assert!(
        wins.load(Ordering::SeqCst) >= 1,
        "at least one thread must observe the terminal close"
    );
    assert_eq!(
        inspect_intent_state(&store, issued.intent_id).unwrap(),
        Some(IntentState::Indeterminate)
    );
    let evidence = listed_evidence(&store).unwrap();
    assert_eq!(
        evidence.len(),
        1,
        "the same intent appends value-free evidence once"
    );
    assert_eq!(evidence[0].outcome, ClosedOutcome::Indeterminate.as_str());
}

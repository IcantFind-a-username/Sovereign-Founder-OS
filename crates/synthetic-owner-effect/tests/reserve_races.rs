//! v01-D05: same-process races against one reservation transaction.
//!
//! Design Accept ≠ product Current. Full cross-process validator race is Target.

#![cfg(feature = "owner-effect-fixture")]

use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

use sovereign_synthetic_owner_effect::{
    inspect_reservation, reserve_exact_authority, revoke_approval, revoke_token, ReserveError,
};

#[path = "support/proofs.rs"]
mod proofs;
#[path = "support/root.rs"]
mod root;

#[test]
fn same_process_concurrent_http_or_thread_reservation_has_one_winner() {
    let dir = tempfile::tempdir().unwrap();
    let mut harness = proofs::Harness::boot(&root::marked_root(dir.path()));
    let issued = harness.issue_for_new_intent();
    let store = harness.owner.open_store().unwrap();
    let wins = AtomicUsize::new(0);
    let losses = AtomicUsize::new(0);
    thread::scope(|scope| {
        for _ in 0..2 {
            scope.spawn(|| {
                let (capability, approval) = harness.verify(
                    &issued.token,
                    &issued.signed_approval,
                    issued.intent_id,
                    issued.context.now_unix,
                );
                match reserve_exact_authority(&store, &issued.context, capability, approval) {
                    Ok(_) => {
                        wins.fetch_add(1, Ordering::SeqCst);
                    }
                    Err(ReserveError::IdempotencyReplay)
                    | Err(ReserveError::ApprovalAlreadySpent)
                    | Err(ReserveError::TokenAlreadyConsumed)
                    | Err(ReserveError::EffectAlreadyReserved) => {
                        losses.fetch_add(1, Ordering::SeqCst);
                    }
                    Err(other) => panic!("unexpected reservation outcome: {other:?}"),
                }
            });
        }
    });
    assert_eq!(wins.load(Ordering::SeqCst), 1, "exactly one winner");
    assert_eq!(
        losses.load(Ordering::SeqCst),
        1,
        "the loser must be visible"
    );
    let view = inspect_reservation(
        &store,
        issued.intent_id,
        issued.approval_id,
        issued.token_id,
        issued.idempotency_key,
        issued.context.fixture_generation,
    )
    .unwrap();
    assert!(proofs::all_of_the_reservation(
        &view,
        issued.intent_id,
        issued.approval_expires_at_unix
    ));
}

#[test]
fn expiry_race_refuses_and_commits_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let mut harness = proofs::Harness::boot(&root::marked_root(dir.path()));
    let mut issued = harness.issue_for_new_intent();
    issued.context.now_unix = issued.approval_expires_at_unix;
    let (capability, approval) = harness.verify(
        &issued.token,
        &issued.signed_approval,
        issued.intent_id,
        proofs::unix_now(),
    );
    let store = harness.owner.open_store().unwrap();
    let error = proofs::refuse(
        reserve_exact_authority(&store, &issued.context, capability, approval),
        "an expired approval must not reserve",
    );
    assert_eq!(error, ReserveError::Expired);
    let view = inspect_reservation(
        &store,
        issued.intent_id,
        issued.approval_id,
        issued.token_id,
        issued.idempotency_key,
        issued.context.fixture_generation,
    )
    .unwrap();
    assert!(proofs::none_of_the_reservation(&view));
}

#[test]
fn logout_race_refuses_and_commits_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let mut harness = proofs::Harness::boot(&root::marked_root(dir.path()));
    let mut issued = harness.issue_for_new_intent();
    harness.logout();
    issued.context.logout_epoch = harness.owner.surface().logout_epoch();
    let (capability, approval) = harness.verify(
        &issued.token,
        &issued.signed_approval,
        issued.intent_id,
        issued.context.now_unix,
    );
    let store = harness.owner.open_store().unwrap();
    let error = proofs::refuse(
        reserve_exact_authority(&store, &issued.context, capability, approval),
        "logout must invalidate an unreserved approval",
    );
    assert_eq!(error, ReserveError::LogoutMismatch);
    let view = inspect_reservation(
        &store,
        issued.intent_id,
        issued.approval_id,
        issued.token_id,
        issued.idempotency_key,
        issued.context.fixture_generation,
    )
    .unwrap();
    assert!(proofs::none_of_the_reservation(&view));
}

#[test]
fn signer_epoch_race_refuses_and_commits_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let mut harness = proofs::Harness::boot(&root::marked_root(dir.path()));
    let mut issued = harness.issue_for_new_intent();
    issued.context.signer_epoch = [0x22; 16];
    let (capability, approval) = harness.verify(
        &issued.token,
        &issued.signed_approval,
        issued.intent_id,
        issued.context.now_unix,
    );
    let store = harness.owner.open_store().unwrap();
    let error = proofs::refuse(
        reserve_exact_authority(&store, &issued.context, capability, approval),
        "a rotated signer epoch must not reserve",
    );
    assert_eq!(error, ReserveError::EpochMismatch);
    let view = inspect_reservation(
        &store,
        issued.intent_id,
        issued.approval_id,
        issued.token_id,
        issued.idempotency_key,
        issued.context.fixture_generation,
    )
    .unwrap();
    assert!(proofs::none_of_the_reservation(&view));
}

#[test]
fn revocation_race_refuses_and_commits_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let mut harness = proofs::Harness::boot(&root::marked_root(dir.path()));
    let issued = harness.issue_for_new_intent();
    {
        let store = harness.owner.open_store().unwrap();
        revoke_token(&store, issued.token_id).unwrap();
        revoke_approval(&store, issued.approval_id).unwrap();
    }
    let (capability, approval) = harness.verify(
        &issued.token,
        &issued.signed_approval,
        issued.intent_id,
        issued.context.now_unix,
    );
    let store = harness.owner.open_store().unwrap();
    let error = proofs::refuse(
        reserve_exact_authority(&store, &issued.context, capability, approval),
        "a revoked token must not reserve",
    );
    assert_eq!(error, ReserveError::Revoked);
    let view = inspect_reservation(
        &store,
        issued.intent_id,
        issued.approval_id,
        issued.token_id,
        issued.idempotency_key,
        issued.context.fixture_generation,
    )
    .unwrap();
    assert!(proofs::none_of_the_reservation(&view));
}

//! v01-D05: one redb write reserves every authority fact, or none of them.
//!
//! Design Accept ≠ product Current. Not 1C0, Exact Effect, or publish.

#![cfg(feature = "owner-effect-fixture")]

use redb::TableDefinition;
use sovereign_authority::broker::store::StoreError;
use sovereign_synthetic_owner_effect::{
    inspect_reservation, reserve_exact_authority, with_failpoint, FixtureOwner, IntentState,
    ReservationFailpoint, ReserveError, SYNTHETIC_NODE_INITIAL_USES,
};

#[path = "support/proofs.rs"]
mod proofs;
#[path = "support/root.rs"]
mod root;

fn reservation_source() -> &'static str {
    include_str!("../src/reserve.rs")
}

#[test]
fn coordinator_never_calls_the_legacy_consuming_validator() {
    let source = reservation_source();
    assert!(!source.contains("authorize_and_consume"));
    assert!(!source.contains("ReservationRequest"));
    assert!(source.contains("VerifiedCapabilityV2"));
    assert!(source.contains("VerifiedApprovalV1"));
}

#[test]
fn a_first_reservation_commits_every_fact_together() {
    let dir = tempfile::tempdir().unwrap();
    let mut harness = proofs::Harness::boot(&root::marked_root(dir.path()));
    let issued = harness.issue_for_new_intent();
    let (capability, approval) = harness.verify(
        &issued.token,
        &issued.signed_approval,
        issued.intent_id,
        issued.context.now_unix,
    );
    let store = harness.owner.open_store().unwrap();
    reserve_exact_authority(&store, &issued.context, capability, approval).unwrap();
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
    assert_eq!(
        view.authority_uses_remaining,
        SYNTHETIC_NODE_INITIAL_USES - 1
    );
}

#[test]
fn failpoint_between_every_logical_mutation_rolls_all_of_them_back() {
    for stage in ReservationFailpoint::ALL_MUTATIONS {
        let dir = tempfile::tempdir().unwrap();
        let mut harness = proofs::Harness::boot(&root::marked_root(dir.path()));
        let issued = harness.issue_for_new_intent();
        let (capability, approval) = harness.verify(
            &issued.token,
            &issued.signed_approval,
            issued.intent_id,
            issued.context.now_unix,
        );
        let store = harness.owner.open_store().unwrap();
        let error = proofs::refuse(
            with_failpoint(*stage, || {
                reserve_exact_authority(&store, &issued.context, capability, approval)
            }),
            "failpoint must abort the transaction",
        );
        assert_eq!(error, ReserveError::Failpoint(*stage));
        let view = inspect_reservation(
            &store,
            issued.intent_id,
            issued.approval_id,
            issued.token_id,
            issued.idempotency_key,
            issued.context.fixture_generation,
        )
        .unwrap();
        assert!(
            proofs::none_of_the_reservation(&view),
            "failpoint {stage:?} committed a subset: {view:?}"
        );
        assert_eq!(view.intent_state, Some(IntentState::Prepared));
        assert_eq!(view.authority_uses_remaining, SYNTHETIC_NODE_INITIAL_USES);
    }
}

#[test]
fn same_key_idempotent_replay_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let mut harness = proofs::Harness::boot(&root::marked_root(dir.path()));
    let issued = harness.issue_for_new_intent();
    let (capability, approval) = harness.verify(
        &issued.token,
        &issued.signed_approval,
        issued.intent_id,
        issued.context.now_unix,
    );
    let store = harness.owner.open_store().unwrap();
    reserve_exact_authority(&store, &issued.context, capability, approval).unwrap();
    let (capability, approval) = {
        drop(store);
        harness.verify(
            &issued.token,
            &issued.signed_approval,
            issued.intent_id,
            issued.context.now_unix,
        )
    };
    let store = harness.owner.open_store().unwrap();
    let error = proofs::refuse(
        reserve_exact_authority(&store, &issued.context, capability, approval),
        "same-key retry must be a replay",
    );
    assert_eq!(error, ReserveError::IdempotencyReplay);
}

#[test]
fn different_intent_conflicts_on_the_same_idempotency_key() {
    let dir = tempfile::tempdir().unwrap();
    let mut harness = proofs::Harness::boot(&root::marked_root(dir.path()));
    let first = harness.issue_for_new_intent();
    let second = harness.issue_for_new_intent();
    {
        let store = harness.owner.open_store().unwrap();
        const IDEMPOTENCY: TableDefinition<&[u8], &[u8]> =
            TableDefinition::new("fixture-reserved-idempotency-v1");
        store
            .write(|transaction| -> Result<(), ReserveError> {
                transaction
                    .open_table(IDEMPOTENCY)
                    .map_err(|_| StoreError::Unavailable)?
                    .insert(
                        second.idempotency_key.as_bytes().as_slice(),
                        first.intent_id.as_uuid().as_bytes().as_slice(),
                    )
                    .map_err(|_| StoreError::Unavailable)?;
                Ok(())
            })
            .unwrap();
    }
    let (capability, approval) = harness.verify(
        &second.token,
        &second.signed_approval,
        second.intent_id,
        second.context.now_unix,
    );
    let store = harness.owner.open_store().unwrap();
    let error = proofs::refuse(
        reserve_exact_authority(&store, &second.context, capability, approval),
        "a key bound to another intent must conflict",
    );
    assert_eq!(error, ReserveError::IdempotencyConflict);
    let view = inspect_reservation(
        &store,
        second.intent_id,
        second.approval_id,
        second.token_id,
        second.idempotency_key,
        second.context.fixture_generation,
    )
    .unwrap();
    assert!(!view.approval_claimed);
    assert!(!view.token_claimed);
    assert_eq!(view.intent_state, Some(IntentState::Prepared));
    assert_eq!(
        view.idempotency_bound_to,
        Some(first.intent_id.as_uuid()),
        "conflict must not rebind the key"
    );
    assert_eq!(view.authority_uses_remaining, SYNTHETIC_NODE_INITIAL_USES);
}

#[test]
fn reopen_sees_either_all_or_none_of_the_reservation() {
    let dir = tempfile::tempdir().unwrap();
    let root = root::marked_root(dir.path());
    let mut harness = proofs::Harness::boot(&root);
    let issued = harness.issue_for_new_intent();
    let generation = issued.context.fixture_generation;
    let ids = (
        issued.intent_id,
        issued.approval_id,
        issued.token_id,
        issued.idempotency_key,
        issued.approval_expires_at_unix,
    );
    {
        let (capability, approval) = harness.verify(
            &issued.token,
            &issued.signed_approval,
            issued.intent_id,
            issued.context.now_unix,
        );
        let store = harness.owner.open_store().unwrap();
        let view = inspect_reservation(&store, ids.0, ids.1, ids.2, ids.3, generation).unwrap();
        assert!(
            proofs::none_of_the_reservation(&view),
            "Prepared must not already be a reservation"
        );
        reserve_exact_authority(&store, &issued.context, capability, approval).unwrap();
    }
    drop(harness);

    let owner = FixtureOwner::boot(&root).expect("reopen after a committed reservation");
    let store = owner.open_store().unwrap();
    let view = inspect_reservation(&store, ids.0, ids.1, ids.2, ids.3, generation).unwrap();
    assert!(proofs::all_of_the_reservation(&view, ids.0, ids.4));
}

//! Reserving everything an effect needs, in one transaction or not at all.
//!
//! Checking six things and then acting is the classic mistake, and it fails
//! in two directions at once. Between the check and the act another caller
//! can consume the token, so the check was worthless. And if the fourth
//! reservation fails after three succeeded, three one-use things are spent on
//! an effect that will never happen — which no retry can recover, because
//! they are one-use by design.

#![cfg(feature = "owner-effect-fixture")]

use sovereign_authority::broker::process_lock::acquire;
use sovereign_authority::broker::reservation::{
    is_reserved, reserve, Kind, ReservationError, ReservationRequest,
};
use sovereign_authority::broker::store::OwnedStore;
use uuid::Uuid;

fn request() -> ReservationRequest {
    ReservationRequest {
        approval_id: Uuid::new_v4(),
        token_id: Uuid::new_v4(),
        idempotency_key: Uuid::new_v4(),
        invocation_fingerprint: [0x33; 32],
        effect_intent_id: Uuid::new_v4(),
        root_generation: 1,
    }
}

/// Run `body` against a store in a fresh locked root.
fn with_store(body: impl FnOnce(&OwnedStore<'_>)) {
    let dir = tempfile::tempdir().unwrap();
    let lock = acquire(dir.path()).unwrap();
    let store = OwnedStore::open(dir.path(), &lock).unwrap();
    body(&store);
}

#[test]
fn a_first_reservation_takes_everything() {
    with_store(|store| {
        let request = request();
        assert!(reserve(store, &request).is_ok());

        for (kind, key) in [
            (Kind::Approval, request.approval_id),
            (Kind::Token, request.token_id),
            (Kind::Effect, request.effect_intent_id),
        ] {
            assert!(
                is_reserved(store, kind, key).unwrap(),
                "{kind:?} was not reserved"
            );
        }
    });
}

/// The property. A reservation that fails part-way must leave *nothing*
/// reserved — the transaction is dropped rather than committed, so there is
/// no partial state to clean up because there is no partial state.
#[test]
fn a_failed_reservation_leaves_nothing_reserved() {
    with_store(|store| {
        // A token already spent by an earlier effect.
        let first = request();
        reserve(store, &first).unwrap();

        // A new effect that reuses that token: its approval and effect id are
        // fresh, and the token is not.
        let second = ReservationRequest {
            token_id: first.token_id,
            ..request()
        };
        assert_eq!(
            reserve(store, &second),
            Err(ReservationError::TokenAlreadyConsumed)
        );

        // The approval and effect id it would have taken are untouched, so
        // the caller can retry with a different token and lose nothing.
        assert!(
            !is_reserved(store, Kind::Approval, second.approval_id).unwrap(),
            "a failed reservation spent the approval"
        );
        assert!(
            !is_reserved(store, Kind::Effect, second.effect_intent_id).unwrap(),
            "a failed reservation spent the effect id"
        );
    });
}

/// And the retry works: nothing was consumed, so a corrected request is a
/// first attempt rather than a doomed one.
#[test]
fn a_corrected_request_succeeds_after_a_failure() {
    with_store(|store| {
        let first = request();
        reserve(store, &first).unwrap();

        let clashing = ReservationRequest {
            approval_id: first.approval_id,
            ..request()
        };
        assert_eq!(
            reserve(store, &clashing),
            Err(ReservationError::ApprovalAlreadySpent)
        );

        let corrected = ReservationRequest {
            approval_id: Uuid::new_v4(),
            ..clashing.clone()
        };
        assert!(
            reserve(store, &corrected).is_ok(),
            "a corrected request failed, so the first attempt spent something"
        );
    });
}

/// Each one-use claim reports which one was already spent, so a caller can
/// tell "someone else used my token" from "I already approved this".
#[test]
fn each_kind_reports_its_own_outcome() {
    for (build, expected) in [
        (
            (|first: &ReservationRequest| ReservationRequest {
                approval_id: first.approval_id,
                ..request()
            }) as fn(&ReservationRequest) -> ReservationRequest,
            ReservationError::ApprovalAlreadySpent,
        ),
        (
            |first: &ReservationRequest| ReservationRequest {
                token_id: first.token_id,
                ..request()
            },
            ReservationError::TokenAlreadyConsumed,
        ),
        (
            |first: &ReservationRequest| ReservationRequest {
                effect_intent_id: first.effect_intent_id,
                ..request()
            },
            ReservationError::EffectAlreadyReserved,
        ),
    ] {
        with_store(|store| {
            let first = request();
            reserve(store, &first).unwrap();
            assert_eq!(reserve(store, &build(&first)), Err(expected));
        });
    }
}

/// A genuine retry — same key, same fingerprint — is a replay, and reserves
/// nothing new.
///
/// The name says what this proves and no more. It first read "recognised
/// before anything is spent", which sounded stronger and was untestable:
/// reversing the checks inside the transaction left it passing, because a
/// replay detected after the claims were written still aborts and takes those
/// writes with it. The guarantee is atomicity, not ordering.
#[test]
fn a_replay_reserves_nothing_new() {
    with_store(|store| {
        let first = request();
        reserve(store, &first).unwrap();

        let retry = ReservationRequest {
            approval_id: Uuid::new_v4(),
            token_id: Uuid::new_v4(),
            effect_intent_id: Uuid::new_v4(),
            ..first.clone()
        };
        assert_eq!(
            reserve(store, &retry),
            Err(ReservationError::IdempotencyReplay)
        );

        // None of the retry's fresh one-use values were taken.
        assert!(!is_reserved(store, Kind::Approval, retry.approval_id).unwrap());
        assert!(!is_reserved(store, Kind::Token, retry.token_id).unwrap());
        assert!(!is_reserved(store, Kind::Effect, retry.effect_intent_id).unwrap());
    });
}

/// The same key with a different fingerprint is a conflict, not a replay.
/// Confusing the two would let a caller reuse a key for a different action.
#[test]
fn the_same_key_with_a_different_fingerprint_conflicts() {
    with_store(|store| {
        let first = request();
        reserve(store, &first).unwrap();

        let different = ReservationRequest {
            invocation_fingerprint: [0x99; 32],
            approval_id: Uuid::new_v4(),
            token_id: Uuid::new_v4(),
            effect_intent_id: Uuid::new_v4(),
            ..first.clone()
        };
        assert_eq!(
            reserve(store, &different),
            Err(ReservationError::IdempotencyConflict)
        );
    });
}

/// A reservation survives the store being closed and reopened: it is durable,
/// not a lock held in memory.
#[test]
fn a_reservation_survives_reopening_the_store() {
    let dir = tempfile::tempdir().unwrap();
    let request = request();

    {
        let lock = acquire(dir.path()).unwrap();
        let store = OwnedStore::open(dir.path(), &lock).unwrap();
        reserve(&store, &request).unwrap();
    }

    let lock = acquire(dir.path()).unwrap();
    let store = OwnedStore::open(dir.path(), &lock).unwrap();
    assert!(is_reserved(&store, Kind::Token, request.token_id).unwrap());
    assert_eq!(
        reserve(&store, &request),
        Err(ReservationError::IdempotencyReplay),
        "a reservation did not survive a reopen"
    );
}

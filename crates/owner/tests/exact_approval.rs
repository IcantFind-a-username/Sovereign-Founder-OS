//! An approval is for one exact invocation, and a session is not one.
//!
//! The first test is the one the whole product rests on. A session says
//! someone logged in at some point; an approval says a person was present for
//! *this specific thing*, just now. Treating the first as the second is how a
//! click on one page authorises an action on another.

#![cfg(feature = "owner-effect-fixture")]

use sovereign_owner::approval::{ApprovalError, ApprovalStore, Binding, APPROVAL_LIFETIME};
use std::time::{Duration, Instant};
use uuid::Uuid;

fn binding(session: Uuid) -> Binding {
    Binding {
        intent_id: Uuid::new_v4(),
        invocation_id: Uuid::new_v4(),
        policy_decision_digest: [0x11; 32],
        session_id: session,
        fixture_generation: 1,
    }
}

/// Holding a session reaches this function and is refused by it. Freshness is
/// an argument rather than an assumption, for the same reason it is in the
/// registry: a caller that could omit the flag could omit the verification.
#[test]
fn session_alone_cannot_approve() {
    let mut store = ApprovalStore::new();
    let now = Instant::now();
    let challenge = Uuid::new_v4();
    let bound = binding(Uuid::new_v4());

    store.begin(challenge, bound.clone(), now);
    assert_eq!(
        store.finish(challenge, &bound, false, now).err(),
        Some(ApprovalError::NotFreshlyUserVerified),
        "a session with no fresh verification produced an approval"
    );
}

#[test]
fn a_fresh_ceremony_binds_all_five_fields() {
    let mut store = ApprovalStore::new();
    let now = Instant::now();
    let challenge = Uuid::new_v4();
    let bound = binding(Uuid::new_v4());

    store.begin(challenge, bound.clone(), now);
    let approval = store.finish(challenge, &bound, true, now).unwrap();
    assert!(store.verify(&approval, &bound).is_ok());
}

/// Change any one of the five and the approval is for something else. Each is
/// varied on its own, because an implementation that bound four of them would
/// pass a test that changed them all at once.
#[test]
fn changing_any_single_binding_invalidates_the_approval() {
    let session = Uuid::new_v4();
    let bound = binding(session);

    let variants: Vec<(&str, Binding)> = vec![
        (
            "intent",
            Binding {
                intent_id: Uuid::new_v4(),
                ..bound.clone()
            },
        ),
        (
            "invocation",
            Binding {
                invocation_id: Uuid::new_v4(),
                ..bound.clone()
            },
        ),
        (
            "policy",
            Binding {
                policy_decision_digest: [0x22; 32],
                ..bound.clone()
            },
        ),
        (
            "session",
            Binding {
                session_id: Uuid::new_v4(),
                ..bound.clone()
            },
        ),
        (
            "generation",
            Binding {
                fixture_generation: 2,
                ..bound.clone()
            },
        ),
    ];

    for (field, changed) in variants {
        let mut store = ApprovalStore::new();
        let now = Instant::now();
        let challenge = Uuid::new_v4();
        store.begin(challenge, bound.clone(), now);
        let approval = store.finish(challenge, &bound, true, now).unwrap();

        assert_eq!(
            store.verify(&approval, &changed).err(),
            Some(ApprovalError::BindingChanged),
            "a changed {field} still verified"
        );
        // And the digests differ, so the change is detectable offline too.
        assert_ne!(
            bound.digest(),
            changed.digest(),
            "{field} is not in the digest"
        );
    }
}

/// Two bindings that differ only in which UUID is which must not hash the
/// same. A digest built by concatenating fields loosely would collide here.
#[test]
fn swapping_two_fields_changes_the_digest() {
    let bound = binding(Uuid::new_v4());
    let swapped = Binding {
        intent_id: bound.invocation_id,
        invocation_id: bound.intent_id,
        ..bound.clone()
    };
    assert_ne!(bound.digest(), swapped.digest());
}

#[test]
fn an_approval_challenge_expires_at_three_hundred_seconds() {
    let mut store = ApprovalStore::new();
    let start = Instant::now();
    let challenge = Uuid::new_v4();
    let bound = binding(Uuid::new_v4());

    store.begin(challenge, bound.clone(), start);
    assert_eq!(
        store
            .finish(challenge, &bound, true, start + APPROVAL_LIFETIME)
            .err(),
        Some(ApprovalError::Expired)
    );
}

/// A rejected finish burns the challenge, so a caller cannot retry with
/// different arguments until one is accepted.
#[test]
fn a_rejected_finish_burns_the_challenge() {
    let mut store = ApprovalStore::new();
    let now = Instant::now();
    let challenge = Uuid::new_v4();
    let bound = binding(Uuid::new_v4());

    store.begin(challenge, bound.clone(), now);
    assert!(store.finish(challenge, &bound, false, now).is_err());
    assert_eq!(
        store.finish(challenge, &bound, true, now).err(),
        Some(ApprovalError::UnknownChallenge),
        "a burnt challenge was usable again"
    );
}

/// Only one finish can win a challenge — the second finds nothing to finish.
#[test]
fn a_challenge_has_exactly_one_winner() {
    let mut store = ApprovalStore::new();
    let now = Instant::now();
    let challenge = Uuid::new_v4();
    let bound = binding(Uuid::new_v4());

    store.begin(challenge, bound.clone(), now);
    assert!(store.finish(challenge, &bound, true, now).is_ok());
    assert_eq!(
        store.finish(challenge, &bound, true, now).err(),
        Some(ApprovalError::UnknownChallenge)
    );
}

/// One use means one. The record is gone after consumption, so the same
/// approval cannot authorise a second invocation.
#[test]
fn an_approval_can_be_consumed_once() {
    let mut store = ApprovalStore::new();
    let now = Instant::now();
    let challenge = Uuid::new_v4();
    let bound = binding(Uuid::new_v4());

    store.begin(challenge, bound.clone(), now);
    let approval = store.finish(challenge, &bound, true, now).unwrap();

    assert!(store.consume(&approval, &bound).is_ok());
    assert_eq!(
        store.consume(&approval, &bound).err(),
        Some(ApprovalError::UnknownChallenge),
        "an approval authorised a second invocation"
    );
}

/// An approval a person walked away from cannot be spent by whatever is left
/// holding it.
#[test]
fn logout_invalidates_an_unconsumed_approval() {
    let mut store = ApprovalStore::new();
    let now = Instant::now();
    let session = Uuid::new_v4();
    let challenge = Uuid::new_v4();
    let bound = binding(session);

    store.begin(challenge, bound.clone(), now);
    let approval = store.finish(challenge, &bound, true, now).unwrap();
    assert_eq!(store.granted_count(), 1);

    store.revoke_session(session);

    assert_eq!(store.granted_count(), 0);
    assert!(
        store.verify(&approval, &bound).is_err(),
        "an approval survived its session's logout"
    );
}

/// The one that matters most. An approval whose digest verifies perfectly —
/// anyone can recompute it — is still refused by a store that has restarted,
/// because the thing it was granted against is gone.
///
/// A design where offline verification were sufficient would mean an approval
/// captured from memory or a log outlives every revocation anyone can perform.
#[test]
fn a_restart_invalidates_an_approval_whose_digest_still_verifies() {
    let mut store = ApprovalStore::new();
    let now = Instant::now();
    let challenge = Uuid::new_v4();
    let bound = binding(Uuid::new_v4());

    store.begin(challenge, bound.clone(), now);
    let approval = store.finish(challenge, &bound, true, now).unwrap();
    assert!(store.verify(&approval, &bound).is_ok());

    // The digest is unchanged and still recomputes — that part is offline.
    let digest_before = bound.digest();
    drop(store);
    let restarted = ApprovalStore::new();
    assert_eq!(
        bound.digest(),
        digest_before,
        "the digest is not the thing that changed"
    );

    assert_eq!(
        restarted.verify(&approval, &bound).err(),
        Some(ApprovalError::StaleEpoch),
        "an approval survived the restart of the store that granted it"
    );
}

/// Two live stores do not honour each other's approvals either: the epoch is
/// per instance, not per process.
#[test]
fn one_store_does_not_honour_anothers_approval() {
    let mut first = ApprovalStore::new();
    let second = ApprovalStore::new();
    let now = Instant::now();
    let challenge = Uuid::new_v4();
    let bound = binding(Uuid::new_v4());

    first.begin(challenge, bound.clone(), now);
    let approval = first.finish(challenge, &bound, true, now).unwrap();

    assert_eq!(
        second.verify(&approval, &bound).err(),
        Some(ApprovalError::StaleEpoch)
    );
}

/// The approval is opaque: a holder can hand it back and cannot read what it
/// authorises or build one that authorises something else.
#[test]
fn the_approval_is_opaque_and_never_prints_itself() {
    let mut store = ApprovalStore::new();
    let now = Instant::now();
    let challenge = Uuid::new_v4();
    let bound = binding(Uuid::new_v4());
    store.begin(challenge, bound.clone(), now);
    let approval = store.finish(challenge, &bound, true, now).unwrap();

    assert_eq!(
        format!("{approval:?}"),
        "OwnerApprovedInvocation(<redacted>)"
    );

    // Structural: no public fields and no getters, so the only thing a caller
    // can do with one is give it back. Comments are stripped, so a doc
    // comment naming a field cannot fail this.
    let source = include_str!("../src/approval.rs");
    let code: String = source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        code.contains("pub struct OwnerApprovedInvocation {\n    id: Uuid,\n    digest: [u8; 32],\n    epoch: Uuid,\n}"),
        "the approval's fields must stay private"
    );
    // The precise claim: the type has no inherent impl block at all, so there
    // is nowhere for an accessor to live. Scanning the whole file for method
    // names would instead catch `Binding::digest`, which is public on purpose
    // — anyone recomputing it is what makes tampering detectable.
    assert!(
        !code.contains("impl OwnerApprovedInvocation {"),
        "the approval gained an inherent impl; the only thing a holder may do with one is give it back"
    );
    assert!(
        code.contains("impl std::fmt::Debug for OwnerApprovedInvocation"),
        "the value-free Debug must stay hand-written"
    );
}

/// An approval expires with its ceremony, so a stale one is refused even
/// before the store is consulted about it.
#[test]
fn an_approval_challenge_just_inside_the_window_still_finishes() {
    let mut store = ApprovalStore::new();
    let start = Instant::now();
    let challenge = Uuid::new_v4();
    let bound = binding(Uuid::new_v4());

    store.begin(challenge, bound.clone(), start);
    assert!(store
        .finish(
            challenge,
            &bound,
            true,
            start + APPROVAL_LIFETIME - Duration::from_secs(1)
        )
        .is_ok());
}

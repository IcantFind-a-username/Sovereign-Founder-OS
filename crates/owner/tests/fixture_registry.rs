//! The registry, and the ceremonies that write to it.
//!
//! The first test states the fixture's honest limit rather than hiding it:
//! whoever completes the first registration wins an empty registry, and
//! nothing here can tell the founder from anything else running as the
//! founder. Every other test is about the properties that hold *after* that
//! point — the ones a real design would also need.

#![cfg(feature = "owner-effect-fixture")]

use sovereign_owner::bootstrap::Qualification;
use sovereign_owner::config::CeremonyConfig;
use sovereign_owner::registry::{CeremonyError, Registry};
use std::time::{Duration, Instant};
use uuid::Uuid;

fn registry() -> Registry {
    Registry::new(CeremonyConfig::frozen(), Qualification::ProtocolFixtureOnly)
}

/// The limit, asserted rather than described. A caller that is not the
/// founder — and the fixture has no way to know it is not — completes the
/// first registration and wins, and the outcome says so in its own words.
#[test]
fn a_hostile_native_caller_can_win_an_empty_registry() {
    let mut registry = registry();
    let now = Instant::now();
    let ceremony = Uuid::new_v4();
    let handle = Uuid::new_v4();

    registry.begin_registration(now, ceremony).unwrap();
    let (_stored, bootstrap) = registry
        .finish_registration(now, ceremony, b"credential".to_vec(), handle, true)
        .expect("nothing distinguishes this caller from the founder");

    assert_eq!(bootstrap.enrolment_winner(), handle);
    let summary = bootstrap.honest_summary();
    assert!(
        summary.contains("not owner admission"),
        "the outcome must say what it is not: {summary}"
    );
}

/// One credential, ever. The refusal is at both ends, so a second party
/// cannot even obtain a challenge to work against.
#[test]
fn a_second_registration_is_refused_at_both_ends() {
    let mut registry = registry();
    let now = Instant::now();
    let first = Uuid::new_v4();
    registry.begin_registration(now, first).unwrap();
    registry
        .finish_registration(now, first, b"first".to_vec(), Uuid::new_v4(), true)
        .unwrap();

    assert_eq!(
        registry.begin_registration(now, Uuid::new_v4()).err(),
        Some(CeremonyError::AlreadyRegistered),
        "a second registration could be started"
    );
}

/// A ceremony is consumed by the attempt that finishes it, whether or not
/// that attempt succeeded. Otherwise a challenge can be retried until it
/// works, which is the same as having no challenge.
#[test]
fn a_failed_finish_still_burns_its_ceremony() {
    let mut registry = registry();
    let now = Instant::now();
    let ceremony = Uuid::new_v4();
    registry.begin_registration(now, ceremony).unwrap();

    // Fails for want of user verification.
    assert_eq!(
        registry
            .finish_registration(now, ceremony, b"c".to_vec(), Uuid::new_v4(), false)
            .err(),
        Some(CeremonyError::UserVerificationMissing)
    );
    // And the ceremony is gone: the second attempt has nothing to finish.
    assert_eq!(
        registry
            .finish_registration(now, ceremony, b"c".to_vec(), Uuid::new_v4(), true)
            .err(),
        Some(CeremonyError::UnknownCeremony),
        "a failed attempt left its challenge available for a retry"
    );
    assert!(
        registry.is_empty(),
        "a refused registration stored something"
    );
}

#[test]
fn a_ceremony_expires_at_three_hundred_seconds() {
    let mut registry = registry();
    let start = Instant::now();
    let ceremony = Uuid::new_v4();
    registry.begin_registration(start, ceremony).unwrap();

    let past = start + Duration::from_secs(300);
    assert_eq!(
        registry
            .finish_registration(past, ceremony, b"c".to_vec(), Uuid::new_v4(), true)
            .err(),
        Some(CeremonyError::CeremonyExpired)
    );
    assert!(registry.is_empty());
}

#[test]
fn a_ceremony_just_inside_the_window_still_completes() {
    let mut registry = registry();
    let start = Instant::now();
    let ceremony = Uuid::new_v4();
    registry.begin_registration(start, ceremony).unwrap();

    let inside = start + Duration::from_secs(299);
    assert!(registry
        .finish_registration(inside, ceremony, b"c".to_vec(), Uuid::new_v4(), true)
        .is_ok());
}

/// User verification is required, and the flag is passed in rather than
/// assumed: a caller that could omit the flag could omit the verification.
#[test]
fn registration_and_login_both_require_user_verification() {
    let mut registry = registry();
    let now = Instant::now();

    let first = Uuid::new_v4();
    registry.begin_registration(now, first).unwrap();
    assert_eq!(
        registry
            .finish_registration(now, first, b"c".to_vec(), Uuid::new_v4(), false)
            .err(),
        Some(CeremonyError::UserVerificationMissing)
    );

    let handle = Uuid::new_v4();
    let second = Uuid::new_v4();
    registry.begin_registration(now, second).unwrap();
    registry
        .finish_registration(now, second, b"c".to_vec(), handle, true)
        .unwrap();

    let login = Uuid::new_v4();
    registry.begin_login(now, login).unwrap();
    assert_eq!(
        registry.finish_login(now, login, b"c", handle, false).err(),
        Some(CeremonyError::UserVerificationMissing)
    );
}

/// A login offers exactly the stored credential — not a filter, not a hint.
/// A caller that could widen this could steer the browser at a credential of
/// its own and present the result as the owner's.
#[test]
fn login_offers_exactly_the_stored_credential() {
    let mut registry = registry();
    let now = Instant::now();
    let ceremony = Uuid::new_v4();
    registry.begin_registration(now, ceremony).unwrap();
    registry
        .finish_registration(now, ceremony, b"the-one".to_vec(), Uuid::new_v4(), true)
        .unwrap();

    let allow = registry.begin_login(now, Uuid::new_v4()).unwrap();
    assert_eq!(allow, b"the-one".to_vec());
}

/// The handle the authenticator returns must be the stored one. A different
/// handle is a different credential wearing the right id — which is exactly
/// what the origin preflight observed a second port can create.
#[test]
fn a_returned_handle_that_is_not_the_stored_one_is_refused() {
    let mut registry = registry();
    let now = Instant::now();
    let handle = Uuid::new_v4();
    let ceremony = Uuid::new_v4();
    registry.begin_registration(now, ceremony).unwrap();
    registry
        .finish_registration(now, ceremony, b"c".to_vec(), handle, true)
        .unwrap();

    let login = Uuid::new_v4();
    registry.begin_login(now, login).unwrap();
    assert_eq!(
        registry
            .finish_login(now, login, b"c", Uuid::new_v4(), true)
            .err(),
        Some(CeremonyError::HandleMismatch),
        "a replacement credential's handle was accepted"
    );
}

#[test]
fn a_credential_that_is_not_the_stored_one_is_refused() {
    let mut registry = registry();
    let now = Instant::now();
    let handle = Uuid::new_v4();
    let ceremony = Uuid::new_v4();
    registry.begin_registration(now, ceremony).unwrap();
    registry
        .finish_registration(now, ceremony, b"stored".to_vec(), handle, true)
        .unwrap();

    let login = Uuid::new_v4();
    registry.begin_login(now, login).unwrap();
    assert_eq!(
        registry
            .finish_login(now, login, b"another", handle, true)
            .err(),
        Some(CeremonyError::CredentialMismatch)
    );
}

/// A registration ceremony cannot be finished as a login, or the reverse. The
/// purposes are separate challenges, and a caller that could cross them could
/// use one to complete the other.
#[test]
fn a_ceremony_cannot_be_finished_for_the_other_purpose() {
    let mut registry = registry();
    let now = Instant::now();
    let ceremony = Uuid::new_v4();
    registry.begin_registration(now, ceremony).unwrap();

    assert_eq!(
        registry
            .finish_login(now, ceremony, b"c", Uuid::new_v4(), true)
            .err(),
        Some(CeremonyError::UnknownCeremony),
        "a registration challenge completed a login"
    );
}

/// Logging in before anything is registered has nothing to log in as.
#[test]
fn login_before_registration_is_refused() {
    let mut registry = registry();
    assert_eq!(
        registry.begin_login(Instant::now(), Uuid::new_v4()).err(),
        Some(CeremonyError::NotRegistered)
    );
}

/// Losing the one credential is unrecoverable, by design: a reset would be a
/// second way to become the winner. The absence is the property, so it is
/// asserted as one.
#[test]
fn one_credential_loss_has_no_reset_path() {
    let source = include_str!("../src/registry.rs");
    for escape in [
        "pub fn reset",
        "pub fn clear",
        "pub fn remove_credential",
        "pub fn forget",
    ] {
        assert!(
            !source.contains(escape),
            "the registry exposes {escape:?}, which is a second way to win"
        );
    }
}

/// The stored handle is random and carries nothing about a person.
#[test]
fn the_user_handle_is_random_and_not_personal() {
    let mut registry = registry();
    let now = Instant::now();
    let handle = Uuid::new_v4();
    let ceremony = Uuid::new_v4();
    registry.begin_registration(now, ceremony).unwrap();
    let (stored, _) = registry
        .finish_registration(now, ceremony, b"c".to_vec(), handle, true)
        .unwrap();

    assert_eq!(stored.user_handle, handle);
    assert_eq!(handle.get_version_num(), 4, "the handle must be random");
}

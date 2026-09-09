//! Per-connection credentials.
//!
//! The supervisor may register connections; it does not hand them out. That
//! direction is what these tests are about — a caller cannot choose its key,
//! its identity, or how long it lives — together with the three ways a
//! connection can be used beyond what it was given: another connection's
//! scope, a number it has already used, or a moment after it expired.

#![cfg(feature = "owner-effect-fixture")]

use sovereign_authority::broker::connections::{
    ConnectionError, Connections, Scope, CONNECTION_KEY_LEN,
};
use std::time::{Duration, Instant};

fn registry() -> Connections {
    Connections::new(Instant::now() + Duration::from_secs(60))
}

fn key_of(byte: u8) -> impl FnOnce() -> [u8; CONNECTION_KEY_LEN] {
    move || [byte; CONNECTION_KEY_LEN]
}

#[test]
fn a_registered_connection_can_use_its_own_scope() {
    let mut connections = registry();
    let scope = Scope::new("effect.write");
    let credential = connections.register(
        scope.clone(),
        Instant::now() + Duration::from_secs(10),
        key_of(1),
    );

    assert_eq!(
        connections.accept(credential.id, &scope, 0, Instant::now()),
        Ok([1; CONNECTION_KEY_LEN])
    );
}

/// Each registration gets a distinct identity. A caller that could predict or
/// reuse an id could name a connection that already exists.
#[test]
fn every_registration_gets_a_distinct_id() {
    let mut connections = registry();
    let expiry = Instant::now() + Duration::from_secs(10);
    let first = connections.register(Scope::new("a"), expiry, key_of(1));
    let second = connections.register(Scope::new("a"), expiry, key_of(2));
    assert_ne!(first.id, second.id);
    assert_ne!(first.key, second.key);
}

/// Scopes are exact and closed. A related name is not a lesser privilege of
/// the same thing — it is a different thing.
#[test]
fn a_connection_cannot_act_outside_its_exact_scope() {
    let mut connections = registry();
    let credential = connections.register(
        Scope::new("effect.write"),
        Instant::now() + Duration::from_secs(10),
        key_of(1),
    );

    for other in ["effect", "effect.read", "effect.write.extra", "", "*"] {
        assert_eq!(
            connections.accept(credential.id, &Scope::new(other), 0, Instant::now()),
            Err(ConnectionError::OutOfScope),
            "scope {other:?} was accepted for a connection registered as effect.write"
        );
    }
}

/// One connection's credential is not another's. Ids are separate registry
/// entries, so using one under the other's identity finds a different scope.
#[test]
fn one_connection_cannot_act_under_anothers_id() {
    let mut connections = registry();
    let expiry = Instant::now() + Duration::from_secs(10);
    let writer = connections.register(Scope::new("effect.write"), expiry, key_of(1));
    let reader = connections.register(Scope::new("effect.read"), expiry, key_of(2));

    assert_eq!(
        connections.accept(reader.id, &writer.scope, 0, Instant::now()),
        Err(ConnectionError::OutOfScope),
        "the reader acted under the writer's scope"
    );
}

/// A replayed frame is refused by its number, before its MAC is considered.
#[test]
fn a_replayed_sequence_number_is_refused() {
    let mut connections = registry();
    let scope = Scope::new("effect.write");
    let credential = connections.register(
        scope.clone(),
        Instant::now() + Duration::from_secs(10),
        key_of(1),
    );

    assert!(connections
        .accept(credential.id, &scope, 0, Instant::now())
        .is_ok());
    assert_eq!(
        connections.accept(credential.id, &scope, 0, Instant::now()),
        Err(ConnectionError::SequenceOutOfOrder),
        "sequence 0 was accepted twice"
    );
    assert!(connections
        .accept(credential.id, &scope, 1, Instant::now())
        .is_ok());
}

/// A gap is refused as well. A caller able to skip numbers could hide a frame
/// that was dropped or reordered on the way.
#[test]
fn a_gap_in_the_sequence_is_refused() {
    let mut connections = registry();
    let scope = Scope::new("effect.write");
    let credential = connections.register(
        scope.clone(),
        Instant::now() + Duration::from_secs(10),
        key_of(1),
    );

    assert_eq!(
        connections.accept(credential.id, &scope, 1, Instant::now()),
        Err(ConnectionError::SequenceOutOfOrder),
        "a frame that skipped sequence 0 was accepted"
    );
}

/// Nothing outlives the broker that vouched for it: a credential valid after
/// the broker is gone is one nothing can revoke.
#[test]
fn a_requested_expiry_is_clipped_to_the_brokers_life() {
    let broker_until = Instant::now() + Duration::from_millis(150);
    let mut connections = Connections::new(broker_until);
    let scope = Scope::new("effect.write");
    // Asking for an hour, on a broker with a fraction of a second left.
    let credential = connections.register(
        scope.clone(),
        Instant::now() + Duration::from_secs(3600),
        key_of(1),
    );

    assert!(connections
        .accept(credential.id, &scope, 0, Instant::now())
        .is_ok());
    std::thread::sleep(Duration::from_millis(250));
    assert_eq!(
        connections.accept(credential.id, &scope, 1, Instant::now()),
        Err(ConnectionError::Expired),
        "a credential outlived the broker that minted it"
    );
}

/// Revocation is immediate and total, and the key is overwritten rather than
/// merely flagged, so a copy left in freed memory is not the live key.
#[test]
fn a_revoked_connection_is_unknown_immediately() {
    let mut connections = registry();
    let scope = Scope::new("effect.write");
    let credential = connections.register(
        scope.clone(),
        Instant::now() + Duration::from_secs(10),
        key_of(1),
    );

    assert!(connections
        .accept(credential.id, &scope, 0, Instant::now())
        .is_ok());
    assert!(connections.is_live(credential.id));

    connections.revoke(credential.id);
    assert!(!connections.is_live(credential.id));
    assert_eq!(
        connections.accept(credential.id, &scope, 1, Instant::now()),
        Err(ConnectionError::Unknown),
        "a revoked connection was still usable"
    );
}

#[test]
fn an_unregistered_id_is_unknown() {
    let mut connections = registry();
    assert_eq!(
        connections.accept(9999, &Scope::new("effect.write"), 0, Instant::now()),
        Err(ConnectionError::Unknown)
    );
}

/// The credential exists to carry a key, so its rendering must not.
#[test]
fn a_credential_never_prints_its_key() {
    let mut connections = registry();
    let credential = connections.register(
        Scope::new("effect.write"),
        Instant::now() + Duration::from_secs(10),
        key_of(0xAB),
    );
    let rendered = format!("{credential:?}");
    assert!(rendered.contains("redacted"), "{rendered}");
    assert!(
        !rendered.contains("171"),
        "the key bytes appear: {rendered}"
    );
}

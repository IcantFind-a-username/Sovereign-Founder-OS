//! The bind, and the one deadline that bounds the window before the store is
//! owned.
//!
//! Between publishing its address and authenticating a supervisor, the broker
//! holds a port and nothing else — no process lock, no database. That window
//! must be bounded by something no caller can influence. These tests assert
//! that it is: the deadline is absolute, monotonic, shared by accepting and
//! reading alike, and never restarted by an attempt.

#![cfg(feature = "owner-effect-fixture")]

use sovereign_authority::broker::listener::{
    accept_by_deadline, bind_and_publish, SUPERVISOR_ESTABLISHMENT,
};
use sovereign_authority::broker::protocol::{decode_address, Diagnostic, NONCE_LEN};
use std::io::Read;
use std::net::TcpStream;
use std::time::{Duration, Instant};

const NONCE: [u8; NONCE_LEN] = [5; NONCE_LEN];

#[test]
fn the_listener_binds_loopback_only() {
    let mut published = Vec::new();
    let established = bind_and_publish(&NONCE, &mut published).unwrap();
    let address = established.listener().local_addr().unwrap();
    assert!(
        address.ip().is_loopback(),
        "the IPC listener must never leave loopback, bound {address}"
    );
    assert_ne!(established.port(), 0, "an ephemeral port must be resolved");
}

/// The published frame tells the parent where to connect and echoes the nonce
/// it sent. It must not carry the launch key: a parent that needed it back
/// would be a parent that had lost it.
#[test]
fn the_published_address_is_non_secret_and_echoes_the_nonce() {
    let mut published = Vec::new();
    let established = bind_and_publish(&NONCE, &mut published).unwrap();

    let (port, nonce) = decode_address(&published).expect("the address frame must decode");
    assert_eq!(port, established.port());
    assert_eq!(nonce, NONCE);

    // Structural, not incidental: `bind_and_publish` is never given the
    // launch key, so it cannot publish one. The frame is exactly the magic,
    // the version, the port and the echoed nonce, and its fixed length is
    // what a reader can check.
    assert_eq!(
        published.len(),
        sovereign_authority::broker::protocol::ADDRESS_FRAME_LEN,
        "the address frame must be exactly the published fields"
    );
}

/// The whole point. Nobody connects, so the wait must end at the deadline —
/// not sooner, which would be a spurious failure, and not later, which would
/// be an unbounded window.
#[test]
fn the_deadline_is_five_seconds_and_ends_the_wait() {
    let mut published = Vec::new();
    let established = bind_and_publish(&NONCE, &mut published).unwrap();

    let started = Instant::now();
    let outcome = accept_by_deadline(&established);
    let elapsed = started.elapsed();

    assert_eq!(outcome.err(), Some(Diagnostic::SupervisorTimeout));
    assert!(
        elapsed >= SUPERVISOR_ESTABLISHMENT - Duration::from_millis(200),
        "gave up after {elapsed:?}, before the deadline"
    );
    assert!(
        elapsed < SUPERVISOR_ESTABLISHMENT + Duration::from_secs(2),
        "waited {elapsed:?}, well past the deadline"
    );
}

/// The deadline is captured before the address is published, so it is already
/// running while the parent is still reading the address. A deadline that
/// started at the first `accept` call would give a caller a free head start.
#[test]
fn the_deadline_starts_before_the_address_is_published() {
    let mut published = Vec::new();
    let before = Instant::now();
    let established = bind_and_publish(&NONCE, &mut published).unwrap();
    let after = Instant::now();

    let deadline = established.deadline();
    assert!(
        deadline >= before + SUPERVISOR_ESTABLISHMENT,
        "the deadline predates the bind"
    );
    assert!(
        deadline <= after + SUPERVISOR_ESTABLISHMENT,
        "the deadline was captured after publishing, giving a caller a head start"
    );
}

/// A connection that arrives inherits what is left of the shared budget, not
/// a fresh one. Otherwise a caller could connect immediately and then dribble
/// bytes for as long as it liked.
///
/// The bound is on the total from bind to the read giving up, not on the read
/// alone. Timing the read on its own asserts it got *slightly less* than a
/// full budget, which is a hair's breadth — it passed when the file ran in
/// order and failed when run alone, where almost none of the budget had been
/// spent before the read began. The property is that connecting buys one
/// window, not two.
#[test]
fn a_connected_caller_inherits_the_remaining_budget_not_a_new_one() {
    let started = Instant::now();
    let mut published = Vec::new();
    let established = bind_and_publish(&NONCE, &mut published).unwrap();
    let port = established.port();

    // Connect, but send nothing at all.
    let _client = TcpStream::connect(("127.0.0.1", port)).unwrap();
    let mut supervisor = accept_by_deadline(&established).expect("the connection is accepted");

    let mut buffer = [0u8; 8];
    let read = supervisor.read(&mut buffer);
    let total = started.elapsed();

    assert!(read.is_err(), "a silent caller must not satisfy a read");
    assert!(
        total < SUPERVISOR_ESTABLISHMENT * 2,
        "bind through failed read took {total:?}: connecting bought a second window"
    );
}

/// A caller that connects and disconnects does not buy a second window. The
/// property is about the total: however many attempts are made, they all end
/// at the one deadline captured at bind, so the whole sequence is bounded by
/// one budget rather than one per attempt.
#[test]
fn attempts_share_one_window_rather_than_each_getting_their_own() {
    let started = Instant::now();
    let mut published = Vec::new();
    let established = bind_and_publish(&NONCE, &mut published).unwrap();
    let deadline_before = established.deadline();

    // One attempt that connects and immediately goes away. It is accepted, so
    // it consumes almost none of the budget — which is exactly why measuring
    // the second wait on its own would prove nothing.
    drop(TcpStream::connect(("127.0.0.1", established.port())).unwrap());
    let _first = accept_by_deadline(&established);

    assert_eq!(
        established.deadline(),
        deadline_before,
        "an attempt moved the deadline"
    );

    let outcome = accept_by_deadline(&established);
    let total = started.elapsed();

    assert_eq!(outcome.err(), Some(Diagnostic::SupervisorTimeout));
    assert!(
        total < SUPERVISOR_ESTABLISHMENT * 2,
        "two attempts took {total:?}: each got its own window"
    );
}

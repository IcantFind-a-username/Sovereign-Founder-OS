//! Kill the broker at each step it claims to take, and see what it left.
//!
//! The ordering — bind, authenticate, lock, open — is the whole argument for
//! why a hostile caller cannot reach the store. Every earlier test asserted
//! it by watching the sequence succeed or fail as a unit, which cannot
//! distinguish "the lock is taken after authentication" from "the lock is
//! taken at some point during a run that also authenticated".
//!
//! A kill lands *between* two steps. If the broker is stopped after the
//! address is published and no lock file exists, then no ordering in which
//! the lock came first is possible. That is what this matrix buys.
//!
//! It also proves the barriers exist. A kill point that was never reached
//! would leave the process running to its own timeout and the test would see
//! the wrong state, so each case waits for the barrier's announcement rather
//! than for a duration.

#![cfg(all(feature = "owner-effect-fixture", feature = "fault-injection"))]

use sovereign_authority::broker::bootstrap::FIXTURE_MARKER;
use sovereign_authority::broker::process_lock::LOCK_FILE;
use sovereign_authority::broker::protocol::{decode_address, encode, ADDRESS_FRAME_LEN};
use sovereign_authority::broker::store::STORE_FILE;
use sovereign_authority::broker::supervisor::encode_hello;
use sovereign_authority::fault_injection::{Barrier, BARRIER_ENV, REACHED_PREFIX};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

const HIDDEN: &str = "__owner-effect-broker";
const KEY: [u8; 32] = [0x5A; 32];
const NONCE: [u8; 16] = [0x3C; 16];

fn fixture_root(parent: &Path) -> PathBuf {
    let root = parent.join("fixture-root");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join(FIXTURE_MARKER), b"synthetic").unwrap();
    root
}

/// Start a broker that will stop at `barrier`, drive the parent's half far
/// enough to reach it, and return the child once it has announced arrival.
///
/// `hello` decides whether the supervisor authenticates, which is what
/// separates the barriers before that step from the ones after it.
fn run_to_barrier(root: &Path, barrier: Barrier, hello: bool) -> Child {
    let mut child = Command::new(env!("CARGO_BIN_EXE_sovereign"))
        .arg(HIDDEN)
        .env(BARRIER_ENV, barrier.name())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn the broker");

    child
        .stdin
        .take()
        .unwrap()
        .write_all(&encode(&KEY, &NONCE, root))
        .unwrap();

    let mut stdout = child.stdout.take().unwrap();
    let mut published = [0u8; ADDRESS_FRAME_LEN];
    stdout.read_exact(&mut published).expect("an address frame");
    let (port, echoed) = decode_address(&published).unwrap();
    assert_eq!(echoed, NONCE);

    // Kept alive for the whole test: dropping it would close the supervisor
    // connection and take the broker down for the wrong reason.
    let supervisor = if hello {
        let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
        stream.write_all(&encode_hello(&KEY, &NONCE)).unwrap();
        stream.flush().unwrap();
        Some(stream)
    } else {
        None
    };

    // Wait for the barrier itself. A kill timed by a sleep would land
    // wherever the machine happened to be.
    let mut reached = false;
    for line in BufReader::new(stdout).lines() {
        let line = line.unwrap();
        if line.contains(REACHED_PREFIX) && line.contains(barrier.name()) {
            reached = true;
            break;
        }
    }
    assert!(
        reached,
        "the broker never announced {}; the barrier is not on this path",
        barrier.name()
    );
    drop(supervisor);
    child
}

fn kill(mut child: Child) {
    let _ = child.kill();
    let _ = child.wait();
}

/// Killed after the address is published and before any supervisor
/// authenticated: nothing may be claimed.
#[test]
fn killed_after_address_before_hello_leaves_no_lock_and_no_store() {
    let dir = tempfile::tempdir().unwrap();
    let root = fixture_root(dir.path());

    let child = run_to_barrier(&root, Barrier::AfterAddressBeforeSupervisorHello, false);
    assert!(
        !root.join(LOCK_FILE).exists(),
        "the lock was taken before any supervisor authenticated"
    );
    assert!(
        !root.join(STORE_FILE).exists(),
        "the store was opened before any supervisor authenticated"
    );
    kill(child);
}

/// Killed after authentication and before the lock: authentication alone
/// claims nothing, so a broker that dies here leaves the root exactly as it
/// found it.
#[test]
fn killed_after_hello_before_lock_leaves_no_lock_and_no_store() {
    let dir = tempfile::tempdir().unwrap();
    let root = fixture_root(dir.path());

    let child = run_to_barrier(&root, Barrier::AfterAuthenticatedHelloBeforeLock, true);
    assert!(
        !root.join(LOCK_FILE).exists(),
        "the lock was taken before the broker got past authentication"
    );
    assert!(!root.join(STORE_FILE).exists());
    kill(child);
}

/// Killed holding the lock and before the store: the lock file exists, and
/// the kernel releases the lock when the holder dies, so the next broker is
/// not blocked by a corpse.
#[test]
fn killed_after_lock_before_redb_leaves_a_releasable_lock_and_no_store() {
    let dir = tempfile::tempdir().unwrap();
    let root = fixture_root(dir.path());

    let child = run_to_barrier(&root, Barrier::AfterLockBeforeRedbOpen, true);
    assert!(
        root.join(LOCK_FILE).is_file(),
        "the broker reported holding the lock without a lock file"
    );
    assert!(
        !root.join(STORE_FILE).exists(),
        "the store was opened before the lock was held"
    );

    // While it lives, the lock is genuinely held.
    assert!(
        matches!(
            sovereign_authority::broker::process_lock::acquire(&root),
            Err(sovereign_authority::broker::process_lock::LockError::BrokerAlreadyRunning)
        ),
        "another process took a lock the broker was holding"
    );

    kill(child);

    // And once it is gone, the lock is free — released by the kernel, not by
    // any cleanup the killed process could have run.
    let mut freed = false;
    for _ in 0..100 {
        if sovereign_authority::broker::process_lock::acquire(&root).is_ok() {
            freed = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    assert!(freed, "a killed broker left a lock nothing can take");
}

/// Killed with the store open: both artefacts exist, and the next broker can
/// still take the root — a store left by a killed writer must be openable,
/// not wedged.
#[test]
fn killed_after_redb_open_leaves_a_store_the_next_broker_can_take() {
    let dir = tempfile::tempdir().unwrap();
    let root = fixture_root(dir.path());

    let child = run_to_barrier(&root, Barrier::AfterRedbOpenBeforeBrokerReady, true);
    assert!(root.join(LOCK_FILE).is_file());
    assert!(
        root.join(STORE_FILE).is_file(),
        "the broker reported opening the store without a store file"
    );
    kill(child);

    let mut taken = false;
    for _ in 0..100 {
        if let Ok(lock) = sovereign_authority::broker::process_lock::acquire(&root) {
            if sovereign_authority::broker::store::OwnedStore::open(&root, &lock).is_ok() {
                taken = true;
                break;
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    assert!(
        taken,
        "a store left by a killed broker could not be reopened"
    );
}

/// Every barrier the broker declares is reachable on some path. A barrier
/// that is declared and never hit is a kill point that silently does not
/// exist, and a matrix built on it would pass by not stopping anywhere.
#[test]
fn every_broker_barrier_is_reachable() {
    let broker_barriers = [
        (Barrier::AfterAddressBeforeSupervisorHello, false),
        (Barrier::AfterAuthenticatedHelloBeforeLock, true),
        (Barrier::AfterLockBeforeRedbOpen, true),
        (Barrier::AfterRedbOpenBeforeBrokerReady, true),
    ];
    // Every barrier in the enum is either the legacy store one or covered
    // above, so adding a new one without a case here fails this.
    assert_eq!(
        Barrier::ALL.len(),
        broker_barriers.len() + 1,
        "a barrier was added without a kill-matrix case"
    );

    for (barrier, hello) in broker_barriers {
        let dir = tempfile::tempdir().unwrap();
        let root = fixture_root(dir.path());
        // `run_to_barrier` asserts the announcement, so reaching this line is
        // the proof.
        let child = run_to_barrier(&root, barrier, hello);
        kill(child);
    }
}

//! The lock that makes "sole writable owner" true rather than intended.
//!
//! The property is exclusivity, and the interesting cases are the two ways an
//! implementation can get it wrong: treating "already locked" as fatal when it
//! is ordinary, and treating "could not tell" as success when it is not.
//! Everything here is about keeping those two apart, and about the lock being
//! real — released by the kernel when the holder dies, rather than a claim
//! written into a file.

#![cfg(feature = "owner-effect-fixture")]

use sovereign_authority::broker::process_lock::{acquire, LockError, LOCK_FILE};

fn root() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

#[test]
fn the_first_broker_takes_the_lock() {
    let dir = root();
    assert!(acquire(dir.path()).is_ok());
    assert!(
        dir.path().join(LOCK_FILE).is_file(),
        "the lock must be a real file, not an in-memory claim"
    );
}

/// A second holder in the same process is refused with the recoverable name.
/// `try_lock` on many platforms is per-file-description, so a second `open`
/// is a genuinely different holder even here.
#[test]
fn a_second_holder_is_told_a_broker_is_already_running() {
    let dir = root();
    let _first = acquire(dir.path()).expect("the first acquire succeeds");
    assert!(
        matches!(acquire(dir.path()), Err(LockError::BrokerAlreadyRunning)),
        "a second holder must get the recoverable outcome, not a generic failure"
    );
}

/// Dropping the lock releases it. The type is the lifetime — there is no
/// `unlock` to forget — so a broker that returns early cannot leave the store
/// claimed.
#[test]
fn dropping_the_lock_releases_it() {
    let dir = root();
    let first = acquire(dir.path()).unwrap();
    drop(first);
    assert!(acquire(dir.path()).is_ok(), "the lock outlived its holder");
}

/// The distinction this module exists for. A root that cannot be opened at
/// all must not be reported as "another broker is running" — that name would
/// send a caller looking for a process that does not exist — and must never
/// be reported as success.
#[test]
fn an_unusable_root_is_unavailable_not_already_running() {
    let dir = root();
    let blocked = sovereign_fault_testing::BlockedPath::block(dir.path().join("store")).unwrap();

    match acquire(blocked.path()) {
        Err(LockError::Unavailable) => {}
        Err(LockError::BrokerAlreadyRunning) => {
            panic!("an unusable root was reported as a running broker")
        }
        Ok(_) => panic!("an unusable root was locked successfully"),
    }
}

/// The one that matters most: exclusivity across real processes, and a lock
/// the kernel releases when the holder is killed rather than exits.
///
/// A PID file cannot do this. It records a claim, and a SIGKILLed process
/// leaves it behind, so the next broker either believes a dead process owns
/// the store or learns to ignore the file — and a claim that can be ignored
/// is not exclusivity.
#[test]
fn the_lock_is_held_across_processes_and_survives_being_killed() {
    let dir = root();
    let path = dir.path().to_path_buf();

    // Spawned directly rather than through `respawn_self`: the worker needs
    // the root as well as the marker, and the helper carries only one.
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "hold_the_lock_worker",
            "--test-threads=1",
            "--ignored",
            "--nocapture",
        ])
        .env("SOVEREIGN_BROKER_LOCK_ROOT_SET", "1")
        .env("SOVEREIGN_BROKER_LOCK_ROOT", &path)
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    // Wait for the worker to say it holds the lock, rather than sleeping.
    let stdout = child.stdout.take().unwrap();
    let mut held = false;
    for line in std::io::BufRead::lines(std::io::BufReader::new(stdout)) {
        if line.unwrap().contains("lock-held") {
            held = true;
            break;
        }
    }
    assert!(held, "the worker never reported holding the lock");

    assert!(
        matches!(acquire(&path), Err(LockError::BrokerAlreadyRunning)),
        "another process held the lock and this one took it anyway"
    );

    // SIGKILL, not a clean exit: the kernel must release the lock.
    child.kill().unwrap();
    child.wait().unwrap();

    let mut freed = false;
    for _ in 0..100 {
        if acquire(&path).is_ok() {
            freed = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    assert!(
        freed,
        "the lock was not released when its holder was killed; it is a stale claim, not a lock"
    );
}

/// Holds the lock and waits to be killed. Bounded, so a parent that regresses
/// and never kills cannot hang the suite.
#[test]
#[ignore = "spawned by the cross-process lock test"]
fn hold_the_lock_worker() {
    if std::env::var("SOVEREIGN_BROKER_LOCK_ROOT_SET").as_deref() != Ok("1") {
        return;
    }
    let Ok(root) = std::env::var("SOVEREIGN_BROKER_LOCK_ROOT") else {
        return;
    };
    let _lock = acquire(std::path::Path::new(&root)).expect("the worker must take the lock");
    use std::io::Write;
    let mut stdout = std::io::stdout();
    let _ = writeln!(stdout, "lock-held");
    let _ = stdout.flush();
    std::thread::sleep(std::time::Duration::from_secs(30));
}

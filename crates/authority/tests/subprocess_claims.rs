//! Authority claims under real processes and one exact crash.
//!
//! The crate's existing contention tests race threads inside one process,
//! which shares a page cache, a file table, and an allocator. Those are the
//! things a durable one-use claim is supposed to be independent of, so racing
//! threads can only ever be a rehearsal. These tests spawn real processes that
//! open the same store and race actual filesystem operations, and one of them
//! kills a process at a named barrier inside `publish_record` rather than at a
//! guessed moment.
//!
//! Shape: each parent test re-enters this same test binary at an `#[ignore]`d
//! worker, guarded by an environment marker so it never runs on its own. The
//! worker prints one framed result line; the parent reads the frames and
//! judges them. Nothing is inferred from an exit status alone.
//!
//! Requires `--features fault-injection`; without it the barrier does not
//! exist and `scripts/run-authority-subprocess-claims.sh` is how it is run.

use sovereign_authority::{AuthorityError, AuthorityStore};
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::Stdio;
use uuid::Uuid;

/// Names the worker to run and the store to run it against.
const ROLE: &str = "SOVEREIGN_SUBPROCESS_CLAIM_ROLE";
const ROOT: &str = "SOVEREIGN_SUBPROCESS_CLAIM_ROOT";
const SUBJECT: &str = "SOVEREIGN_SUBPROCESS_CLAIM_SUBJECT";
/// Every worker prints exactly one of these, so a parent never guesses.
const RESULT: &str = "claim-result: ";

const NOW: i64 = 1_760_000_000;
const LATER: i64 = NOW + 3_600;

// ----------------------------------------------------------------- parents --

fn worker(name: &str, root: &Path, subject: Uuid, barrier: Option<&str>) -> std::process::Child {
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            name,
            "--test-threads=1",
            "--ignored",
            "--nocapture",
        ])
        .env(ROLE, "1")
        .env(ROOT, root)
        .env(SUBJECT, subject.to_string())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    if let Some(barrier) = barrier {
        command.env(sovereign_authority::fault_injection::BARRIER_ENV, barrier);
    }
    command.spawn().expect("spawn worker")
}

/// Read the one framed result a worker prints, or `None` if it produced none
/// — which is what a killed worker does, and is itself an observation.
///
/// The marker is searched for anywhere in a line, not at its start: the test
/// harness writes `test <name> ... ` without a newline before running the
/// test, so the worker's own output lands on the end of that line.
fn framed_result(child: &mut std::process::Child) -> Option<String> {
    let stdout = child.stdout.take().expect("worker stdout");
    for line in BufReader::new(stdout).lines() {
        let line = line.ok()?;
        if let Some(index) = line.find(RESULT) {
            return Some(line[index + RESULT.len()..].trim().to_owned());
        }
    }
    None
}

/// Two real processes consume the same one-use token. Exactly one may win,
/// and the loser must say so specifically, not merely fail.
#[test]
fn real_subprocess_token_claim_has_one_winner() {
    let dir = tempfile::tempdir().unwrap();
    let token = Uuid::new_v4();
    AuthorityStore::open(dir.path()).unwrap();

    let mut a = worker("consume_token_worker", dir.path(), token, None);
    let mut b = worker("consume_token_worker", dir.path(), token, None);
    let first = framed_result(&mut a);
    let second = framed_result(&mut b);
    a.wait().unwrap();
    b.wait().unwrap();

    let mut outcomes = vec![
        first.expect("worker a framed no result"),
        second.expect("worker b framed no result"),
    ];
    outcomes.sort();
    assert_eq!(
        outcomes,
        vec!["already-consumed".to_string(), "won".to_string()],
        "exactly one process must win a one-use token"
    );
}

/// A claim made by a process that has since exited must still bind. This is
/// the property an in-process test cannot check at all: the winner is gone.
#[test]
fn real_subprocess_approval_survives_restart() {
    let dir = tempfile::tempdir().unwrap();
    let approval = Uuid::new_v4();
    AuthorityStore::open(dir.path()).unwrap();

    let mut first = worker("consume_approval_worker", dir.path(), approval, None);
    assert_eq!(framed_result(&mut first).as_deref(), Some("won"));
    first.wait().unwrap();

    // A fresh process, a fresh store handle, nothing shared but the directory.
    let mut second = worker("consume_approval_worker", dir.path(), approval, None);
    assert_eq!(
        framed_result(&mut second).as_deref(),
        Some("already-consumed"),
        "a claim must survive the death of the process that made it"
    );
    second.wait().unwrap();
}

/// Replay and conflict are different answers to different questions — the
/// same key with the same fingerprint, versus with a different one — and a
/// caller that cannot tell them apart cannot retry safely.
#[test]
fn real_subprocess_idempotency_distinguishes_replay_and_conflict() {
    let dir = tempfile::tempdir().unwrap();
    let key = Uuid::new_v4();
    AuthorityStore::open(dir.path()).unwrap();

    let mut bind = worker("bind_idempotency_worker", dir.path(), key, None);
    assert_eq!(framed_result(&mut bind).as_deref(), Some("won"));
    bind.wait().unwrap();

    let mut replay = worker("bind_idempotency_worker", dir.path(), key, None);
    assert_eq!(framed_result(&mut replay).as_deref(), Some("replay"));
    replay.wait().unwrap();

    let mut conflict = worker("bind_idempotency_conflict_worker", dir.path(), key, None);
    assert_eq!(
        framed_result(&mut conflict).as_deref(),
        Some("conflict"),
        "a different fingerprint under the same key is a conflict, not a replay"
    );
    conflict.wait().unwrap();
}

/// A worker killed part-way through a bundle burns some claims and not
/// others. This records what actually happens today rather than asserting a
/// property the store does not yet have: it is a characterisation, and the
/// value is that a later change to that behaviour has to change this test.
#[test]
fn real_subprocess_mixed_claims_record_current_partial_consumption() {
    let dir = tempfile::tempdir().unwrap();
    let subject = Uuid::new_v4();
    AuthorityStore::open(dir.path()).unwrap();

    let mut child = worker("mixed_claims_worker", dir.path(), subject, None);
    assert_eq!(
        framed_result(&mut child).as_deref(),
        Some("token-then-killed")
    );
    child.kill().unwrap();
    child.wait().unwrap();

    // The token was consumed; the approval never was. Recorded, not endorsed.
    let store = AuthorityStore::open(dir.path()).unwrap();
    assert!(
        matches!(
            store.consume_token(subject, NOW, LATER),
            Err(AuthorityError::AlreadyConsumed)
        ),
        "the token claim the worker completed must be durable"
    );
    assert!(
        store.consume_approval(subject, NOW, LATER).is_ok(),
        "the approval it never reached must be untouched — today's partial-consumption behaviour"
    );
}

/// The semantic crash. A process is killed at the exact instant the temp file
/// is written and fsynced and nothing is published. What must be true
/// afterwards is that a reader sees no record at all — not a truncated one,
/// not a half-published one — and that the store still works.
#[test]
fn kill_after_legacy_temp_sync_before_publish_exposes_no_partial_record() {
    let dir = tempfile::tempdir().unwrap();
    let token = Uuid::new_v4();
    AuthorityStore::open(dir.path()).unwrap();

    let mut child = worker(
        "consume_token_worker",
        dir.path(),
        token,
        Some("LegacyAfterTempSyncBeforePublish"),
    );

    // Wait for the barrier itself, not for a duration. The child announces the
    // line the moment it reaches it, so the kill lands at that instant.
    let stdout = child.stdout.take().expect("worker stdout");
    let mut reached = false;
    for line in BufReader::new(stdout).lines() {
        let line = line.unwrap();
        if line.contains(sovereign_authority::fault_injection::REACHED_PREFIX) {
            reached = true;
            break;
        }
    }
    assert!(reached, "the worker never reached the barrier");
    child.kill().unwrap();
    child.wait().unwrap();

    // No record was published, so the claim is still available.
    let store = AuthorityStore::open(dir.path()).unwrap();
    assert!(
        store.consume_token(token, NOW, LATER).is_ok(),
        "a claim killed before publication must not have taken effect"
    );
    // And it took effect exactly once now that it completed.
    assert!(matches!(
        store.consume_token(token, NOW, LATER),
        Err(AuthorityError::AlreadyConsumed)
    ));
}

// ----------------------------------------------------------------- workers --

fn worker_context() -> Option<(std::path::PathBuf, Uuid)> {
    if std::env::var(ROLE).as_deref() != Ok("1") {
        return None;
    }
    let root = std::env::var(ROOT).ok()?;
    let subject = std::env::var(SUBJECT).ok()?.parse().ok()?;
    Some((root.into(), subject))
}

fn frame(outcome: &str) {
    use std::io::Write;
    let mut stdout = std::io::stdout();
    let _ = writeln!(stdout, "{RESULT}{outcome}");
    let _ = stdout.flush();
}

fn classify(result: Result<(), AuthorityError>) -> &'static str {
    match result {
        Ok(()) => "won",
        Err(AuthorityError::AlreadyConsumed) => "already-consumed",
        Err(AuthorityError::IdempotencyReplay) => "replay",
        Err(AuthorityError::IdempotencyConflict) => "conflict",
        Err(_) => "error",
    }
}

#[test]
#[ignore = "spawned as a child; not a standalone test"]
fn consume_token_worker() {
    let Some((root, subject)) = worker_context() else {
        return;
    };
    let store = AuthorityStore::open(&root).unwrap();
    frame(classify(store.consume_token(subject, NOW, LATER)));
}

#[test]
#[ignore = "spawned as a child; not a standalone test"]
fn consume_approval_worker() {
    let Some((root, subject)) = worker_context() else {
        return;
    };
    let store = AuthorityStore::open(&root).unwrap();
    frame(classify(store.consume_approval(subject, NOW, LATER)));
}

#[test]
#[ignore = "spawned as a child; not a standalone test"]
fn bind_idempotency_worker() {
    let Some((root, subject)) = worker_context() else {
        return;
    };
    let store = AuthorityStore::open(&root).unwrap();
    frame(classify(
        store.bind_idempotency(subject, &[7u8; 32], NOW, LATER),
    ));
}

#[test]
#[ignore = "spawned as a child; not a standalone test"]
fn bind_idempotency_conflict_worker() {
    let Some((root, subject)) = worker_context() else {
        return;
    };
    let store = AuthorityStore::open(&root).unwrap();
    // A different fingerprint under the same key.
    frame(classify(
        store.bind_idempotency(subject, &[9u8; 32], NOW, LATER),
    ));
}

/// Consumes a token, announces it, then blocks so the parent can kill it
/// before it reaches the approval. Bounded, so a parent that never kills
/// cannot hang the suite.
#[test]
#[ignore = "spawned as a child; not a standalone test"]
fn mixed_claims_worker() {
    let Some((root, subject)) = worker_context() else {
        return;
    };
    let store = AuthorityStore::open(&root).unwrap();
    store.consume_token(subject, NOW, LATER).unwrap();
    frame("token-then-killed");
    std::thread::sleep(std::time::Duration::from_secs(30));
    let _ = store.consume_approval(subject, NOW, LATER);
}

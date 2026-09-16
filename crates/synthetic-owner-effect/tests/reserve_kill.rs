//! v01-D05: real process kill immediately before and after commit.
//!
//! A second live process is tested only for pre-redb lock denial (D02).
//! Full cross-process validator race remains Target.

#![cfg(feature = "owner-effect-fixture")]

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::Stdio;

use serde::{Deserialize, Serialize};
use sovereign_synthetic_owner_effect::{
    inspect_reservation, reserve_exact_authority, FixtureOwner, IntentState, BARRIER_AFTER_COMMIT,
    BARRIER_BEFORE_COMMIT, KILL_BARRIER_ENV, KILL_REACHED_PREFIX, SYNTHETIC_NODE_INITIAL_USES,
};
use uuid::Uuid;

#[path = "support/proofs.rs"]
mod proofs;
#[path = "support/root.rs"]
mod root;

const ROLE: &str = "SOVEREIGN_FIXTURE_RESERVE_ROLE";
const ROOT: &str = "SOVEREIGN_FIXTURE_RESERVE_ROOT";
const PROBE: &str = "reservation-probe.json";

#[derive(Serialize, Deserialize)]
struct Probe {
    intent_id: String,
    approval_id: String,
    token_id: String,
    idempotency_key: String,
    generation: u64,
    approval_expires_at_unix: i64,
}

fn in_worker() -> bool {
    std::env::var(ROLE).as_deref() == Ok("1")
}

fn worker(name: &str, root: &Path, barrier: &str) -> std::process::Child {
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
        .env(KILL_BARRIER_ENV, barrier)
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    command.spawn().expect("spawn reservation worker")
}

fn wait_for_barrier(child: &mut std::process::Child) {
    let stdout = child.stdout.take().expect("worker stdout");
    let mut reached = false;
    for line in BufReader::new(stdout).lines() {
        let line = line.unwrap();
        if line.contains(KILL_REACHED_PREFIX) {
            reached = true;
            break;
        }
    }
    assert!(reached, "the worker never reached the kill barrier");
}

fn probe_path(root: &Path) -> PathBuf {
    root.join(PROBE)
}

fn inspect_after_reopen(root: &Path) -> (sovereign_synthetic_owner_effect::ReservationView, Probe) {
    let probe: Probe =
        serde_json::from_slice(&std::fs::read(probe_path(root)).expect("probe file")).unwrap();
    let owner = FixtureOwner::boot(root).expect("reopen after kill");
    let store = owner.open_store().unwrap();
    let view = inspect_reservation(
        &store,
        sovereign_synthetic_owner_effect::EffectIntentId::from_uuid(
            Uuid::parse_str(&probe.intent_id).unwrap(),
        ),
        Uuid::parse_str(&probe.approval_id).unwrap(),
        Uuid::parse_str(&probe.token_id).unwrap(),
        Uuid::parse_str(&probe.idempotency_key).unwrap(),
        probe.generation,
    )
    .unwrap();
    (view, probe)
}

#[test]
fn real_process_kill_before_commit_leaves_none_of_the_reservation() {
    if in_worker() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let root = root::marked_root(dir.path());
    let mut child = worker("reserve_kill_worker", &root, BARRIER_BEFORE_COMMIT);
    wait_for_barrier(&mut child);
    child.kill().unwrap();
    child.wait().unwrap();
    let (view, _) = inspect_after_reopen(&root);
    assert!(
        proofs::none_of_the_reservation(&view),
        "kill before commit left a subset: {view:?}"
    );
    assert_eq!(view.intent_state, Some(IntentState::Prepared));
    assert_eq!(view.authority_uses_remaining, SYNTHETIC_NODE_INITIAL_USES);
}

#[test]
fn real_process_kill_after_commit_leaves_all_of_the_reservation() {
    if in_worker() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let root = root::marked_root(dir.path());
    let mut child = worker("reserve_kill_worker", &root, BARRIER_AFTER_COMMIT);
    wait_for_barrier(&mut child);
    child.kill().unwrap();
    child.wait().unwrap();
    let (view, probe) = inspect_after_reopen(&root);
    let intent_id = sovereign_synthetic_owner_effect::EffectIntentId::from_uuid(
        Uuid::parse_str(&probe.intent_id).unwrap(),
    );
    assert!(proofs::all_of_the_reservation(
        &view,
        intent_id,
        probe.approval_expires_at_unix
    ));
}

#[test]
#[ignore = "spawned as a child; not a standalone test"]
fn reserve_kill_worker() {
    if !in_worker() {
        return;
    }
    let root = PathBuf::from(std::env::var(ROOT).expect("worker root"));
    let mut harness = proofs::Harness::boot(&root);
    let issued = harness.issue_for_new_intent();
    let probe = Probe {
        intent_id: issued.intent_id.as_uuid().to_string(),
        approval_id: issued.approval_id.to_string(),
        token_id: issued.token_id.to_string(),
        idempotency_key: issued.idempotency_key.to_string(),
        generation: issued.context.fixture_generation,
        approval_expires_at_unix: issued.approval_expires_at_unix,
    };
    std::fs::write(probe_path(&root), serde_json::to_vec(&probe).unwrap()).unwrap();
    let (capability, approval) = harness.verify(
        &issued.token,
        &issued.signed_approval,
        issued.intent_id,
        issued.context.now_unix,
    );
    let store = harness.owner.open_store().unwrap();
    let _ = reserve_exact_authority(&store, &issued.context, capability, approval);
}

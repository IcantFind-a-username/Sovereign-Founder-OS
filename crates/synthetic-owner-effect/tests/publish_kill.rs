//! v01-D07: real process kill at dispatch crash boundaries.
//!
//! A second live process is tested only for pre-redb lock denial (D02).
//! Full cross-process validator race remains Target.
//! Design Accept ≠ product Current. Not 1C0, Exact Effect, ActiveV2, or RP1.

#![cfg(feature = "owner-effect-fixture")]

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::Stdio;

use serde::{Deserialize, Serialize};
use sovereign_synthetic_owner_effect::{
    inspect_intent_state, publish_once, reconcile_without_writing, reserve_exact_authority,
    FixtureOwner, IntentState, BARRIER_AFTER_DISPATCHING_COMMIT, BARRIER_AFTER_PUBLICATION,
    BARRIER_BEFORE_DISPATCHING_COMMIT, KILL_BARRIER_ENV, KILL_REACHED_PREFIX,
};
use uuid::Uuid;

#[path = "support/proofs.rs"]
mod proofs;
#[path = "support/root.rs"]
mod root;

const ROLE: &str = "SOVEREIGN_FIXTURE_PUBLISH_ROLE";
const ROOT: &str = "SOVEREIGN_FIXTURE_PUBLISH_ROOT";
const PROBE: &str = "publish-probe.json";

#[derive(Serialize, Deserialize)]
struct Probe {
    intent_id: String,
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
    command.spawn().expect("spawn publish worker")
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

fn kill_at(barrier: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let root = root::marked_root(dir.path());
    let mut child = worker("publish_kill_worker", &root, barrier);
    wait_for_barrier(&mut child);
    child.kill().unwrap();
    child.wait().unwrap();
    (dir, root)
}

fn reopen_and_reconcile(
    root: &Path,
) -> (
    sovereign_synthetic_owner_effect::ClosedOutcome,
    Option<IntentState>,
) {
    let probe: Probe =
        serde_json::from_slice(&std::fs::read(probe_path(root)).expect("probe file")).unwrap();
    let intent_id = sovereign_synthetic_owner_effect::EffectIntentId::from_uuid(
        Uuid::parse_str(&probe.intent_id).unwrap(),
    );
    let owner = FixtureOwner::boot(root).expect("reopen after kill");
    let store = owner.open_store().unwrap();
    let outcome =
        reconcile_without_writing(&store, root, intent_id, owner.bridge().signer_epoch()).unwrap();
    let state = inspect_intent_state(&store, intent_id).unwrap();
    (outcome, state)
}

#[test]
fn real_process_kill_before_dispatching_commit_stays_pre_dispatch() {
    if in_worker() {
        return;
    }
    let (_dir, root) = kill_at(BARRIER_BEFORE_DISPATCHING_COMMIT);
    let (outcome, state) = reopen_and_reconcile(&root);
    assert_eq!(
        outcome,
        sovereign_synthetic_owner_effect::ClosedOutcome::FailedBeforeDispatch
    );
    assert_eq!(state, Some(IntentState::FailedBeforeDispatch));
}

#[test]
fn real_process_kill_after_dispatching_before_write_is_indeterminate() {
    if in_worker() {
        return;
    }
    let (_dir, root) = kill_at(BARRIER_AFTER_DISPATCHING_COMMIT);
    let (outcome, state) = reopen_and_reconcile(&root);
    assert_eq!(
        outcome,
        sovereign_synthetic_owner_effect::ClosedOutcome::Indeterminate
    );
    assert_eq!(state, Some(IntentState::Indeterminate));
}

#[test]
fn real_process_kill_after_publication_reopens_succeeded() {
    if in_worker() {
        return;
    }
    let (_dir, root) = kill_at(BARRIER_AFTER_PUBLICATION);
    let (outcome, state) = reopen_and_reconcile(&root);
    assert_eq!(
        outcome,
        sovereign_synthetic_owner_effect::ClosedOutcome::Succeeded
    );
    assert_eq!(state, Some(IntentState::Succeeded));
}

#[test]
#[ignore = "spawned as a child; not a standalone test"]
fn publish_kill_worker() {
    if !in_worker() {
        return;
    }
    let root = PathBuf::from(std::env::var(ROOT).expect("worker root"));
    let mut harness = proofs::Harness::boot(&root);
    let issued = harness.issue_for_new_intent();
    let probe = Probe {
        intent_id: issued.intent_id.as_uuid().to_string(),
    };
    std::fs::write(probe_path(&root), serde_json::to_vec(&probe).unwrap()).unwrap();
    let (capability, approval) = harness.verify(
        &issued.token,
        &issued.signed_approval,
        issued.intent_id,
        issued.context.now_unix,
    );
    let live = proofs::live_context(&issued);
    let store = harness.owner.open_store().unwrap();
    let reserved = reserve_exact_authority(&store, &issued.context, capability, approval).unwrap();
    let _ = publish_once(&store, &root, reserved, &live);
}

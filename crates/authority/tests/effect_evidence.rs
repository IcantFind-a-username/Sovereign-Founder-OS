//! Terminal coordinator commit, then value-free evidence — and nothing else.
//!
//! The projection tests live in `sovereign-audit-ledger` `effect_v1`. This
//! file is the authority remainder: append only after a terminal commit,
//! heal missing evidence after a crash in that window, and never retry
//! the effect. Fixture-only. Not product RP1-05.

#![cfg(feature = "owner-effect-fixture")]

use sovereign_audit_ledger::effect_v1::{EffectOutcome, FIXTURE_SIGNER};
use sovereign_authority::broker::dispatch::{dispatch, recover, DispatchError, DispatchOutcome};
use sovereign_authority::broker::exact_fixture::{EffectIntentId, ProtectedFixturePayload};
use sovereign_authority::broker::process_lock::acquire;
use sovereign_authority::broker::store::OwnedStore;
use sovereign_authority::effect_evidence::{
    commit_terminal, heal_evidence, listed_terminals, load_chain, record_terminal_and_append,
    EvidenceError,
};
use std::path::Path;

fn with_store(body: impl FnOnce(&OwnedStore<'_>, &Path)) {
    let dir = tempfile::tempdir().unwrap();
    let lock = acquire(dir.path()).unwrap();
    let store = OwnedStore::open(dir.path(), &lock).unwrap();
    body(&store, dir.path());
}

fn outbox(root: &Path) -> std::path::PathBuf {
    let path = root.join("outbox");
    std::fs::create_dir_all(&path).unwrap();
    path
}

fn dispatch_one(root: &Path) -> EffectIntentId {
    let id = EffectIntentId::allocate();
    let payload = ProtectedFixturePayload::compose(id);
    assert_eq!(
        dispatch(&outbox(root), &payload),
        Ok(DispatchOutcome::Dispatched)
    );
    id
}

/// The cursor is the allow-list: a random intent id and a closed outcome.
/// Nothing that describes the effect is stored beside them.
#[test]
fn cursor_carries_only_intent_and_closed_outcome() {
    with_store(|store, _| {
        let id = EffectIntentId::allocate();
        commit_terminal(store, id, EffectOutcome::Dispatched).unwrap();

        let listed = listed_terminals(store).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].0, id);
        assert_eq!(listed[0].1, EffectOutcome::Dispatched);

        let source = include_str!("../src/effect_evidence.rs");
        let code: String = source
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(code.contains("terminal-outcomes-v1"));
        assert!(code.contains("effect-evidence-v1"));
        for forbidden in [
            "recipient",
            "content",
            "digest",
            "timestamp",
            "subject",
            "policy",
            "reason",
        ] {
            assert!(
                !code.contains(forbidden),
                "the cursor gained a {forbidden} column"
            );
        }
    });
}

/// Heal writes the projection and does not call dispatch.
#[test]
fn heal_never_retries_a_dispatched_effect() {
    with_store(|store, root| {
        let id = dispatch_one(root);
        commit_terminal(store, id, EffectOutcome::Dispatched).unwrap();
        assert_eq!(load_chain(store).unwrap().records().len(), 0);

        let report = heal_evidence(store).unwrap();
        assert_eq!(report.appended, 1);

        let chain = load_chain(store).unwrap();
        assert_eq!(chain.records().len(), 1);
        assert_eq!(chain.records()[0].outcome, EffectOutcome::Dispatched);
        assert_eq!(chain.records()[0].signer, FIXTURE_SIGNER);
        assert!(chain.verify().is_ok());

        assert_eq!(recover(&outbox(root), id), DispatchOutcome::Dispatched);
        assert_eq!(
            dispatch(&outbox(root), &ProtectedFixturePayload::compose(id)),
            Err(DispatchError::AlreadyDispatched),
            "heal retried a terminal effect"
        );
        let names: Vec<String> = std::fs::read_dir(outbox(root))
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, vec![format!("{}.eml", id.file_stem())]);
    });
}

#[test]
fn heal_is_idempotent_for_the_same_terminal() {
    with_store(|store, _| {
        let id = EffectIntentId::allocate();
        record_terminal_and_append(store, id, EffectOutcome::Refused).unwrap();
        let first = load_chain(store).unwrap().records()[0].hash();

        let again = heal_evidence(store).unwrap();
        assert_eq!(again.appended, 0);
        record_terminal_and_append(store, id, EffectOutcome::Refused).unwrap();

        let chain = load_chain(store).unwrap();
        assert_eq!(chain.records().len(), 1);
        assert_eq!(chain.records()[0].hash(), first);
    });
}

#[test]
fn a_different_terminal_outcome_conflicts() {
    with_store(|store, _| {
        let id = EffectIntentId::allocate();
        commit_terminal(store, id, EffectOutcome::Dispatched).unwrap();
        assert_eq!(
            commit_terminal(store, id, EffectOutcome::Refused),
            Err(EvidenceError::OutcomeConflict)
        );
        assert_eq!(
            listed_terminals(store).unwrap()[0].1,
            EffectOutcome::Dispatched
        );
    });
}

#[test]
fn an_indeterminate_terminal_is_never_relabelled_by_heal() {
    with_store(|store, _| {
        let id = EffectIntentId::allocate();
        commit_terminal(store, id, EffectOutcome::Indeterminate).unwrap();
        heal_evidence(store).unwrap();

        for later in [EffectOutcome::Dispatched, EffectOutcome::Refused] {
            assert_eq!(
                commit_terminal(store, id, later),
                Err(EvidenceError::OutcomeConflict),
                "heal or replay relabelled indeterminate as {later:?}"
            );
        }
        assert_eq!(
            load_chain(store).unwrap().records()[0].outcome,
            EffectOutcome::Indeterminate
        );
    });
}

/// The glue describes; it does not decide. Heal and commit have no path
/// that writes an outbox file or changes a dispatch outcome.
#[test]
fn evidence_never_advances_effect_state() {
    let source = include_str!("../src/effect_evidence.rs");
    let code: String = source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    for mutator in [
        "broker::dispatch::dispatch",
        "fn dispatch(",
        "fn set_outcome",
        "fn retry",
        "fn execute",
    ] {
        assert!(
            !code.contains(mutator),
            "effect_evidence exposes {mutator:?}, so it can change what it records"
        );
    }
    assert!(
        code.contains("pub fn heal_evidence"),
        "stripping comments removed the code as well"
    );
}

#[test]
fn appended_evidence_is_the_value_free_projection() {
    with_store(|store, _| {
        let id = EffectIntentId::allocate();
        record_terminal_and_append(store, id, EffectOutcome::Dispatched).unwrap();
        let chain = load_chain(store).unwrap();
        let record = &chain.records()[0];
        let rendered = format!("{record:?}");
        for field in [
            "intent_id",
            "outcome",
            "sequence",
            "previous_hash",
            "signer",
        ] {
            assert!(rendered.contains(field), "{field} missing from {rendered}");
        }
        assert_eq!(record.signer, FIXTURE_SIGNER);
        assert!(FIXTURE_SIGNER.contains("synthetic") && FIXTURE_SIGNER.contains("fixture"));
    });
}

#[cfg(feature = "fault-injection")]
const EVIDENCE_ROLE: &str = "SOVEREIGN_EFFECT_EVIDENCE_ROLE";
#[cfg(feature = "fault-injection")]
const EVIDENCE_ROOT: &str = "SOVEREIGN_EFFECT_EVIDENCE_ROOT";

#[cfg(feature = "fault-injection")]
fn evidence_worker(root: &Path) -> std::process::Child {
    use sovereign_authority::fault_injection::{Barrier, BARRIER_ENV};
    use std::process::Stdio;

    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            "record_terminal_worker",
            "--test-threads=1",
            "--ignored",
            "--nocapture",
        ])
        .env(EVIDENCE_ROLE, "1")
        .env(EVIDENCE_ROOT, root)
        .env(
            BARRIER_ENV,
            Barrier::AfterTerminalCommitBeforeEvidenceAppend.name(),
        )
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    command.spawn().expect("spawn worker")
}

/// The named remainder: terminal is durable, evidence is not, a kill
/// lands in that window, and reopen heals the projection only.
#[cfg(feature = "fault-injection")]
#[test]
fn crash_after_terminal_before_append_heals_evidence_only() {
    use sovereign_authority::fault_injection::REACHED_PREFIX;
    use std::io::{BufRead, BufReader};

    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    {
        let lock = acquire(root).unwrap();
        let _store = OwnedStore::open(root, &lock).unwrap();
    }

    let mut child = evidence_worker(root);
    let stdout = child.stdout.take().expect("worker stdout");
    let mut reached = false;
    for line in BufReader::new(stdout).lines() {
        let line = line.unwrap();
        if line.contains(REACHED_PREFIX) {
            reached = true;
            break;
        }
    }
    assert!(
        reached,
        "the worker never reached the terminal/evidence barrier"
    );
    child.kill().unwrap();
    child.wait().unwrap();

    let stem = std::fs::read_to_string(root.join("intent.stem")).expect("worker stem");
    let id = EffectIntentId::from_file_stem(stem.trim()).expect("stem");

    let lock = acquire(root).unwrap();
    let store = OwnedStore::open(root, &lock).unwrap();

    assert_eq!(
        listed_terminals(&store).unwrap(),
        vec![(id, EffectOutcome::Dispatched)]
    );
    assert_eq!(
        load_chain(&store).unwrap().records().len(),
        0,
        "evidence was appended before the kill"
    );
    assert_eq!(recover(&outbox(root), id), DispatchOutcome::Dispatched);

    let report = heal_evidence(&store).unwrap();
    assert_eq!(report.appended, 1);
    let chain = load_chain(&store).unwrap();
    assert_eq!(chain.records().len(), 1);
    assert_eq!(chain.records()[0].outcome, EffectOutcome::Dispatched);
    assert!(chain.verify().is_ok());

    assert_eq!(recover(&outbox(root), id), DispatchOutcome::Dispatched);
    assert_eq!(
        dispatch(&outbox(root), &ProtectedFixturePayload::compose(id)),
        Err(DispatchError::AlreadyDispatched),
        "heal retried the effect after a crash"
    );
    let names: Vec<String> = std::fs::read_dir(outbox(root))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, vec![format!("{}.eml", id.file_stem())]);
}

#[cfg(feature = "fault-injection")]
#[test]
#[ignore = "spawned as a child; not a standalone test"]
fn record_terminal_worker() {
    if std::env::var(EVIDENCE_ROLE).as_deref() != Ok("1") {
        return;
    }
    let root = std::path::PathBuf::from(std::env::var(EVIDENCE_ROOT).expect("root"));
    let lock = acquire(&root).unwrap();
    let store = OwnedStore::open(&root, &lock).unwrap();
    let id = dispatch_one(&root);
    std::fs::write(root.join("intent.stem"), id.file_stem()).unwrap();
    record_terminal_and_append(&store, id, EffectOutcome::Dispatched).unwrap();
}

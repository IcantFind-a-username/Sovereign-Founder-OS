//! v01-D06: publish once, reconcile without writing, value-free evidence.
//!
//! Design Accept ≠ product Current. Not 1C0, Exact Effect, ActiveV2, or RP1.

#![cfg(feature = "owner-effect-fixture")]

use redb::{ReadableTable, TableDefinition};
use sovereign_authority::broker::corpus::{contains_canary, BODY_CANARY, RECIPIENT};
use sovereign_authority::broker::store::STORE_FILE;
use sovereign_synthetic_owner_effect::{
    accept_closed_guest_output, canonical_guest_input, expected_closed_exit, inspect_intent_state,
    listed_evidence, publish_once, published_path, reconcile_without_writing,
    reserve_exact_authority, run_core_wasm_module, temp_path, with_publish_failpoint,
    writer_io_observed, ClosedOutcome, FixtureOwner, IntentState, PublishError, PublishFailpoint,
    FIXED_GUEST_WAT,
};

#[path = "support/proofs.rs"]
mod proofs;
#[path = "support/root.rs"]
mod root;

const BYTE_TABLES: &[&str] = &[
    "fixture-reserved-approvals-v1",
    "fixture-reserved-tokens-v1",
    "fixture-reserved-idempotency-v1",
    "fixture-effect-intents-v1",
    "fixture-value-free-evidence-v1",
];

fn dispatch_source() -> &'static str {
    include_str!("../src/dispatch.rs")
}

fn publish_source() -> &'static str {
    include_str!("../src/publish.rs")
}

fn lib_source() -> &'static str {
    include_str!("../src/lib.rs")
}

fn comments_stripped(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn reserve_handle(
    harness: &mut proofs::Harness,
    issued: &proofs::Issued,
) -> sovereign_synthetic_owner_effect::AuthorityReservedEffect {
    let (capability, approval) = harness.verify(
        &issued.token,
        &issued.signed_approval,
        issued.intent_id,
        issued.context.now_unix,
    );
    let store = harness.owner.open_store().unwrap();
    reserve_exact_authority(&store, &issued.context, capability, approval).unwrap()
}

#[test]
fn no_alternate_writer_entry_exists() {
    let lib = comments_stripped(lib_source());
    let dispatch = comments_stripped(dispatch_source());
    let publish = comments_stripped(publish_source());
    for source in [&lib, &dispatch, &publish] {
        for forbidden in [
            "fn write_message",
            "OutboxBroker",
            "ReservationRequest",
            "fn dispatch_raw",
            "fn write_eml",
            "pub fn from_intent_id",
            "broker::dispatch::dispatch",
        ] {
            assert!(
                !source.contains(forbidden),
                "alternate writer entry `{forbidden}` is present"
            );
        }
    }
    assert!(dispatch.contains("reserved: AuthorityReservedEffect"));
    assert!(!dispatch.contains("reserved: &AuthorityReservedEffect"));
    assert_eq!(
        dispatch.matches("pub fn publish_once").count(),
        1,
        "exactly one writer entry"
    );
}

#[test]
fn guest_imports_are_refused() {
    let module = wat::parse_str(
        r#"(module
            (import "wasi_snapshot_preview1" "random_get" (func (param i32 i32) (result i32)))
            (memory (export "memory") 1)
            (func (export "sovereign_run") (param i32 i32) (result i32) i32.const 0))"#,
    )
    .unwrap();
    let error = run_core_wasm_module(&module, b"{\"operation\":\"write_rfc5322\"}")
        .expect_err("imported guest must be refused");
    assert_eq!(error, PublishError::GuestImport);
}

#[test]
fn changed_guest_output_is_rejected() {
    let module = wat::parse_str(
        r#"(module
            (memory (export "memory") 1)
            (func (export "sovereign_run") (param i32 i32) (result i32) i32.const 1))"#,
    )
    .unwrap();
    let input = b"authenticated-canonical-input";
    let exit = run_core_wasm_module(&module, input).expect("import-free guest runs");
    assert_eq!(exit, 1);
    assert_ne!(expected_closed_exit(input), 1);
    assert_eq!(
        accept_closed_guest_output(exit, input),
        Err(PublishError::OutputRejected)
    );
    assert!(FIXED_GUEST_WAT.contains("sovereign_run"));
    assert!(!FIXED_GUEST_WAT.contains("import "));
}

#[test]
fn publish_once_commits_dispatching_then_writes_the_exact_file() {
    let dir = tempfile::tempdir().unwrap();
    let root = root::marked_root(dir.path());
    let mut harness = proofs::Harness::boot(&root);
    let issued = harness.issue_for_new_intent();
    let reserved = reserve_handle(&mut harness, &issued);
    let live = proofs::live_context(&issued);
    let store = harness.owner.open_store().unwrap();
    let outcome = publish_once(&store, &root, reserved, &live).unwrap();
    assert_eq!(outcome, ClosedOutcome::Succeeded);
    assert_eq!(
        inspect_intent_state(&store, issued.intent_id).unwrap(),
        Some(IntentState::Succeeded)
    );
    let published = published_path(&root, issued.intent_id);
    let bytes = std::fs::read(&published).unwrap();
    assert!(
        bytes
            .windows(RECIPIENT.len())
            .any(|w| w == RECIPIENT.as_bytes()),
        "published file is not the sealed message"
    );
    assert!(bytes
        .windows(BODY_CANARY.len())
        .any(|w| w == BODY_CANARY.as_bytes()));
    assert!(!temp_path(&root, issued.intent_id).exists());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&published).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "published file must be owner-only");
    }
}

#[test]
fn filesystem_and_terminal_commit_failpoints_are_named() {
    for stage in PublishFailpoint::ALL {
        let dir = tempfile::tempdir().unwrap();
        let root = root::marked_root(dir.path());
        let mut harness = proofs::Harness::boot(&root);
        let issued = harness.issue_for_new_intent();
        let reserved = reserve_handle(&mut harness, &issued);
        let live = proofs::live_context(&issued);
        let store = harness.owner.open_store().unwrap();
        let result =
            with_publish_failpoint(*stage, || publish_once(&store, &root, reserved, &live));
        match stage {
            PublishFailpoint::BeforeDispatchingCommit => {
                assert_eq!(result, Err(PublishError::Failpoint(*stage)));
                assert_eq!(
                    inspect_intent_state(&store, issued.intent_id).unwrap(),
                    Some(IntentState::AuthorityReserved)
                );
                assert!(!writer_io_observed(&root, issued.intent_id));
            }
            PublishFailpoint::AfterDispatchingCommit => {
                assert_eq!(result, Err(PublishError::Failpoint(*stage)));
                assert_eq!(
                    inspect_intent_state(&store, issued.intent_id).unwrap(),
                    Some(IntentState::Dispatching)
                );
                assert!(!writer_io_observed(&root, issued.intent_id));
            }
            _ => {
                assert_eq!(result, Ok(ClosedOutcome::Indeterminate));
                assert_eq!(
                    inspect_intent_state(&store, issued.intent_id).unwrap(),
                    Some(IntentState::Indeterminate)
                );
            }
        }
    }
}

#[test]
fn crash_reopen_identical_file_is_succeeded() {
    let dir = tempfile::tempdir().unwrap();
    let root = root::marked_root(dir.path());
    let intent_id;
    {
        let mut harness = proofs::Harness::boot(&root);
        let issued = harness.issue_for_new_intent();
        intent_id = issued.intent_id;
        let reserved = reserve_handle(&mut harness, &issued);
        let live = proofs::live_context(&issued);
        let store = harness.owner.open_store().unwrap();
        let error = with_publish_failpoint(PublishFailpoint::AfterDispatchingCommit, || {
            publish_once(&store, &root, reserved, &live)
        });
        assert_eq!(
            error,
            Err(PublishError::Failpoint(
                PublishFailpoint::AfterDispatchingCommit
            ))
        );
        let expected = sovereign_synthetic_owner_effect::expected_publication_bytes(intent_id);
        std::fs::write(published_path(&root, intent_id), expected).unwrap();
    }
    let owner = FixtureOwner::boot(&root).expect("reopen after crash");
    let store = owner.open_store().unwrap();
    let outcome =
        reconcile_without_writing(&store, &root, intent_id, owner.bridge().signer_epoch()).unwrap();
    assert_eq!(outcome, ClosedOutcome::Succeeded);
    assert_eq!(
        inspect_intent_state(&store, intent_id).unwrap(),
        Some(IntentState::Succeeded)
    );
}

#[test]
fn crash_reopen_absent_or_different_is_indeterminate() {
    for different in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let root = root::marked_root(dir.path());
        let intent_id;
        {
            let mut harness = proofs::Harness::boot(&root);
            let issued = harness.issue_for_new_intent();
            intent_id = issued.intent_id;
            let reserved = reserve_handle(&mut harness, &issued);
            let live = proofs::live_context(&issued);
            let store = harness.owner.open_store().unwrap();
            with_publish_failpoint(PublishFailpoint::AfterDispatchingCommit, || {
                publish_once(&store, &root, reserved, &live)
            })
            .expect_err("leave Dispatching");
            if different {
                std::fs::write(published_path(&root, intent_id), b"not-the-sealed-bytes").unwrap();
            }
        }
        let owner = FixtureOwner::boot(&root).expect("reopen after crash");
        let store = owner.open_store().unwrap();
        let outcome =
            reconcile_without_writing(&store, &root, intent_id, owner.bridge().signer_epoch())
                .unwrap();
        assert_eq!(outcome, ClosedOutcome::Indeterminate);
        assert_eq!(
            inspect_intent_state(&store, intent_id).unwrap(),
            Some(IntentState::Indeterminate)
        );
    }
}

#[test]
fn restart_of_old_pre_dispatch_work_fails_without_resign() {
    let dir = tempfile::tempdir().unwrap();
    let root = root::marked_root(dir.path());
    let intent_id;
    {
        let mut harness = proofs::Harness::boot(&root);
        let issued = harness.issue_for_new_intent();
        intent_id = issued.intent_id;
        let _reserved = reserve_handle(&mut harness, &issued);
        assert!(!writer_io_observed(&root, intent_id));
    }
    let owner = FixtureOwner::boot(&root).expect("new signer epoch");
    let store = owner.open_store().unwrap();
    let outcome =
        reconcile_without_writing(&store, &root, intent_id, owner.bridge().signer_epoch()).unwrap();
    assert_eq!(outcome, ClosedOutcome::FailedBeforeDispatch);
    assert!(!writer_io_observed(&root, intent_id));
    assert_eq!(
        inspect_intent_state(&store, intent_id).unwrap(),
        Some(IntentState::FailedBeforeDispatch)
    );
    let dispatch = comments_stripped(dispatch_source());
    assert!(!dispatch.contains("approve_invocation"));
    assert!(!dispatch.contains("sign_cose"));
}

#[test]
fn restart_of_dispatching_only_reaches_succeeded_or_indeterminate() {
    let dispatch = comments_stripped(dispatch_source());
    let start = dispatch
        .find("IntentState::Dispatching =>")
        .expect("Dispatching recovery arm");
    let arm = &dispatch[start..];
    assert!(arm.contains("ClosedOutcome::Succeeded"));
    assert!(arm.contains("ClosedOutcome::Indeterminate"));
    let end = arm.find("fn recheck_live").unwrap_or(arm.len().min(800));
    let recovery = &arm[..end];
    assert!(
        !recovery.contains("FailedBeforeDispatch"),
        "Dispatching recovery must not fail-before-dispatch"
    );
}

#[test]
fn no_retry_or_automatic_resign_after_ambiguity() {
    let dir = tempfile::tempdir().unwrap();
    let root = root::marked_root(dir.path());
    let mut harness = proofs::Harness::boot(&root);
    let issued = harness.issue_for_new_intent();
    let reserved = reserve_handle(&mut harness, &issued);
    let live = proofs::live_context(&issued);
    let store = harness.owner.open_store().unwrap();
    let first = with_publish_failpoint(PublishFailpoint::AfterFirstWrite, || {
        publish_once(&store, &root, reserved, &live)
    });
    assert_eq!(first, Ok(ClosedOutcome::Indeterminate));
    let debris = std::fs::read(temp_path(&root, issued.intent_id)).ok();
    let again =
        reconcile_without_writing(&store, &root, issued.intent_id, issued.context.signer_epoch)
            .unwrap();
    assert_eq!(again, ClosedOutcome::Indeterminate);
    assert_eq!(
        std::fs::read(temp_path(&root, issued.intent_id)).ok(),
        debris,
        "ambiguity must not retry, rewrite, or delete"
    );
    assert!(!published_path(&root, issued.intent_id).exists());
    let publish = comments_stripped(publish_source());
    assert!(!publish.contains("std::fs::rename"));
    assert!(publish.contains("hard_link"));
}

#[test]
fn evidence_is_value_free() {
    let dir = tempfile::tempdir().unwrap();
    let root = root::marked_root(dir.path());
    let mut harness = proofs::Harness::boot(&root);
    let issued = harness.issue_for_new_intent();
    let reserved = reserve_handle(&mut harness, &issued);
    let live = proofs::live_context(&issued);
    let store = harness.owner.open_store().unwrap();
    publish_once(&store, &root, reserved, &live).unwrap();
    let listed = listed_evidence(&store).unwrap();
    assert_eq!(listed.len(), 1);
    let record = &listed[0];
    let json = serde_json::to_value(record).unwrap();
    let object = json.as_object().unwrap();
    for required in [
        "version",
        "type",
        "event_id",
        "intent_id",
        "outcome",
        "previous_event_hash",
        "signer_issuer",
        "signer_key_id",
        "event_hash",
        "signature",
    ] {
        assert!(object.contains_key(required), "missing {required}");
    }
    for forbidden in [
        "recipient",
        "content",
        "digest",
        "path",
        "size",
        "timestamp",
        "subject",
        "body",
        "policy",
        "reason",
    ] {
        assert!(
            !object.contains_key(forbidden),
            "evidence gained a {forbidden} field"
        );
    }
    let rendered = format!("{record:?}{}", serde_json::to_string(record).unwrap());
    assert!(!contains_canary(rendered.as_bytes()));
    assert_eq!(record.outcome, "succeeded");
    assert_eq!(record.intent_id, issued.intent_id.as_uuid().to_string());
}

#[test]
fn table_and_path_aware_canary_allowlist_is_green() {
    let dir = tempfile::tempdir().unwrap();
    let root = root::marked_root(dir.path());
    let mut harness = proofs::Harness::boot(&root);
    let issued = harness.issue_for_new_intent();
    let reserved = reserve_handle(&mut harness, &issued);
    let live = proofs::live_context(&issued);
    let store = harness.owner.open_store().unwrap();
    publish_once(&store, &root, reserved, &live).unwrap();

    let published = published_path(&root, issued.intent_id);
    let eml = std::fs::read(&published).unwrap();
    assert!(contains_canary(&eml), "expected canary missing from .eml");
    assert!(!temp_path(&root, issued.intent_id).exists());

    store
        .read(
            |transaction| -> Result<(), sovereign_synthetic_owner_effect::ReserveError> {
                for name in BYTE_TABLES {
                    let table = TableDefinition::<&[u8], &[u8]>::new(name);
                    let open = match transaction.open_table(table) {
                        Ok(open) => open,
                        Err(redb::TableError::TableDoesNotExist(_)) => continue,
                        Err(_) => panic!("could not open {name}"),
                    };
                    for entry in open.iter().unwrap() {
                        let (key, value) = entry.unwrap();
                        assert!(
                            !contains_canary(key.value()) && !contains_canary(value.value()),
                            "canary leaked into table {name}"
                        );
                    }
                }
                Ok(())
            },
        )
        .unwrap();

    let db = std::fs::read(root.join(STORE_FILE)).unwrap();
    assert!(!contains_canary(&db), "canary leaked into authority.redb");

    for entry in walkdir(&root) {
        if entry == published {
            continue;
        }
        if entry.is_file() {
            let bytes = std::fs::read(&entry).unwrap();
            assert!(
                !contains_canary(&bytes),
                "canary leaked into {}",
                entry.display()
            );
        }
    }
}

fn walkdir(root: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else {
                files.push(path);
            }
        }
    }
    files
}

#[test]
fn canonical_input_never_binds_recipient_or_content() {
    let input = canonical_guest_input(
        sovereign_synthetic_owner_effect::EffectIntentId::allocate(),
        1,
    );
    assert!(!contains_canary(&input));
}

//! RFC 0007 Amendment 1 workspace open-time generation protection.

use super::*;
use sovereign_audit_ledger::{enrolled_freshness_path, ledger_head_path};
use tempfile::tempdir;

#[test]
fn a_paired_workspace_tree_restore_is_refused_when_generation_survives_outside_tree() {
    let dir = tempdir().unwrap();
    let tree = dir.path().join("workspace");
    let enrollment = dir.path().join("enrollment");
    std::fs::create_dir_all(&tree).unwrap();

    let store = Store::open_enrolled(&tree, &enrollment).unwrap();
    store.set_venture("Acme", "consulting").unwrap();
    store
        .add_customer("Dr. Tan", "dr.tan@example.com", "met at expo")
        .unwrap();

    let ledger_path = tree.join("ledger.json");
    let head_path = ledger_head_path(&ledger_path);
    assert!(head_path.is_file());
    assert!(
        enrolled_freshness_path(&enrollment).is_file(),
        "enrollment must live outside the workspace tree"
    );
    let snap_ledger = std::fs::read(&ledger_path).unwrap();
    let snap_head = std::fs::read(&head_path).unwrap();

    store
        .add_customer("Second", "second@example.com", "later")
        .unwrap();

    std::fs::write(&ledger_path, snap_ledger).unwrap();
    std::fs::write(&head_path, snap_head).unwrap();

    match Store::open_enrolled(&tree, &enrollment) {
        Err(err) => {
            let msg = err.to_string();
            assert!(
                msg.contains("generation") || msg.contains("limited recovery"),
                "expected a generation-downgrade refusal, got: {msg}"
            );
        }
        Ok(_) => panic!(
            "paired ledger+anchor restore must be refused when enrolled generation \
             survives outside the workspace tree"
        ),
    }
}

#[test]
fn missing_enrolled_generation_is_limited_recovery_not_a_normal_open() {
    let dir = tempdir().unwrap();
    let tree = dir.path().join("workspace");
    let enrollment = dir.path().join("enrollment");
    std::fs::create_dir_all(&tree).unwrap();

    let store = Store::open_enrolled(&tree, &enrollment).unwrap();
    store.set_venture("Acme", "consulting").unwrap();
    drop(store);

    std::fs::remove_file(enrolled_freshness_path(&enrollment)).unwrap();
    match Store::open_enrolled(&tree, &enrollment) {
        Err(WorkspaceError::LimitedRecovery(detail)) => {
            assert!(
                detail.contains("not a normal workspace open"),
                "limited recovery must be explicit: {detail}"
            );
        }
        Ok(_) => panic!("expected LimitedRecovery, got a normal open"),
        Err(err) => panic!("expected LimitedRecovery, got {err}"),
    }
}

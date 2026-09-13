//! Workspace-path adversarial coverage for RFC 0003 Amendment 1: the
//! consumption bundle and durable revocation must hold when driven through
//! the founder send workflow, not only inside `sovereign-authority`.

use chrono::Utc;
use sovereign_authority::AuthorityStore;
use sovereign_cli::workspace::{ApprovalStatus, DocumentKind, DocumentStatus, Store};
use sovereign_fault_testing::BlockedPath;
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use uuid::Uuid;

fn pending_send_fixture() -> (tempfile::TempDir, Store, Uuid, Uuid) {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    store.set_venture("Acme", "Landing pages").unwrap();
    let workspace = store
        .add_customer("Dr. Tan", "dr.tan@example.com", "")
        .unwrap();
    let customer_id = workspace.customers[0].id;
    let workspace = store
        .create_document(DocumentKind::Invoice, customer_id, Some(250_000), "en")
        .unwrap();
    let document_id = workspace.documents[0].id;
    let workspace = store.request_send(document_id).unwrap();
    let approval_id = workspace.approvals[0].id;
    (dir, store, document_id, approval_id)
}

fn outbox_eml_paths(root: &Path) -> Vec<PathBuf> {
    let outbox = root.join("outbox");
    if !outbox.is_dir() {
        return Vec::new();
    }
    std::fs::read_dir(outbox)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "eml"))
        .collect()
}

fn any_bundle_committed(authority_root: &Path) -> bool {
    let bundles = authority_root.join("bundles");
    if !bundles.is_dir() {
        return false;
    }
    std::fs::read_dir(bundles)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .any(|entry| {
            entry
                .path()
                .extension()
                .is_some_and(|ext| ext == "committed")
        })
}

/// Every bundle interruption point on the real send path must leave either a
/// retriable partial bundle (no outbox effect) or a committed bundle with the
/// effect — never a burned claim without success, and never an outbox file
/// without committed authority.
#[test]
fn a_workspace_bundle_interruption_never_splits_effect_from_authority() {
    for blocked_kind in ["tokens", "approvals", "idempotency", "bundles"] {
        let (dir, store, document_id, approval_id) = pending_send_fixture();
        let authority_root = dir.path().join("authority");
        AuthorityStore::open(&authority_root).unwrap();
        let block_target = authority_root.join(blocked_kind);
        let blocked = BlockedPath::block(block_target).unwrap();

        let first = store.decide(approval_id, true);
        drop(blocked);

        assert!(
            first.is_err(),
            "decide must fail when {blocked_kind} is unavailable"
        );
        assert_eq!(
            store.load().unwrap().approvals[0].status,
            ApprovalStatus::Pending,
            "a failed bundle must not commit workspace state"
        );
        assert!(
            outbox_eml_paths(dir.path()).is_empty(),
            "no outbox effect without a completed send"
        );
        assert!(
            !any_bundle_committed(&authority_root),
            "no committed bundle without a completed send"
        );

        let workspace = store.decide(approval_id, true).expect("retry must succeed");
        assert_eq!(workspace.approvals[0].status, ApprovalStatus::Approved);
        assert_eq!(
            workspace.documents[0].status,
            DocumentStatus::ApprovedPendingDelivery
        );
        assert!(
            any_bundle_committed(&authority_root),
            "a successful send must commit the bundle"
        );
        assert_eq!(
            outbox_eml_paths(dir.path()).len(),
            1,
            "exactly one outbox message for document {document_id}"
        );
    }
}

/// Revoked authority must fail closed on the workspace send path, including
/// when revocation races dispatch: pending approvals never gain an outbox
/// effect, and approved sends always have a committed bundle.
#[test]
fn a_workspace_revoke_vs_dispatch_race_never_places_a_revoked_send_in_the_outbox() {
    for _ in 0..12 {
        let (dir, store, _document_id, approval_id) = pending_send_fixture();
        let root = dir.path().to_path_buf();
        let authority_root = root.join("authority");
        AuthorityStore::open(&authority_root).unwrap();
        let now = Utc::now().timestamp();

        let _decide_result = std::thread::scope(|scope| {
            let decider = scope.spawn(|| store.decide(approval_id, true));
            scope.spawn(|| {
                let mut stall = None;
                for _ in 0..20_000 {
                    let tokens_dir = authority_root.join("tokens");
                    if tokens_dir.is_dir() {
                        if let Ok(auth) = AuthorityStore::open(&authority_root) {
                            for entry in std::fs::read_dir(&tokens_dir)
                                .unwrap()
                                .filter_map(|entry| entry.ok())
                            {
                                let Some(id) = entry
                                    .file_name()
                                    .to_str()
                                    .and_then(|name| Uuid::parse_str(name).ok())
                                else {
                                    continue;
                                };
                                let _ = auth.revoke_token(id, now, now + 3_600);
                            }
                            let approvals_dir = authority_root.join("approvals");
                            if approvals_dir.is_dir() {
                                for entry in std::fs::read_dir(&approvals_dir)
                                    .unwrap()
                                    .filter_map(|entry| entry.ok())
                                {
                                    let Some(id) = entry
                                        .file_name()
                                        .to_str()
                                        .and_then(|name| Uuid::parse_str(name).ok())
                                    else {
                                        continue;
                                    };
                                    let _ = auth.revoke_approval(id, now, now + 3_600);
                                }
                            }
                        }
                        if stall.is_none() && authority_root.join("approvals").is_dir() {
                            stall =
                                Some(BlockedPath::block(authority_root.join("approvals")).unwrap());
                        }
                    }
                    std::thread::yield_now();
                }
                drop(stall);
            });
            decider.join().unwrap()
        });

        let workspace = Store::open(&root).unwrap().load().unwrap();
        let outbox = outbox_eml_paths(&root);
        match workspace.approvals[0].status {
            ApprovalStatus::Pending => assert!(
                outbox.is_empty(),
                "a pending approval must not have an outbox effect"
            ),
            ApprovalStatus::Approved => {
                assert_eq!(outbox.len(), 1);
                assert!(any_bundle_committed(&authority_root));
            }
            other => panic!("unexpected approval status after race: {other:?}"),
        }
    }
}

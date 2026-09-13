//! Real SIGKILL during an approved send: the workspace must reopen fail-closed.

use super::*;
use sovereign_fault_testing::respawn_self;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::time::Duration;
use tempfile::tempdir;
use uuid::Uuid;

const WORKER_ROOT_ENV: &str = "SOVEREIGN_SIGKILL_SEND_WORKER_ROOT";

/// A venture, a customer, and an invoice ready to send.
fn ready_to_send(store: &Store) -> (Uuid, Uuid) {
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
    (workspace.approvals[0].id, document_id)
}

fn outbox_eml_paths(root: &Path) -> Vec<std::path::PathBuf> {
    std::fs::read_dir(root.join("outbox"))
        .map(|entries| {
            entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| path.extension().is_some_and(|ext| ext == "eml"))
                .collect()
        })
        .unwrap_or_default()
}

/// A composed message is all-or-nothing: real headers and a body, never a torn file.
fn eml_is_complete(path: &Path) -> bool {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(_) => return false,
    };
    if bytes.is_empty() {
        return false;
    }
    let message = match String::from_utf8(bytes) {
        Ok(message) => message,
        Err(_) => return false,
    };
    message.contains("To:")
        && message.contains("Subject:")
        && message.contains("X-Sovereign-Composed:")
        && message.contains("dr.tan@example.com")
        && (message.contains("\r\n\r\n") || message.contains("\n\n"))
}

fn integrity_findings_are_documented_only(report: &IntegrityReport) -> bool {
    report.findings.iter().all(|finding| {
        finding.severity == "warning"
            && (finding.detail.contains("interrupted operation")
                || finding.detail.contains("indeterminate"))
    })
}

fn assert_fail_closed_workspace(root: &Path) {
    let store = Store::open(root).expect("workspace must reopen after SIGKILL");
    let report = store.integrity_check().expect("integrity check must run");
    assert!(
        report.chain_verified,
        "the signed chain must still verify: {:?}",
        report
    );
    assert!(
        report.ok || integrity_findings_are_documented_only(&report),
        "unexpected integrity findings after kill: {:?}",
        report.findings
    );

    let eml_paths = outbox_eml_paths(root);
    assert!(
        eml_paths.len() <= 1,
        "at most one composed .eml, found {}",
        eml_paths.len()
    );
    for path in &eml_paths {
        assert!(
            eml_is_complete(path),
            "a present .eml must be complete, not truncated: {}",
            path.display()
        );
    }

    // A later send on the same root must still complete — retry or a fresh document.
    let workspace = store.load().unwrap();
    if let Some(pending) = workspace
        .approvals
        .iter()
        .find(|approval| approval.status == ApprovalStatus::Pending)
    {
        store.decide(pending.id, true).unwrap();
    } else {
        let customer_id = workspace.customers[0].id;
        let workspace = store
            .create_document(DocumentKind::Offer, customer_id, None, "en")
            .unwrap();
        let document_id = workspace.documents.last().unwrap().id;
        let workspace = store.request_send(document_id).unwrap();
        store
            .decide(workspace.approvals.last().unwrap().id, true)
            .unwrap();
    }

    let after = Store::open(root).unwrap();
    let report = after.integrity_check().unwrap();
    assert!(
        report.ok,
        "integrity check must not fail closed: {:?}",
        report
    );
    assert!(
        report.findings.is_empty() || integrity_findings_are_documented_only(&report),
        "after a completed send only documented journal/chain warnings may remain: {:?}",
        report.findings
    );
    let eml_paths = outbox_eml_paths(root);
    assert!(
        !eml_paths.is_empty(),
        "the follow-up send must leave at least one complete .eml"
    );
    assert!(
        eml_paths.iter().all(|path| eml_is_complete(path)),
        "every .eml after follow-up send must be complete"
    );
}

#[test]
fn a_sigkilled_send_leaves_a_fail_closed_workspace_that_reopens_clean() {
    use rand::Rng;
    let mut rng = rand::thread_rng();

    for attempt in 0..5 {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let marker = (WORKER_ROOT_ENV, root.to_string_lossy().into_owned());

        let mut child = respawn_self(
            "workspace::send_sigkill_tests::sigkilled_send_worker",
            (marker.0, &marker.1),
        )
        .expect("spawn the send worker");

        let stdout = child.stdout.take().expect("worker stdout is piped");
        let mut ready = false;
        for line in BufReader::new(stdout).lines() {
            if line.unwrap().contains("READY") {
                ready = true;
                break;
            }
        }
        assert!(ready, "attempt {attempt}: worker never printed READY");

        let delay_ms = rng.gen_range(0..2_000);
        std::thread::sleep(Duration::from_millis(delay_ms));

        child.kill().expect("SIGKILL the worker mid-send");
        let status = child.wait().expect("reap the worker");
        assert!(
            !status.success(),
            "attempt {attempt}: a killed worker must not report success"
        );

        assert_fail_closed_workspace(&root);
    }
}

/// Seeds a workspace, signals readiness, then runs one full approved send until
/// killed. No-op when the driver did not set the root env var.
#[test]
#[ignore = "spawned by a_sigkilled_send_leaves_a_fail_closed_workspace_that_reopens_clean"]
fn sigkilled_send_worker() {
    let Ok(root) = std::env::var(WORKER_ROOT_ENV) else {
        return;
    };
    if root.is_empty() {
        return;
    }

    let store = Store::open(Path::new(&root)).expect("open seeded workspace root");
    let (approval_id, _document_id) = ready_to_send(&store);

    let mut stdout = std::io::stdout();
    let _ = writeln!(stdout, "READY");
    let _ = stdout.flush();

    // Run until the parent kills us — no timeout here; the driver bounds the soak.
    let _ = store.decide(approval_id, true);
}

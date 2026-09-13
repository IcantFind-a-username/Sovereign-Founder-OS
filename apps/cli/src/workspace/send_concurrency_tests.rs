//! Two real processes deciding the same delivery must produce exactly one effect.

use super::*;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use tempfile::tempdir;
use uuid::Uuid;

const ROOT_ENV: &str = "SOVEREIGN_DECIDE_RACE_ROOT";
const APPROVAL_ENV: &str = "SOVEREIGN_DECIDE_RACE_APPROVAL_ID";

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

fn spawn_decide_worker(root: &Path, approval_id: Uuid) -> (Child, mpsc::Receiver<String>) {
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "workspace::send_concurrency_tests::decide_race_worker",
            "--test-threads=1",
            "--ignored",
            "--nocapture",
        ])
        .env(ROOT_ENV, root.to_string_lossy().as_ref())
        .env(APPROVAL_ENV, approval_id.to_string())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn decide race worker");
    let stdout = child.stdout.take().expect("worker stdout is piped");
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            let _ = tx.send(line.unwrap());
        }
    });
    (child, rx)
}

fn recv_ready(rx: &mpsc::Receiver<String>) {
    loop {
        match rx.recv_timeout(Duration::from_secs(120)) {
            Ok(line) if line.contains("READY") => return,
            Ok(_) => continue,
            Err(_) => panic!("timed out waiting for READY"),
        }
    }
}

fn recv_prefix(rx: &mpsc::Receiver<String>, prefix: &str) -> String {
    loop {
        match rx.recv_timeout(Duration::from_secs(120)) {
            Ok(line) if line.is_empty() => continue,
            Ok(line) if line.starts_with(prefix) => return line,
            Ok(line) => panic!("expected prefix {prefix}, got {line}"),
            Err(_) => panic!("timed out waiting for {prefix}"),
        }
    }
}

#[test]
fn two_processes_deciding_the_same_delivery_produce_exactly_one_effect() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let store = Store::open(root).unwrap();
    let (approval_id, _document_id) = ready_to_send(&store);

    let (mut first, first_lines) = spawn_decide_worker(root, approval_id);
    let (mut second, second_lines) = spawn_decide_worker(root, approval_id);

    recv_ready(&first_lines);
    recv_ready(&second_lines);

    let first_result = recv_prefix(&first_lines, "RESULT ");
    let second_result = recv_prefix(&second_lines, "RESULT ");

    first.wait().expect("reap first worker");
    second.wait().expect("reap second worker");

    let outcomes = [first_result.as_str(), second_result.as_str()];
    let ok_count = outcomes
        .iter()
        .filter(|line| line.starts_with("RESULT ok"))
        .count();
    let err_count = outcomes
        .iter()
        .filter(|line| line.starts_with("RESULT err"))
        .count();
    assert_eq!(ok_count, 1, "exactly one decide must succeed: {outcomes:?}");
    assert_eq!(
        err_count, 1,
        "exactly one decide must fail closed: {outcomes:?}"
    );

    let eml_paths = outbox_eml_paths(root);
    assert_eq!(
        eml_paths.len(),
        1,
        "exactly one .eml must exist, found {}",
        eml_paths.len()
    );

    let store = Store::open(root).expect("workspace reopens after race");
    let report = store.integrity_check().expect("integrity check runs");
    assert!(
        report.ok && report.findings.is_empty(),
        "workspace must pass integrity_check after the race: {:?}",
        report.findings
    );
}

/// Opens the seeded root, signals readiness, then runs `decide(true)` until
/// completion. No-op when the driver did not set the marker env vars.
#[test]
#[ignore = "spawned by two_processes_deciding_the_same_delivery_produce_exactly_one_effect"]
fn decide_race_worker() {
    let Ok(root) = std::env::var(ROOT_ENV) else {
        return;
    };
    let Ok(approval_raw) = std::env::var(APPROVAL_ENV) else {
        return;
    };
    if root.is_empty() || approval_raw.is_empty() {
        return;
    }
    let Ok(approval_id) = Uuid::parse_str(&approval_raw) else {
        return;
    };

    let store = Store::open(Path::new(&root)).expect("open race workspace root");
    let mut stdout = std::io::stdout();
    let _ = writeln!(stdout, "READY");
    let _ = stdout.flush();

    match store.decide(approval_id, true) {
        Ok(_) => {
            let _ = writeln!(stdout, "RESULT ok");
        }
        Err(error) => {
            let _ = writeln!(stdout, "RESULT err {error}");
        }
    }
    let _ = stdout.flush();
}

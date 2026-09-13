//! Product send path: compile worker + signed compiled cache (v01-15).

use super::*;
/// Gauntlet-only signing material (`demo.rs`); product path must not reuse it.
const DEMO_CACHE_SECRET: [u8; 32] = *b"sovereign-demo-cache-signkey-01!";
use std::sync::{Mutex, MutexGuard};
use uuid::Uuid;

static COMPILE_WORKER_ENV: Mutex<()> = Mutex::new(());

fn env_lock() -> MutexGuard<'static, ()> {
    COMPILE_WORKER_ENV.lock().unwrap_or_else(|e| e.into_inner())
}

/// Serializes every test in this module and restores any compile-worker env
/// override on drop so parallel `stage1_suite` runs never inherit a broken path.
struct SerialSendTest {
    _lock: MutexGuard<'static, ()>,
    previous_program: Option<String>,
}

impl SerialSendTest {
    fn begin() -> Self {
        let lock = env_lock();
        Self {
            _lock: lock,
            previous_program: std::env::var("SOVEREIGN_COMPILE_WORKER_PROGRAM").ok(),
        }
    }

    fn override_program(&mut self, path: &str) {
        std::env::set_var("SOVEREIGN_COMPILE_WORKER_PROGRAM", path);
    }
}

impl Drop for SerialSendTest {
    fn drop(&mut self) {
        match &self.previous_program {
            Some(path) => std::env::set_var("SOVEREIGN_COMPILE_WORKER_PROGRAM", path),
            None => std::env::remove_var("SOVEREIGN_COMPILE_WORKER_PROGRAM"),
        }
    }
}

fn store() -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    (dir, store)
}

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

fn cache_blob_count(cache_dir: &std::path::Path) -> usize {
    std::fs::read_dir(cache_dir)
        .map(|entries| {
            entries
                .flatten()
                .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "blob"))
                .count()
        })
        .unwrap_or(0)
}

fn quarantined_count(cache_dir: &std::path::Path) -> usize {
    std::fs::read_dir(cache_dir.join("quarantine"))
        .map(|entries| entries.flatten().count())
        .unwrap_or(0)
}

#[test]
fn product_send_populates_workspace_signed_compile_cache() {
    let _serial = SerialSendTest::begin();
    let (dir, store) = store();
    let (approval_id, _document_id) = ready_to_send(&store);
    store.decide(approval_id, true).unwrap();

    let cache_dir = dir.path().join("compiled-cache");
    assert!(
        cache_blob_count(&cache_dir) >= 1,
        "expected a compiled blob after a successful send"
    );
    let vault = sovereign_vault::Vault::init(dir.path().join("vault")).unwrap();
    let secret = vault.get("compiled_cache_key").unwrap();
    assert_ne!(
        secret.as_slice(),
        DEMO_CACHE_SECRET.as_slice(),
        "product cache must not use the demo cache signing key"
    );
}

#[test]
fn product_send_rejects_a_poisoned_compile_cache_entry() {
    let _serial = SerialSendTest::begin();
    let (dir, store) = store();
    let (approval_id, _) = ready_to_send(&store);
    store.decide(approval_id, true).unwrap();

    let cache_dir = dir.path().join("compiled-cache");
    let blob = std::fs::read_dir(&cache_dir)
        .unwrap()
        .flatten()
        .map(|entry| entry.path())
        .find(|path| path.extension().is_some_and(|ext| ext == "blob"))
        .expect("compiled blob after first send");
    let mut bytes = std::fs::read(&blob).unwrap();
    bytes[0] ^= 0xff;
    std::fs::write(&blob, &bytes).unwrap();

    let workspace = store
        .add_customer("Other", "other@example.com", "")
        .unwrap();
    let customer_id = workspace.customers.last().unwrap().id;
    let workspace = store
        .create_document(DocumentKind::Invoice, customer_id, Some(100_000), "en")
        .unwrap();
    let document_id = workspace.documents.last().unwrap().id;
    let workspace = store.request_send(document_id).unwrap();
    let second_approval = workspace.approvals.last().unwrap().id;
    store.decide(second_approval, true).unwrap();

    assert!(
        quarantined_count(&cache_dir) >= 2,
        "poisoned cache entry must be quarantined before reuse"
    );
    assert!(
        cache_blob_count(&cache_dir) >= 1,
        "send must still succeed via recompile after quarantine"
    );
}

#[test]
fn product_send_fails_when_the_compile_worker_cannot_start() {
    let mut serial = SerialSendTest::begin();
    serial.override_program("/nonexistent/sovereign-compile-worker");

    let (_dir, store) = store();
    let (approval_id, _) = ready_to_send(&store);
    let error = store.decide(approval_id, true).unwrap_err();

    let message = error.to_string();
    assert!(
        message.contains("spawn:") || message.contains("compilation worker"),
        "missing worker must hard-fail, got: {message}"
    );
}

#[test]
fn a_successful_send_proves_the_worker_launched_not_only_failed_closed() {
    let mut serial = SerialSendTest::begin();

    let (dir, store) = store();
    let (approval_id, _) = ready_to_send(&store);
    store.decide(approval_id, true).unwrap();
    let cache_dir = dir.path().join("compiled-cache");
    assert!(cache_blob_count(&cache_dir) >= 1);

    // Force a cache miss on the next send so compilation must run again.
    for entry in std::fs::read_dir(&cache_dir).unwrap().flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "blob") {
            let mut bytes = std::fs::read(&path).unwrap();
            bytes[0] ^= 0xff;
            std::fs::write(&path, &bytes).unwrap();
        }
    }

    serial.override_program("/nonexistent/sovereign-compile-worker");
    let workspace = store
        .add_customer("Other", "other@example.com", "")
        .unwrap();
    let customer_id = workspace.customers.last().unwrap().id;
    let workspace = store
        .create_document(DocumentKind::Invoice, customer_id, Some(100_000), "en")
        .unwrap();
    let document_id = workspace.documents.last().unwrap().id;
    let workspace = store.request_send(document_id).unwrap();
    let approval_id = workspace.approvals.last().unwrap().id;
    let error = store.decide(approval_id, true).unwrap_err();

    let message = error.to_string();
    assert!(
        message.contains("spawn:"),
        "after a proven worker run, a broken worker must fail at spawn on \
         recompile — not masquerade as containment: {message}"
    );
}

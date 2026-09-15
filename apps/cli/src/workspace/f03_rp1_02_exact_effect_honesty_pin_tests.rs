//! Runtime Phase 1 honesty pin for **F03 / RP1-02** (product path).
//!
//! The owner's signed approval and Capability V2 chain bind
//! `workspace.delivery/prepare` over `document_id` + `document:{id}` resource
//! grant only. Final RFC 5322 bytes are passed separately into
//! `execute_signed_approval` and written by `write_outbox_effect` without an
//! exact-effect grant handle.
//!
//! **Invert:** when Program 2 exact local outbox lands (RFC 0006, owner plan
//! Tasks 8/10/11), this test must fail closed — tampered recipient or body
//! bytes must be rejected and approval must not succeed.

use super::compose::compose_email;
use super::*;

use sovereign_artifact::Digest;
use sovereign_audit_ledger::hash_bytes;

fn store() -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    (dir, store)
}

fn seeded_document(store: &Store) -> Document {
    store.set_venture("Acme", "Landing pages").unwrap();
    let workspace = store
        .add_customer("Dr. Tan", "dr.tan@example.com", "")
        .unwrap();
    let customer_id = workspace.customers[0].id;
    let workspace = store
        .create_document(DocumentKind::Invoice, customer_id, Some(250_000), "en")
        .unwrap();
    workspace.documents[0].clone()
}

/// RP1-02 pin: product/local outbox approval does **not** yet bind final
/// recipient + exact `.eml` bytes — only delivery preparation over the document
/// handle. Documents the gap to invert after exact-effect product path ships.
#[test]
fn rp1_02_product_approval_does_not_bind_final_recipient_or_exact_eml_bytes() {
    let (dir, store) = store();
    let document = seeded_document(&store);
    let workspace = store.load().unwrap();
    let customer = workspace
        .customers
        .iter()
        .find(|c| c.id == document.customer_id)
        .unwrap();

    let canonical = compose_email(workspace.venture.as_ref(), Some(customer), &document);
    assert!(
        canonical.contains("dr.tan@example.com"),
        "fixture must use the real customer address in the composed preview"
    );

    let tampered = canonical.replace("dr.tan@example.com", "attacker@evil.example");
    assert_ne!(
        hash_bytes(canonical.as_bytes()),
        hash_bytes(tampered.as_bytes()),
        "tampered message must differ from the honest compose output"
    );

    let record = store
        .execute_signed_approval(&document, tampered.as_bytes())
        .expect(
            "RP1-02 pin: host-supplied delivery bytes still succeed today; \
             invert to expect_err when exact effect binds final bytes",
        );

    let outbox = record.outbox.as_ref().expect("outbox receipt");
    assert_eq!(
        outbox.content_sha256,
        hash_bytes(tampered.as_bytes()),
        "written file must match caller-supplied bytes, not an approval-bound digest"
    );
    assert_ne!(
        outbox.content_sha256,
        hash_bytes(canonical.as_bytes()),
        "approval chain did not seal the canonical compose output"
    );

    let written = std::fs::read(dir.path().join("outbox").join(&outbox.relative_path)).unwrap();
    let on_disk = String::from_utf8(written).unwrap();
    assert!(
        on_disk.contains("attacker@evil.example"),
        "final .eml reflects tampered recipient"
    );
    assert!(
        !on_disk.contains("dr.tan@example.com"),
        "canonical recipient is not enforced by the approval chain"
    );

    // Signed approval covers preparation input (document id + resource), not To:/body.
    let prepare_input = serde_json::json!({
        "document_id": document.id.to_string(),
        "resource": format!("document:{}", document.id),
    });
    let prepare_bytes = serde_json_canonicalizer::to_vec(&prepare_input).unwrap();
    assert_eq!(
        record.canonical_input_digest,
        Digest::domain_separated(b"sovereign.invocation.input.jcs.v1", &prepare_bytes).as_hex(),
        "approval binds canonicalized document preparation input only"
    );
    assert!(
        !prepare_bytes
            .windows(b"dr.tan@example.com".len())
            .any(|w| w == b"dr.tan@example.com"),
        "prepared invocation input must not carry recipient email"
    );
    assert!(
        !prepare_bytes
            .windows(b"attacker@evil.example".len())
            .any(|w| w == b"attacker@evil.example"),
        "prepared invocation input must not carry final delivery bytes"
    );
}

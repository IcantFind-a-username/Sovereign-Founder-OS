//! Getting the composed message out of the product and into the founder's
//! own mail client, over the real HTTP surface.
//!
//! Nothing here sends anything: the product composes an `.eml` after a
//! signed approval, and sending stays the founder's act. Before these routes
//! the only way to that file was a path into a hidden application folder.
//! Two routes close the gap, and each is pinned at its edges:
//!
//! - `POST /api/workspace/message-preview` shows the message a send *will*
//!   compose, before the founder decides;
//! - `GET /api/outbox/<name>` hands over a message a send *did* compose —
//!   only one a signed receipt names, and only while its bytes still match
//!   that receipt.

#[path = "support/ui_server.rs"]
mod ui_server;

use serde_json::json;
use ui_server::UiServer;

fn ok(response: ui_server::Response, what: &str) -> serde_json::Value {
    assert_eq!(response.status, 200, "{what}: status {}", response.status);
    let state = response.json();
    assert_eq!(state["ok"], true, "{what}: {state}");
    state
}

/// A company, a customer with an address, and an invoice with an amount,
/// submitted for sending. Returns the document id and the pending approval.
fn pending_invoice(server: &UiServer) -> (String, String) {
    ok(
        server.post(
            "/api/workspace/profile",
            &json!({
                "name": "Acme Consulting", "service": "Reporting clarity",
                "jurisdiction": "SG", "currency": "SGD", "uen": "202401234A",
                "gst_registered": false, "incorporated_at": null,
                "fiscal_year_end_month": 12, "revenue_estimate_cents": null
            }),
        ),
        "profile",
    );
    let state = ok(
        server.post(
            "/api/workspace/customer",
            &json!({ "name": "Dr. Tan", "email": "tan@clinic.example", "notes": "" }),
        ),
        "customer",
    );
    let customer_id = state["workspace"]["customers"][0]["id"]
        .as_str()
        .expect("customer id")
        .to_owned();
    let state = ok(
        server.post(
            "/api/workspace/invoice",
            &json!({ "customer_id": customer_id, "amount": "4200" }),
        ),
        "invoice",
    );
    let document_id = state["workspace"]["documents"][0]["id"]
        .as_str()
        .expect("document id")
        .to_owned();
    let state = ok(
        server.post(
            "/api/workspace/request-send",
            &json!({ "document_id": document_id }),
        ),
        "request-send",
    );
    let approval_id = state["workspace"]["approvals"][0]["id"]
        .as_str()
        .expect("approval id")
        .to_owned();
    (document_id, approval_id)
}

/// The receipt the approval signed for this document: `(name, sha256)`.
fn receipt(state: &serde_json::Value, document_id: &str) -> (String, String) {
    let approval = state["workspace"]["approvals"]
        .as_array()
        .expect("approvals")
        .iter()
        .find(|approval| approval["document_id"] == document_id && !approval["evidence"].is_null())
        .expect("an approval with evidence");
    let outbox = &approval["evidence"]["outbox"];
    (
        outbox["relative_path"].as_str().expect("path").to_owned(),
        outbox["content_sha256"]
            .as_str()
            .expect("digest")
            .to_owned(),
    )
}

/// Everything after the header block — the part that does not carry the
/// composition time.
fn body_of(message: &str) -> &str {
    message
        .split_once("\r\n\r\n")
        .expect("headers and a body")
        .1
}

/// The whole path a founder takes: read what will go out, approve, take the
/// file. What they downloaded is byte for byte what the approval signed, and
/// its body is exactly what they previewed.
#[test]
fn a_send_can_be_previewed_then_downloaded_byte_for_byte() {
    let server = UiServer::start();
    let (document_id, approval_id) = pending_invoice(&server);

    let preview = ok(
        server.post(
            "/api/workspace/message-preview",
            &json!({ "document_id": document_id }),
        ),
        "message-preview",
    );
    let preview = &preview["preview"];
    let previewed = preview["message"].as_str().expect("message").to_owned();
    assert!(previewed.contains("\r\nTo: \"Dr. Tan\" <tan@clinic.example>\r\n"));
    assert!(previewed.contains("Amount due: SGD 4,200.00\r\n"));
    // And the same envelope decoded for a person, not as encoded-words —
    // including that the sender address is a placeholder.
    assert_eq!(preview["to"], "Dr. Tan <tan@clinic.example>");
    assert_eq!(preview["from"], "Acme Consulting <founder@example.invalid>");
    assert_eq!(preview["from_is_placeholder"], true);
    assert_eq!(preview["to_is_placeholder"], false);
    assert!(preview["subject"].as_str().unwrap().starts_with("Invoice"));
    assert!(preview["body"]
        .as_str()
        .unwrap()
        .contains("Amount due: SGD 4,200.00\n"));
    // A preview records nothing and writes nothing.
    assert!(server
        .find_file(&format!("{}.eml", document_id.replace('-', "")))
        .is_none());

    let state = ok(
        server.post(
            "/api/workspace/decide",
            &json!({ "approval_id": approval_id, "approve": true }),
        ),
        "decide",
    );
    let (name, digest) = receipt(&state, &document_id);

    let download = server.get(&format!("/api/outbox/{name}"));
    assert_eq!(download.status, 200);
    assert_eq!(download.header("Content-Type"), Some("message/rfc822"));
    assert_eq!(
        download.header("Content-Disposition"),
        Some(format!("attachment; filename=\"{name}\"").as_str())
    );
    assert_eq!(download.header("X-Content-Type-Options"), Some("nosniff"));
    assert_eq!(sovereign_audit_ledger::hash_bytes(&download.body), digest);
    let on_disk = std::fs::read(server.find_file(&name).expect("the outbox file")).unwrap();
    assert_eq!(download.body, on_disk);
    let downloaded = String::from_utf8(download.body).expect("utf-8 message");
    assert_eq!(body_of(&downloaded), body_of(&previewed));
}

/// The download answers for signed receipts and nothing else — not another
/// name in the outbox, not a path out of it, not a file changed after it was
/// approved, not a revoked send, and not a foreign page's request.
#[test]
fn a_download_refuses_what_no_signed_receipt_vouches_for() {
    let server = UiServer::start();
    let (document_id, approval_id) = pending_invoice(&server);
    let state = ok(
        server.post(
            "/api/workspace/decide",
            &json!({ "approval_id": approval_id, "approve": true }),
        ),
        "decide",
    );
    let (name, _) = receipt(&state, &document_id);
    let path = server.find_file(&name).expect("the outbox file");

    for (request, why) in [
        (
            "/api/outbox/00000000000000000000000000000000.eml",
            "a well-formed name no receipt names",
        ),
        ("/api/outbox/../ledger.json", "traversal"),
        ("/api/outbox/%2e%2e%2fledger.json", "encoded traversal"),
        ("/api/outbox/", "no name"),
        (&format!("/api/outbox/{name}?x=1") as &str, "a query string"),
    ] {
        let response = server.get(request);
        assert_eq!(response.status, 404, "{why}: {request}");
        assert_eq!(response.json()["ok"], false, "{why}");
    }

    // A file sitting in the outbox that no receipt names — planted there,
    // or left by an interrupted run — is not served just because it exists.
    let planted = path.with_file_name("feedfacefeedfacefeedfacefeedface.eml");
    std::fs::write(&planted, b"Subject: not approved\r\n\r\nhello").unwrap();
    let response = server.get("/api/outbox/feedfacefeedfacefeedfacefeedface.eml");
    assert_eq!(response.status, 404, "an unreceipted file was served");

    // A foreign Host is refused before anything is looked up.
    let headers = vec![
        ("Host".to_string(), "evil.example".to_string()),
        ("Connection".to_string(), "close".to_string()),
    ];
    let response = server.exchange("GET", &format!("/api/outbox/{name}"), &headers, &[]);
    assert_eq!(response.status, 403);

    // Changed after approval: not what was approved, so not handed over.
    let original = std::fs::read(&path).unwrap();
    let mut changed = original.clone();
    changed.extend_from_slice(b"P.S. pay to a different account\r\n");
    std::fs::write(&path, &changed).unwrap();
    let response = server.get(&format!("/api/outbox/{name}"));
    assert_eq!(response.status, 409);
    assert!(response.json()["error"]
        .as_str()
        .unwrap()
        .contains("no longer matches its signed evidence"));
    std::fs::write(&path, &original).unwrap();
    assert_eq!(server.get(&format!("/api/outbox/{name}")).status, 200);

    // Revoked: the file is gone, and the receipt kept as history does not
    // resurrect it.
    ok(
        server.post(
            "/api/workspace/revoke",
            &json!({ "document_id": document_id }),
        ),
        "revoke",
    );
    assert_eq!(server.get(&format!("/api/outbox/{name}")).status, 404);
}

/// A preview needs a real document, and says so in the usual envelope.
#[test]
fn a_preview_of_an_unknown_document_is_refused_with_a_message() {
    let server = UiServer::start();
    let response = server.post(
        "/api/workspace/message-preview",
        &json!({ "document_id": "00000000-0000-0000-0000-000000000000" }),
    );
    assert_eq!(response.status, 200);
    let state = response.json();
    assert_eq!(state["ok"], false);
    assert!(
        state["error"].as_str().unwrap().contains("not found"),
        "{state}"
    );
}

//! The loopback API's HTTP boundary, pinned as it behaves today.
//!
//! Program 1C0 changes exactly this surface. Until these tests existed, no
//! test drove it at all: every approval test called `Store::decide` directly,
//! so both what the server accepts and what it refuses were unverified.

#[path = "support/ui_server.rs"]
mod ui_server;

use serde_json::json;
use ui_server::UiServer;

/// Drive the shipped flow up to a pending approval and return its id.
fn pending_approval(server: &UiServer) -> String {
    let venture = server.post(
        "/api/workspace/venture",
        &json!({ "name": "Boundary Co", "service": "Pinning the HTTP surface" }),
    );
    assert_eq!(venture.status, 200, "venture: {:?}", venture.json());

    let customer = server.post(
        "/api/workspace/customer",
        &json!({ "name": "A Customer", "email": "", "notes": "" }),
    );
    let state = customer.json();
    assert_eq!(state["ok"], true, "customer: {state}");
    let customer_id = state["workspace"]["customers"][0]["id"]
        .as_str()
        .expect("customer id")
        .to_owned();

    let offer = server.post(
        "/api/workspace/offer",
        &json!({ "customer_id": customer_id }),
    );
    let state = offer.json();
    assert_eq!(state["ok"], true, "offer: {state}");
    let document_id = state["workspace"]["documents"][0]["id"]
        .as_str()
        .expect("document id")
        .to_owned();

    let requested = server.post(
        "/api/workspace/request-send",
        &json!({ "document_id": document_id }),
    );
    let state = requested.json();
    assert_eq!(state["ok"], true, "request-send: {state}");
    state["workspace"]["approvals"][0]["id"]
        .as_str()
        .expect("approval id")
        .to_owned()
}

/// A POST that carries no credential of any kind approves a real send, and
/// the backend signs it as the owner's decision.
///
/// This pins today's boundary, and it is the whole reason the product cannot
/// yet claim that only the owner approves: the server has no way to tell the
/// founder from any other process on this machine, so an agent, a script, or
/// a page in another local app can produce owner-signed approval evidence.
///
/// **When this test fails, the 1C0 boundary landed. Invert it — assert the
/// unauthenticated POST is refused — do not delete it.**
#[test]
fn an_unauthenticated_local_post_can_approve_today_1c0_pin() {
    let server = UiServer::start();
    let approval_id = pending_approval(&server);

    let decided = server.post(
        "/api/workspace/decide",
        &json!({ "approval_id": approval_id, "approve": true }),
    );
    assert_eq!(decided.status, 200);
    let state = decided.json();
    assert_eq!(state["ok"], true, "decide: {state}");

    let approval = &state["workspace"]["approvals"][0];
    assert_eq!(approval["id"], approval_id.as_str());
    assert_eq!(
        approval["status"], "approved",
        "an anonymous POST approved a send: {approval}"
    );
    assert!(
        !approval["evidence"].is_null(),
        "and the backend signed owner evidence for it: {approval}"
    );
}

/// A request that reaches the loopback port with someone else's `Host` is
/// refused: a browser on this machine can be pointed at 127.0.0.1 by a remote
/// page, but it still sends that page's `Host`.
#[test]
fn a_foreign_host_header_is_refused() {
    let server = UiServer::start();
    let body = serde_json::to_vec(&json!({ "name": "x", "service": "y" })).expect("body");
    let headers = vec![
        ("Host".into(), "evil.example".into()),
        ("Content-Type".into(), "application/json".into()),
        ("Content-Length".into(), body.len().to_string()),
        ("Connection".into(), "close".into()),
    ];

    let response = server.exchange("POST", "/api/workspace/venture", &headers, &body);
    let state = response.json();
    assert_eq!(state["ok"], false, "{state}");
    assert_eq!(state["error"], "forbidden host");
}

/// Mutations must be JSON. Requiring the header keeps a plain HTML form —
/// which cannot set it — from driving the API cross-origin.
#[test]
fn a_mutation_without_a_json_content_type_is_refused() {
    let server = UiServer::start();
    let body = serde_json::to_vec(&json!({ "name": "x", "service": "y" })).expect("body");
    let headers = vec![
        ("Host".into(), format!("127.0.0.1:{}", server.port())),
        (
            "Content-Type".into(),
            "application/x-www-form-urlencoded".into(),
        ),
        ("Content-Length".into(), body.len().to_string()),
        ("Connection".into(), "close".into()),
    ];

    let response = server.exchange("POST", "/api/workspace/venture", &headers, &body);
    let state = response.json();
    assert_eq!(state["ok"], false, "{state}");
    assert!(
        state["error"]
            .as_str()
            .is_some_and(|error| error.contains("Content-Type")),
        "{state}"
    );
}

/// The body cap is enforced on the bytes read, not on a declared length, so
/// an oversized body cannot exhaust memory before it is rejected.
#[test]
fn a_body_over_the_cap_is_refused() {
    let server = UiServer::start();
    let padding = "x".repeat(64 * 1024 + 1);
    let body = serde_json::to_vec(&json!({ "name": padding, "service": "y" })).expect("body");
    assert!(body.len() > 64 * 1024);

    let response = server.exchange(
        "POST",
        "/api/workspace/venture",
        &server.default_headers(body.len()),
        &body,
    );
    let state = response.json();
    assert_eq!(state["ok"], false, "{state}");
}

/// The export bundle is the one body the app itself asks a person to post
/// back — "verify a backup file" — and a workspace with a few dozen records
/// is already past the general cap. Before this test, every real backup was
/// refused by the very route that exists to check it.
#[test]
fn an_export_past_the_general_cap_still_verifies() {
    let server = UiServer::start();
    let venture = server
        .post(
            "/api/workspace/venture",
            &json!({ "name": "Cap Test Pte Ltd", "service": "backups" }),
        )
        .json();
    assert_eq!(venture["ok"], true, "{venture}");
    let notes = "n".repeat(2048);
    for index in 0..48 {
        let added = server
            .post(
                "/api/workspace/customer",
                &json!({ "name": format!("Customer {index}"), "email": "", "notes": notes }),
            )
            .json();
        assert_eq!(added["ok"], true, "{added}");
    }

    let get_headers = vec![
        ("Host".to_string(), format!("127.0.0.1:{}", server.port())),
        ("Connection".to_string(), "close".to_string()),
    ];
    let bundle = server
        .exchange("GET", "/api/export", &get_headers, &[])
        .json();
    let size = serde_json::to_vec(&bundle).expect("serialize bundle").len();
    assert!(
        size > 64 * 1024,
        "the export is {size} bytes, under the general cap, so this test proves nothing"
    );

    let verdict = server
        .post("/api/verify-export", &json!({ "bundle": bundle }))
        .json();
    assert_eq!(verdict["ok"], true, "{verdict}");
    assert_eq!(verdict["report"]["audit_chain_verified"], true, "{verdict}");
    assert_eq!(verdict["report"]["customers"], 48, "{verdict}");
}

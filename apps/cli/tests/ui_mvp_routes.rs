//! Every route of the founder MVP, over the real HTTP surface.
//!
//! `ui_http_boundary.rs` pins what the loopback server accepts and refuses on
//! one route. This file is about the rest of them: that each route exists and
//! does its job, that the boundary rules are enforced on *every* one rather
//! than on the one that happened to be tested, and that a malformed body is
//! answered rather than crashed on.
//!
//! The first test is the one that keeps the rest honest: it reads the
//! routers' own source and fails if a route is reachable that this file does
//! not name. A route added without a test does not quietly go uncovered.

#[path = "support/ui_server.rs"]
mod ui_server;

use serde_json::json;
use ui_server::UiServer;

/// Every POST route, each exercised below.
const POST_ROUTES: &[&str] = &[
    "/api/gauntlet",
    "/api/verify-export",
    "/api/privacy/preset",
    "/api/privacy/preview",
    "/api/privacy/state",
    "/api/workspace/assist",
    "/api/workspace/compliance/check",
    "/api/workspace/compliance/rules",
    "/api/workspace/confirm-delivery",
    "/api/workspace/customer",
    "/api/workspace/customer/update",
    "/api/workspace/decide",
    "/api/workspace/decision",
    "/api/workspace/document/update",
    "/api/workspace/employee/hire",
    "/api/workspace/employee/run",
    "/api/workspace/employee/status",
    "/api/workspace/follow-up",
    "/api/workspace/follow-up/done",
    "/api/workspace/invoice",
    "/api/workspace/message-preview",
    "/api/workspace/offer",
    "/api/workspace/offer/accepted",
    "/api/workspace/payment",
    "/api/workspace/profile",
    "/api/workspace/project",
    "/api/workspace/project/status",
    "/api/workspace/receivables",
    "/api/workspace/request-send",
    "/api/workspace/revoke",
    "/api/workspace/roles",
    "/api/workspace/task",
    "/api/workspace/task/done",
    "/api/workspace/timeline",
    "/api/workspace/venture",
    "/api/workspace/work-suggestions",
];

/// Every GET route.
const GET_ROUTES: &[&str] = &[
    "/api/command-center",
    "/api/export",
    "/api/model/status",
    "/api/state",
    "/api/workspace",
];

/// Literals in the routers that look like routes and are not, each with the
/// reason it is exempt. An exemption is a claim, so it is written down.
const NOT_ROUTES: &[(&str, &str)] = &[
    (
        "/api/workspace/",
        "the prefix guard that dispatches to workspace_post",
    ),
    (
        "/api/privacy/",
        "the prefix guard that dispatches to the privacy handlers",
    ),
    (
        "/api/outbox/",
        "the prefix guard for composed-message downloads; every answer it gives is pinned in ui_outbox.rs",
    ),
    (
        "/api/workspace/nope",
        "an unknown path used by a unit test inside ui_mvp.rs",
    ),
];

/// The routers' own source, so the scan below reads what is served rather
/// than a list someone maintained by hand.
const ROUTER_SOURCES: [&str; 2] = [
    include_str!("../src/ui.rs"),
    include_str!("../src/ui_mvp.rs"),
];

/// The test that keeps this file from rotting. Every `"/api/…"` literal in
/// the routers is either exercised here or listed as not-a-route with a
/// reason; a new route is neither, so adding one fails this test.
#[test]
fn every_route_in_the_routers_is_named_by_this_file() {
    let mut found: Vec<String> = Vec::new();
    for source in ROUTER_SOURCES {
        let mut rest = source;
        while let Some(at) = rest.find("\"/api/") {
            rest = &rest[at + 1..];
            if let Some(end) = rest.find('"') {
                found.push(rest[..end].to_owned());
            }
        }
    }
    found.sort();
    found.dedup();
    assert!(
        found.len() > 30,
        "the scan found only {} routes, so it is not reading the routers",
        found.len()
    );

    let exempt: Vec<&str> = NOT_ROUTES.iter().map(|(path, _)| *path).collect();
    let uncovered: Vec<&String> = found
        .iter()
        .filter(|path| {
            !POST_ROUTES.contains(&path.as_str())
                && !GET_ROUTES.contains(&path.as_str())
                && !exempt.contains(&path.as_str())
        })
        .collect();
    assert!(
        uncovered.is_empty(),
        "these routes are served and not named in this file: {uncovered:?}"
    );

    // And the reverse: a route named here that no longer exists is a test
    // guarding nothing.
    for path in POST_ROUTES.iter().chain(GET_ROUTES) {
        assert!(
            found.contains(&path.to_string()),
            "{path} is tested here but no longer served"
        );
    }
}

/// A browser on this machine can be pointed at 127.0.0.1 by a remote page,
/// but it still sends that page's `Host`. Every route refuses it — not only
/// the one that was tested when the rule was written.
#[test]
fn every_post_route_refuses_a_foreign_host() {
    let server = UiServer::start();
    let body = serde_json::to_vec(&json!({})).expect("body");
    for path in POST_ROUTES {
        let headers = vec![
            ("Host".to_string(), "evil.example".to_string()),
            ("Content-Type".to_string(), "application/json".to_string()),
            ("Content-Length".to_string(), body.len().to_string()),
            ("Connection".to_string(), "close".to_string()),
        ];
        let response = server.exchange("POST", path, &headers, &body);
        assert_eq!(response.status, 403, "{path} accepted a foreign Host");
    }
}

/// Requiring `application/json` is a CSRF defence: a cross-origin page cannot
/// send that content type without a preflight this server never approves. It
/// only defends the routes that enforce it.
#[test]
fn every_post_route_requires_a_json_content_type() {
    let server = UiServer::start();
    let body = serde_json::to_vec(&json!({})).expect("body");
    for path in POST_ROUTES {
        let headers = vec![
            ("Host".to_string(), format!("127.0.0.1:{}", server.port())),
            (
                "Content-Type".to_string(),
                "application/x-www-form-urlencoded".to_string(),
            ),
            ("Content-Length".to_string(), body.len().to_string()),
            ("Connection".to_string(), "close".to_string()),
        ];
        let response = server.exchange("POST", path, &headers, &body);
        let state = response.json();
        assert_eq!(
            state["ok"], false,
            "{path} accepted a non-JSON content type: {state}"
        );
    }
}

/// The general body cap, on every route that has it. `/api/verify-export` is
/// deliberately not one of them — a real backup is larger than the general
/// cap, which is why that route has its own — so it is checked separately.
#[test]
fn every_post_route_but_verify_export_refuses_a_body_over_the_general_cap() {
    let server = UiServer::start();
    let padding = "x".repeat(64 * 1024 + 1);
    let body = serde_json::to_vec(&json!({ "padding": padding })).expect("body");
    assert!(body.len() > 64 * 1024);

    for path in POST_ROUTES {
        if *path == "/api/verify-export" {
            continue;
        }
        let response = server.exchange("POST", path, &server.default_headers(body.len()), &body);
        let state = response.json();
        assert_eq!(
            state["ok"], false,
            "{path} accepted a body over the general cap: {state}"
        );
    }

    // The one exception takes it, because the thing it exists to check is
    // bigger than the general cap.
    let response = server.exchange(
        "POST",
        "/api/verify-export",
        &server.default_headers(body.len()),
        &body,
    );
    let state = response.json();
    assert_eq!(state["ok"], false, "{state}");
    assert_ne!(
        state["error"], "request body too large",
        "verify-export lost its own larger cap"
    );
}

/// A malformed body is an answer, not a crash. Every route gets one that is
/// valid JSON and wrong, and must reply with a JSON envelope; the server must
/// still be serving afterwards.
#[test]
fn every_post_route_answers_a_malformed_body_and_keeps_serving() {
    let server = UiServer::start();
    // Valid JSON, wrong in several ways at once: unknown keys, a uuid field
    // holding a number, a string field holding an object.
    let malformed = json!({
        "customer_id": 5,
        "document_id": { "nested": true },
        "role": [],
        "name": null,
        "unknown_key": "ignored or refused, never a panic"
    });
    for path in POST_ROUTES {
        if *path == "/api/gauntlet" {
            // Runs the sandbox for real; it is exercised once in the flow
            // test, and running it here would add seconds for no new signal.
            continue;
        }
        let response = server.post(path, &malformed);
        assert_eq!(response.status, 200, "{path} did not answer with 200");
        let state = response.json();
        assert!(
            state.get("ok").is_some(),
            "{path} answered without an ok field: {state}"
        );
        if state["ok"] == false {
            assert!(
                state["error"].is_string(),
                "{path} refused without saying why: {state}"
            );
        }
    }

    // Still alive, and still answering correctly.
    let state = server.get("/api/state").json();
    assert!(
        state["integrity"]["ok"].as_bool().unwrap_or(false),
        "{state}"
    );
}

/// Every GET route answers JSON to the app's own request.
#[test]
fn every_get_route_answers() {
    let server = UiServer::start();
    for path in GET_ROUTES {
        let response = server.get(path);
        assert_eq!(response.status, 200, "{path} did not answer 200");
        assert!(
            !response.body.is_empty(),
            "{path} answered with an empty body"
        );
        // Every one of them is JSON, including the export.
        serde_json::from_slice::<serde_json::Value>(&response.body)
            .unwrap_or_else(|error| panic!("{path} did not answer JSON: {error}"));
    }
}

/// The whole business flow over HTTP, once, so every route is exercised
/// doing its actual job and not only refusing things. This is the walkthrough
/// from `docs/handoff/reports/2026-09-10-mvp-walkthrough-without-a-model.md`,
/// which found two defects by hand; here it runs on every change.
#[test]
fn the_business_flow_runs_end_to_end_over_http() {
    let server = UiServer::start();
    let ok = |response: ui_server::Response, what: &str| -> serde_json::Value {
        assert_eq!(response.status, 200, "{what}");
        let state = response.json();
        assert_eq!(state["ok"], true, "{what}: {state}");
        state
    };

    // ── the company ──
    ok(
        server.post(
            "/api/workspace/venture",
            &json!({ "name": "Route Co", "service": "Reporting sprints" }),
        ),
        "venture",
    );
    let state = ok(
        server.post(
            "/api/workspace/profile",
            &json!({
                "name": "Route Co", "service": "Reporting sprints", "jurisdiction": "SG",
                "currency": "SGD", "uen": "202612345K", "gst_registered": false,
                "fiscal_year_end_month": 12, "revenue_estimate_cents": 1_000_000
            }),
        ),
        "profile",
    );
    assert_eq!(state["workspace"]["venture"]["jurisdiction"], "SG");

    // ── a customer ──
    let state = ok(
        server.post(
            "/api/workspace/customer",
            &json!({ "name": "A Customer", "email": "", "notes": "met at the expo" }),
        ),
        "customer",
    );
    let customer_id = state["workspace"]["customers"][0]["id"]
        .as_str()
        .expect("customer id")
        .to_owned();
    ok(
        server.post(
            "/api/workspace/customer/update",
            &json!({
                "customer_id": customer_id, "name": "A Customer", "email": "a@example.test",
                "notes": "met at the expo",
                "discovery_notes": "Monthly close takes nine days. Budget SGD 4,000-6,000.",
                "stage": "lead", "jurisdiction": "SG"
            }),
        ),
        "customer/update",
    );

    // ── the team ──
    let roles = ok(server.post("/api/workspace/roles", &json!({})), "roles");
    let role_ids: Vec<String> = roles["roles"]
        .as_array()
        .expect("roles")
        .iter()
        .map(|card| card["id"].as_str().expect("role id").to_owned())
        .collect();
    assert!(role_ids.contains(&"analyst".to_owned()), "{role_ids:?}");
    let state = ok(
        server.post(
            "/api/workspace/employee/hire",
            &json!({ "role": "analyst", "name": "Requirements Analyst" }),
        ),
        "employee/hire",
    );
    let employee_id = state["workspace"]["employees"][0]["id"]
        .as_str()
        .expect("employee id")
        .to_owned();
    ok(
        server.post(
            "/api/workspace/employee/status",
            &json!({ "employee_id": employee_id, "status": "paused" }),
        ),
        "employee/status paused",
    );
    ok(
        server.post(
            "/api/workspace/employee/status",
            &json!({ "employee_id": employee_id, "status": "hired" }),
        ),
        "employee/status hired",
    );
    let run = ok(
        server.post(
            "/api/workspace/employee/run",
            &json!({ "employee_id": employee_id, "customer_id": customer_id }),
        ),
        "employee/run",
    );
    let decision_id = run["decision"]["id"]
        .as_str()
        .expect("decision id")
        .to_owned();
    // No model is configured in a test root, so this is the template draft and
    // the answer says so rather than implying a model wrote it.
    assert_eq!(run["decision"]["model_backed"], false, "{run}");
    ok(
        server.post(
            "/api/workspace/decision",
            &json!({ "decision_id": decision_id, "approve": true }),
        ),
        "decision",
    );
    ok(
        server.post("/api/workspace/work-suggestions", &json!({})),
        "work-suggestions",
    );

    // ── an offer, sent and delivered ──
    let state = ok(
        server.post(
            "/api/workspace/offer",
            &json!({ "customer_id": customer_id }),
        ),
        "offer",
    );
    let offer_id = state["workspace"]["documents"][0]["id"]
        .as_str()
        .expect("offer id")
        .to_owned();
    ok(
        server.post(
            "/api/workspace/document/update",
            &json!({ "document_id": offer_id, "title": "Proposal", "body": "Scope.", "amount": "5000" }),
        ),
        "document/update",
    );
    let state = ok(
        server.post(
            "/api/workspace/request-send",
            &json!({ "document_id": offer_id }),
        ),
        "request-send",
    );
    let approval_id = state["workspace"]["approvals"]
        .as_array()
        .expect("approvals")
        .iter()
        .find(|approval| approval["status"] == "pending")
        .expect("a pending approval")["id"]
        .as_str()
        .expect("approval id")
        .to_owned();
    ok(
        server.post(
            "/api/workspace/decide",
            &json!({ "approval_id": approval_id, "approve": true }),
        ),
        "decide",
    );
    let state = ok(
        server.post("/api/workspace/revoke", &json!({ "document_id": offer_id })),
        "revoke",
    );
    assert_eq!(state["workspace"]["documents"][0]["status"], "revoked");
    // Revoked is not a dead end: an edit reopens it as a new draft revision.
    ok(
        server.post(
            "/api/workspace/document/update",
            &json!({ "document_id": offer_id, "title": "Proposal", "body": "Narrower scope.", "amount": "5000" }),
        ),
        "document/update after revoke",
    );
    let state = ok(
        server.post(
            "/api/workspace/request-send",
            &json!({ "document_id": offer_id }),
        ),
        "request-send again",
    );
    let approval_id = state["workspace"]["approvals"]
        .as_array()
        .expect("approvals")
        .iter()
        .find(|approval| approval["status"] == "pending")
        .expect("a pending approval")["id"]
        .as_str()
        .expect("approval id")
        .to_owned();
    ok(
        server.post(
            "/api/workspace/decide",
            &json!({ "approval_id": approval_id, "approve": true }),
        ),
        "decide again",
    );
    ok(
        server.post(
            "/api/workspace/confirm-delivery",
            &json!({ "document_id": offer_id }),
        ),
        "confirm-delivery",
    );
    let state = ok(
        server.post(
            "/api/workspace/offer/accepted",
            &json!({ "document_id": offer_id }),
        ),
        "offer/accepted",
    );
    assert_eq!(state["workspace"]["customers"][0]["stage"], "customer");

    // ── a project, its tasks, and a follow-up ──
    let state = ok(
        server.post(
            "/api/workspace/project",
            &json!({ "customer_id": customer_id, "name": "Close acceleration" }),
        ),
        "project",
    );
    let project_id = state["workspace"]["projects"][0]["id"]
        .as_str()
        .expect("project id")
        .to_owned();
    let state = ok(
        server.post(
            "/api/workspace/task",
            &json!({ "project_id": project_id, "title": "Map the spreadsheets", "due_at": "2026-12-01" }),
        ),
        "task",
    );
    let task_id = state["workspace"]["tasks"][0]["id"]
        .as_str()
        .expect("task id")
        .to_owned();
    ok(
        server.post("/api/workspace/task/done", &json!({ "task_id": task_id })),
        "task/done",
    );
    ok(
        server.post(
            "/api/workspace/project/status",
            &json!({ "project_id": project_id, "status": "done" }),
        ),
        "project/status",
    );
    let state = ok(
        server.post(
            "/api/workspace/follow-up",
            &json!({ "customer_id": customer_id, "note": "Ask for a referral", "due_at": "2026-12-15" }),
        ),
        "follow-up",
    );
    let follow_up_id = state["workspace"]["follow_ups"][0]["id"]
        .as_str()
        .expect("follow-up id")
        .to_owned();
    ok(
        server.post(
            "/api/workspace/follow-up/done",
            &json!({ "follow_up_id": follow_up_id }),
        ),
        "follow-up/done",
    );

    // ── an invoice and its payment ──
    let state = ok(
        server.post(
            "/api/workspace/invoice",
            &json!({ "customer_id": customer_id, "amount": "5000" }),
        ),
        "invoice",
    );
    let invoice_id = state["workspace"]["documents"]
        .as_array()
        .expect("documents")
        .iter()
        .find(|document| document["kind"] == "invoice")
        .expect("an invoice")["id"]
        .as_str()
        .expect("invoice id")
        .to_owned();
    // A draft invoice is not money owed: nothing has been issued yet, and
    // receivables counts only invoices the founder actually approved.
    let receivables = ok(
        server.post("/api/workspace/receivables", &json!({})),
        "receivables while the invoice is a draft",
    );
    assert!(
        receivables["receivables"]
            .as_array()
            .expect("rows")
            .is_empty(),
        "a draft invoice was counted as a receivable: {receivables}"
    );

    let state = ok(
        server.post(
            "/api/workspace/request-send",
            &json!({ "document_id": invoice_id }),
        ),
        "request-send the invoice",
    );
    let approval_id = state["workspace"]["approvals"]
        .as_array()
        .expect("approvals")
        .iter()
        .find(|approval| approval["status"] == "pending")
        .expect("a pending approval")["id"]
        .as_str()
        .expect("approval id")
        .to_owned();
    ok(
        server.post(
            "/api/workspace/decide",
            &json!({ "approval_id": approval_id, "approve": true }),
        ),
        "approve the invoice send",
    );
    let receivables = ok(
        server.post("/api/workspace/receivables", &json!({})),
        "receivables once it is issued",
    );
    assert_eq!(
        receivables["receivables"][0]["outstanding_cents"], 500_000,
        "{receivables}"
    );

    ok(
        server.post(
            "/api/workspace/payment",
            &json!({ "invoice_id": invoice_id, "amount": "5000", "note": "bank transfer" }),
        ),
        "payment",
    );
    let receivables = ok(
        server.post("/api/workspace/receivables", &json!({})),
        "receivables after payment",
    );
    assert_eq!(receivables["receivables"][0]["outstanding_cents"], 0);

    // ── compliance, privacy, assistance, evidence ──
    let report = ok(
        server.post("/api/workspace/compliance/check", &json!({})),
        "compliance/check",
    );
    assert!(!report["report"]["findings"]
        .as_array()
        .expect("findings")
        .is_empty());
    ok(
        server.post(
            "/api/workspace/compliance/check",
            &json!({ "document_id": invoice_id }),
        ),
        "compliance/check on a document",
    );
    let rules = ok(
        server.post(
            "/api/workspace/compliance/rules",
            &json!({ "query": "GST" }),
        ),
        "compliance/rules",
    );
    assert!(!rules["packs"].as_array().expect("packs").is_empty());
    let timeline = ok(
        server.post(
            "/api/workspace/timeline",
            &json!({ "customer_id": customer_id }),
        ),
        "timeline",
    );
    assert!(!timeline["timeline"].as_array().expect("rows").is_empty());
    ok(
        server.post(
            "/api/workspace/assist",
            &json!({ "customer_id": customer_id }),
        ),
        "assist",
    );
    ok(
        server.post("/api/privacy/state", &json!({})),
        "privacy/state",
    );
    ok(
        server.post("/api/privacy/preset", &json!({ "preset": "local_only" })),
        "privacy/preset",
    );
    ok(
        server.post(
            "/api/privacy/preview",
            &json!({ "purpose": "draft_proposal", "customer_id": customer_id }),
        ),
        "privacy/preview",
    );
    ok(
        server.post("/api/privacy/preset", &json!({ "preset": "auto_protect" })),
        "privacy/preset back",
    );

    // The export verifies against its own route, and the gauntlet runs the
    // real sandbox — the two routes that check the kernel rather than the
    // business graph.
    let bundle = server.get("/api/export").json();
    let verdict = ok(
        server.post("/api/verify-export", &json!({ "bundle": bundle })),
        "verify-export",
    );
    assert_eq!(verdict["report"]["audit_chain_verified"], true, "{verdict}");
    let gauntlet = ok(server.post("/api/gauntlet", &json!({})), "gauntlet");
    let results = gauntlet["results"].as_array().expect("results");
    assert!(!results.is_empty());
    assert!(
        results.iter().all(|check| check["pass"] == true),
        "the gauntlet reported a failing check: {gauntlet}"
    );

    // And the audit chain still verifies after all of it.
    let state = server.get("/api/state").json();
    assert_eq!(state["integrity"]["ok"], true, "{state}");
    assert_eq!(state["integrity"]["chain_verified"], true, "{state}");
}

/// The two the backlog named: a subject and a profile that deserialise
/// wrongly must come back as a refusal with a message, not a panic and not a
/// 500. Named separately from the sweep because these are the shapes a real
/// frontend bug would produce.
#[test]
fn a_malformed_run_subject_or_profile_is_refused_with_a_message() {
    let server = UiServer::start();
    ok_venture(&server);

    let state = server
        .post(
            "/api/workspace/employee/run",
            &json!({ "employee_id": "not-a-uuid", "customer_id": "also-not-a-uuid" }),
        )
        .json();
    assert_eq!(state["ok"], false, "{state}");
    assert!(state["error"].is_string(), "{state}");

    let state = server
        .post(
            "/api/workspace/profile",
            &json!({ "name": 5, "service": ["wrong"], "gst_registered": "yes" }),
        )
        .json();
    assert_eq!(state["ok"], false, "{state}");
    assert!(
        state["error"]
            .as_str()
            .expect("an error message")
            .contains("profile"),
        "the refusal does not say what was wrong: {state}"
    );

    // Still serving.
    assert_eq!(server.get("/api/state").status, 200);
}

fn ok_venture(server: &UiServer) {
    let state = server
        .post(
            "/api/workspace/venture",
            &json!({ "name": "Route Co", "service": "Reporting sprints" }),
        )
        .json();
    assert_eq!(state["ok"], true, "{state}");
}

/// An unknown path under a dispatched prefix is refused the same way every
/// other bad request is: 200 with `ok: false` and a message.
///
/// Not a 404, and that is a deliberate reading rather than an oversight. The
/// only client is the frontend shipped in the same binary, so a request for a
/// route that does not exist is a bug in this repository, not a stale caller
/// to be told apart from a refusal. One envelope for every "no" keeps the
/// frontend's error path single. If a second client ever appears, this test
/// is where the decision is written down and where changing it starts.
#[test]
fn an_unknown_route_is_refused_in_the_same_envelope_as_everything_else() {
    let server = UiServer::start();
    let response = server.post("/api/workspace/no-such-route", &json!({}));
    assert_eq!(response.status, 200);
    let state = response.json();
    assert_eq!(state["ok"], false, "{state}");
    assert!(
        state["error"]
            .as_str()
            .expect("a message")
            .contains("not found"),
        "{state}"
    );
}

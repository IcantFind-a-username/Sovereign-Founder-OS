//! HTTP-layer tests over the real loopback server: the ephemeral-port bind
//! the desktop shell depends on, and the request rules the browser relies on
//! (Host allow-list, JSON content type, body cap). These pin today's posture,
//! including the known gap that a local request needs no owner session.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use super::ui;
use super::workspace::{DocumentKind, Store};

/// Serve a temp workspace on an ephemeral port; returns the port and the
/// pending approval id for a document that is waiting on the owner.
fn serve_fixture() -> (tempfile::TempDir, u16, uuid::Uuid) {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    store
        .set_venture("Acme Consulting", "Reporting sprints")
        .unwrap();
    let workspace = store
        .add_customer("Acme Ltd", "alex@example.test", "")
        .unwrap();
    let customer_id = workspace.customers[0].id;
    let workspace = store
        .create_document(DocumentKind::Invoice, customer_id, Some(250_000), "en")
        .unwrap();
    let workspace = store.request_send(workspace.documents[0].id).unwrap();
    let approval_id = workspace.approvals[0].id;

    let (server, port) = ui::bind(0).unwrap();
    let root = dir.path().to_path_buf();
    std::thread::spawn(move || ui::serve(server, port, &root));
    (dir, port, approval_id)
}

/// One raw HTTP/1.1 exchange; returns the status code and body.
fn request(
    port: u16,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: &str,
) -> (u16, String) {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    let mut head = format!("{method} {path} HTTP/1.1\r\n");
    for (name, value) in headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str(&format!(
        "Content-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    ));
    stream.write_all(head.as_bytes()).unwrap();
    stream.write_all(body.as_bytes()).unwrap();
    stream.flush().unwrap();
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).unwrap();
    let text = String::from_utf8_lossy(&raw).to_string();
    let status = text
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or(0);
    let body = text
        .split_once("\r\n\r\n")
        .map(|(_, body)| body.to_owned())
        .unwrap_or_default();
    (status, body)
}

fn json_headers(port: u16) -> Vec<(String, String)> {
    vec![
        ("Host".into(), format!("127.0.0.1:{port}")),
        ("Content-Type".into(), "application/json".into()),
    ]
}

fn borrow(headers: &[(String, String)]) -> Vec<(&str, &str)> {
    headers
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect()
}

/// The desktop shell parses this exact line from the runtime's stdout to
/// learn the port. Changing it silently would break the packaged app, so the
/// string is pinned here and repeated in `apps/desktop/src/main.rs`.
#[test]
fn the_ready_line_the_desktop_shell_parses_is_stable() {
    assert_eq!(ui::READY_PREFIX, "sovereign-ui listening on ");
}

#[test]
fn port_zero_binds_an_ephemeral_port_and_serves_on_it() {
    let (_dir, port, _approval) = serve_fixture();
    assert_ne!(port, 0, "the OS must report the port it actually assigned");
    let headers = json_headers(port);
    let (status, body) = request(port, "GET", "/api/workspace", &borrow(&headers), "");
    assert_eq!(status, 200);
    assert!(body.contains("\"ok\":true"));
    // The Host allow-list follows the bound port, not a compiled-in default.
    let wrong = vec![("Host".to_owned(), "127.0.0.1:7787".to_owned())];
    let (status, _) = request(port, "GET", "/api/workspace", &borrow(&wrong), "");
    assert_eq!(status, 403, "a Host naming another port is not this server");
}

/// When this test fails because the request was refused, the 1C0 owner-session
/// boundary has landed: invert the assertion, do not delete the test.
#[test]
fn an_unauthenticated_local_post_can_approve_today_1c0_pin() {
    let (dir, port, approval_id) = serve_fixture();
    let headers = json_headers(port);
    let (status, body) = request(
        port,
        "POST",
        "/api/workspace/decide",
        &borrow(&headers),
        &format!("{{\"approval_id\":\"{approval_id}\",\"approve\":true}}"),
    );
    assert_eq!(status, 200);
    assert!(body.contains("\"ok\":true"), "{body}");
    let workspace = Store::open(dir.path()).unwrap().load().unwrap();
    assert_eq!(
        workspace.documents[0].status,
        super::workspace::DocumentStatus::ApprovedPendingDelivery,
        "no owner session is required today; this is the gap 1C0 closes"
    );
}

#[test]
fn a_foreign_host_header_is_refused() {
    let (dir, port, approval_id) = serve_fixture();
    let hostile = vec![
        ("Host".to_owned(), "attacker.example".to_owned()),
        ("Content-Type".to_owned(), "application/json".to_owned()),
    ];
    let (status, _) = request(
        port,
        "POST",
        "/api/workspace/decide",
        &borrow(&hostile),
        &format!("{{\"approval_id\":\"{approval_id}\",\"approve\":true}}"),
    );
    assert_eq!(status, 403);
    let workspace = Store::open(dir.path()).unwrap().load().unwrap();
    assert_eq!(
        workspace.approvals[0].status,
        super::workspace::ApprovalStatus::Pending
    );
}

#[test]
fn a_mutation_without_json_content_type_is_refused() {
    let (dir, port, approval_id) = serve_fixture();
    let body = format!("{{\"approval_id\":\"{approval_id}\",\"approve\":true}}");
    for headers in [
        vec![
            ("Host".to_owned(), format!("127.0.0.1:{port}")),
            ("Content-Type".to_owned(), "text/plain".to_owned()),
        ],
        vec![("Host".to_owned(), format!("127.0.0.1:{port}"))],
    ] {
        let (status, _) = request(
            port,
            "POST",
            "/api/workspace/decide",
            &borrow(&headers),
            &body,
        );
        assert_eq!(status, 400);
    }
    let workspace = Store::open(dir.path()).unwrap().load().unwrap();
    assert_eq!(
        workspace.approvals[0].status,
        super::workspace::ApprovalStatus::Pending
    );
}

#[test]
fn a_body_over_the_cap_is_refused() {
    let (dir, port, approval_id) = serve_fixture();
    let headers = json_headers(port);
    let padding = "x".repeat(70 * 1024);
    let body =
        format!("{{\"approval_id\":\"{approval_id}\",\"approve\":true,\"pad\":\"{padding}\"}}");
    let (status, _) = request(
        port,
        "POST",
        "/api/workspace/decide",
        &borrow(&headers),
        &body,
    );
    assert_eq!(status, 400);
    let workspace = Store::open(dir.path()).unwrap().load().unwrap();
    assert_eq!(
        workspace.approvals[0].status,
        super::workspace::ApprovalStatus::Pending
    );
}

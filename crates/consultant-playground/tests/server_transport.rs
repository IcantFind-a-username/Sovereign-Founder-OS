#[path = "../src/catalog.rs"]
mod catalog;
#[path = "../src/domain.rs"]
mod domain;
#[path = "support/transport.rs"]
mod transport;

use domain::{PlaygroundAction, PlaygroundSession};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::io::Write;
use std::net::{Shutdown, TcpListener, TcpStream};
use std::panic;
use std::thread;
use std::time::{Duration, Instant};
use transport::{
    connect, peer_is_gone, raw_request, read_response, request, ChildServer, Response,
};

const STATE: &str = "/api/playground/consultant";
const ACTION: &str = "/api/playground/consultant/action";
const JSON: &str = "application/json; charset=utf-8";

fn get(port: u16, target: &str) -> Response {
    request(port, &raw_request(port, "GET", target, "", b"")).unwrap()
}

fn action_bytes(port: u16, action: &str) -> Vec<u8> {
    let body = serde_json::to_vec(&json!({"action": action})).unwrap();
    raw_request(
        port,
        "POST",
        ACTION,
        &format!(
            "Content-Type: application/json\r\nContent-Length: {}\r\n",
            body.len()
        ),
        &body,
    )
}

fn expected(session: &PlaygroundSession) -> Value {
    json!({"profile": "synthetic_playground", "real_data_enabled": false,
        "persistence": "none", "state": session.read_model(),
        "catalog": catalog::CATALOG, "teaching": session.teaching_read_model()})
}

fn assert_wire(
    response: &Response,
    status: u16,
    mime: Option<&str>,
    allow: Option<&str>,
    length: usize,
    head: bool,
) {
    assert_eq!(response.status, status);
    let mut headers = Vec::new();
    if let Some(mime) = mime {
        headers.push(("Content-Type".to_owned(), mime.to_owned()));
    }
    headers.extend([
        ("Cache-Control".to_owned(), "no-store".to_owned()),
        ("X-Content-Type-Options".to_owned(), "nosniff".to_owned()),
    ]);
    if let Some(allow) = allow {
        headers.push(("Allow".to_owned(), allow.to_owned()));
    }
    headers.extend([
        ("Content-Length".to_owned(), length.to_string()),
        ("Connection".to_owned(), "close".to_owned()),
    ]);
    assert_eq!(
        response.headers, headers,
        "exact ordered application and framing headers"
    );
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        408 => "Request Timeout",
        413 => "Payload Too Large",
        415 => "Unsupported Media Type",
        431 => "Request Header Fields Too Large",
        _ => panic!("unlisted expected status"),
    };
    let mut wire = format!("HTTP/1.1 {status} {reason}\r\n");
    for (name, value) in headers {
        wire.push_str(&format!("{name}: {value}\r\n"));
    }
    wire.push_str("\r\n");
    let mut bytes = wire.into_bytes();
    assert_eq!(response.body.len(), if head { 0 } else { length });
    bytes.extend_from_slice(&response.body);
    assert_eq!(response.raw, bytes, "no extra bytes or second response");
}

fn snapshot(response: Response) -> Value {
    assert_wire(&response, 200, Some(JSON), None, response.body.len(), false);
    serde_json::from_slice(&response.body).unwrap()
}

fn state(port: u16) -> Value {
    snapshot(get(port, STATE))
}

fn post(port: u16, action: &str) -> Value {
    snapshot(request(port, &action_bytes(port, action)).unwrap())
}

fn assert_error(response: Response, status: u16, code: &str, allow: Option<&str>, head: bool) {
    let body = json!({"profile": "synthetic_playground", "real_data_enabled": false,
        "persistence": "none", "error": code});
    let length = serde_json::to_vec(&body).unwrap().len();
    assert_wire(&response, status, Some(JSON), allow, length, head);
    if !head {
        assert_eq!(
            serde_json::from_slice::<Value>(&response.body).unwrap(),
            body
        );
    }
}

fn assert_transport(response: Response, status: u16) {
    assert_wire(&response, status, None, None, 0, false);
}

#[test]
fn server_serves_exact_assets_and_typed_json() {
    let server = ChildServer::start().unwrap();
    assert_ne!(server.port, 0);
    let initial = state(server.port);
    assert_eq!(initial, expected(&PlaygroundSession::new()));
    let entries = initial["catalog"].as_array().unwrap();
    assert_eq!(entries.len(), 32);
    assert_eq!(
        entries
            .iter()
            .map(|entry| entry["key"].as_str().unwrap())
            .collect::<HashSet<_>>()
            .len(),
        32
    );
    assert_eq!(post(server.port, "ShowReportingSearch"), initial); // eighth route
    for (target, mime, bytes) in [
        (
            "/",
            "text/html; charset=utf-8",
            include_bytes!("../assets/index.html").as_slice(),
        ),
        (
            "/assets/styles.css",
            "text/css; charset=utf-8",
            include_bytes!("../assets/styles.css").as_slice(),
        ),
        (
            "/assets/i18n.js",
            "text/javascript; charset=utf-8",
            include_bytes!("../assets/i18n.js").as_slice(),
        ),
        (
            "/assets/app.js",
            "text/javascript; charset=utf-8",
            include_bytes!("../assets/app.js").as_slice(),
        ),
        (
            "/assets/consultant-ui.js",
            "text/javascript; charset=utf-8",
            include_bytes!("../assets/consultant-ui.js").as_slice(),
        ),
        (
            "/favicon.svg",
            "image/svg+xml",
            include_bytes!("../assets/favicon.svg").as_slice(),
        ),
    ] {
        let response = get(server.port, target);
        assert!(!bytes.is_empty());
        assert_wire(&response, 200, Some(mime), None, bytes.len(), false);
        assert_eq!(response.body, bytes, "{target}");
    }
}

#[test]
fn server_preserves_raw_target_method_and_duplicate_headers() {
    let server = ChildServer::start().unwrap();
    let before = post(server.port, "CorrectOfferPrice");
    let port = server.port;
    let body = br#"{"action":"PromoteAcmeToCustomer"}"#;
    let framing = format!(
        "Content-Type: application/json\r\nContent-Length: {}\r\n",
        body.len()
    );
    let valid_host = format!("127.0.0.1:{port}");
    for (host, extras, status, code) in [
        ("localhost".to_owned(), String::new(), 400, "invalid_host"),
        ("127.0.0.1:0".to_owned(), String::new(), 400, "invalid_host"),
        (
            format!("127.0.0.1:{}", if port == 65535 { 65534 } else { port + 1 }),
            String::new(),
            400,
            "invalid_host",
        ),
        (
            valid_host.clone(),
            format!("hOsT: {valid_host}\r\n"),
            400,
            "invalid_host",
        ),
        (
            valid_host.clone(),
            format!("Origin: http://127.0.0.1:{port}\r\noRiGiN: http://127.0.0.1:{port}\r\n"),
            403,
            "origin_forbidden",
        ),
        (
            valid_host.clone(),
            "Origin: https://example.test\r\n".to_owned(),
            403,
            "origin_forbidden",
        ),
        (
            valid_host.clone(),
            "content-type: application/json\r\n".to_owned(),
            415,
            "unsupported_media_type",
        ),
    ] {
        let mut raw =
            format!("POST {ACTION} HTTP/1.1\r\nHost: {host}\r\n{framing}{extras}\r\n").into_bytes();
        raw.extend_from_slice(body);
        assert_error(request(port, &raw).unwrap(), status, code, None, false);
        assert_eq!(state(port), before);
    }
    for (target, status, code) in [
        ("/unknown", 404, "not_found"),
        ("/api/playground/consultant?x", 400, "invalid_target"),
        ("/api/playground/%63onsultant", 400, "invalid_target"),
        ("/assets/./app.js", 400, "invalid_target"),
        ("/assets/../app.js", 400, "invalid_target"),
        ("http://127.0.0.1/", 400, "invalid_target"),
        ("/assets\\app.js", 400, "invalid_target"),
        ("//", 400, "invalid_target"),
        ("/#fragment", 400, "invalid_target"),
    ] {
        assert_error(get(port, target), status, code, None, false);
        assert_eq!(state(port), before);
    }
    for (target, allow) in [("/", "GET"), (STATE, "GET"), (ACTION, "POST")] {
        for method in ["get", "post", "HEAD", "OPTIONS"] {
            assert_error(
                request(port, &raw_request(port, method, target, "", b"")).unwrap(),
                405,
                "method_not_allowed",
                Some(allow),
                method == "HEAD",
            );
            assert_eq!(state(port), before);
        }
    }
    for (target, status, code) in [
        ("/unknown", 404, "not_found"),
        ("/a?b", 400, "invalid_target"),
    ] {
        assert_error(
            request(port, &raw_request(port, "HEAD", target, "", b"")).unwrap(),
            status,
            code,
            None,
            true,
        );
        assert_eq!(state(port), before);
    }
    let correct_origin = format!("Origin: http://127.0.0.1:{port}\r\n");
    assert_eq!(
        snapshot(request(port, &raw_request(port, "GET", STATE, &correct_origin, b"")).unwrap()),
        before
    );
    let http10 = format!("GET {STATE} HTTP/1.0\r\nHost: {valid_host}\r\n\r\n");
    assert_eq!(snapshot(request(port, http10.as_bytes()).unwrap()), before);
}

#[test]
fn server_rejects_framing_without_mutation() {
    let server = ChildServer::start().unwrap();
    let before = post(server.port, "CorrectOfferPrice");
    let port = server.port;
    for framing in [
        "Transfer-Encoding: chunked",
        "Transfer-Encoding: gzip",
        "Transfer-Encoding: chunked\r\nContent-Length: 0",
        "Content-Length: 1\r\nContent-Length: 1",
        "Content-Length: 1\r\nContent-Length: 2",
        "Content-Length: 01",
        "Content-Length: +1",
        "Content-Length: -1",
        "Content-Length: 1,1",
        "Content-Length:",
        "Content-Length: abc",
        "Content-Length: 999999999999999999999999999999999",
        "Expect: 100-continue",
        "Upgrade: h2c",
        "Connection: keep-alive, UpGrAdE",
        "X-Name: a\r\n folded",
        "X-Name : bad",
        "X-Name: a\0b",
    ] {
        let raw = raw_request(port, "POST", ACTION, &format!("{framing}\r\n"), b"");
        assert_transport(request(port, &raw).unwrap(), 400);
        assert_eq!(state(port), before, "{framing:?}");
    }
    for raw in [
        b"GET / HTTP/1.1\nHost: x\n\n".as_slice(),
        b"\r\nGET / HTTP/1.1\r\n\r\n",
        b"GET / HTTP/1.1 extra\r\nHost: x\r\n\r\n",
        b"GET / HTTP/1.1\r\nX: \xff\r\n\r\n",
        b"GET / HTTP/1.1\rX: y\r\n\r\n",
        b"GET / HTTP/1.1\r\nHost:",
        b"",
    ] {
        assert_transport(request(port, raw).unwrap(), 400);
        assert_eq!(state(port), before, "{raw:?}");
    }
    for length in [1, 256, 257, usize::MAX] {
        let raw = raw_request(
            port,
            "POST",
            ACTION,
            &format!("Content-Type: application/json\r\nContent-Length: {length}\r\n"),
            b"",
        );
        assert_transport(request(port, &raw).unwrap(), 400);
        assert_eq!(state(port), before);
    }
    for (additional, status) in [(31, 200), (32, 431)] {
        let raw = raw_request(port, "GET", STATE, &"X-Test: a\r\n".repeat(additional), b"");
        let response = request(port, &raw).unwrap();
        if status == 200 {
            assert_eq!(snapshot(response), before);
        } else {
            assert_transport(response, status);
        }
        assert_eq!(state(port), before);
    }
    for (total, status) in [(8192, 200), (8193, 431)] {
        let empty = raw_request(port, "GET", STATE, "X-Pad: \r\n", b"");
        let padding = "x".repeat(total - empty.len());
        let raw = raw_request(port, "GET", STATE, &format!("X-Pad: {padding}\r\n"), b"");
        assert_eq!(raw.len(), total);
        let response = request(port, &raw).unwrap();
        if status == 200 {
            assert_eq!(snapshot(response), before);
        } else {
            assert_transport(response, status);
        }
        assert_eq!(state(port), before);
    }
}

#[test]
fn server_bounds_body_and_closes_without_draining() {
    let server = ChildServer::start().unwrap();
    let port = server.port;
    let mut padded = br#"{"action":"CorrectOfferPrice"}"#.to_vec();
    padded.resize(256, b' ');
    assert_eq!(padded.len(), 256);
    let raw = raw_request(
        port,
        "POST",
        ACTION,
        "Content-Type: application/json\r\nContent-Length: 256\r\n",
        &padded,
    );
    let mut oracle = PlaygroundSession::new();
    oracle.apply(PlaygroundAction::CorrectOfferPrice);
    assert_eq!(snapshot(request(port, &raw).unwrap()), expected(&oracle));
    let before = state(port);
    padded.push(b' ');
    for length in [257, usize::MAX] {
        let mut stream = connect(port).unwrap();
        let raw = raw_request(
            port,
            "POST",
            ACTION,
            &format!("Content-Type: application/json\r\nContent-Length: {length}\r\n"),
            &padded,
        );
        let started = Instant::now();
        stream.write_all(&raw).unwrap();
        // Keep the sender's write half open: read_response must observe server
        // close without depending on EOF or draining the declared remainder.
        assert_error(
            read_response(&mut stream).unwrap(),
            413,
            "payload_too_large",
            None,
            false,
        );
        assert!(started.elapsed() < Duration::from_secs(3));
        assert_eq!(state(port), before);
    }
}

#[test]
fn server_slow_partial_clients_expire_and_next_request_works() {
    let server = ChildServer::start().unwrap();
    let port = server.port;
    let before = post(port, "CorrectOfferPrice");
    for mode in ["head", "body", "trickle"] {
        let mut stream = connect(port).unwrap();
        let started = Instant::now();
        if mode == "body" {
            stream
                .write_all(&raw_request(
                    port,
                    "POST",
                    ACTION,
                    "Content-Type: application/json\r\nContent-Length: 32\r\n",
                    b"{",
                ))
                .unwrap();
        } else {
            stream
                .write_all(b"POST /api/playground/consultant/action HTTP/1.1\r\n")
                .unwrap();
        }
        if mode == "trickle" {
            // Spend three seconds trickling the head, then enter the body.
            // A reset per read OR a fresh body budget would exceed 6.5 seconds.
            for byte in b"Hos" {
                thread::sleep(Duration::from_secs(1));
                stream.write_all(&[*byte]).unwrap();
            }
            stream.write_all(format!("t: 127.0.0.1:{port}\r\nContent-Type: application/json\r\nContent-Length: 32\r\n\r\n{{").as_bytes()).unwrap();
        }
        assert_transport(read_response(&mut stream).unwrap(), 408);
        let elapsed = started.elapsed();
        assert!(
            elapsed >= Duration::from_secs(4),
            "early timeout for {mode}: {elapsed:?}"
        );
        assert!(
            elapsed < Duration::from_millis(6500),
            "total deadline reset for {mode}: {elapsed:?}"
        );
        assert_eq!(state(port), before);
    }
}

#[test]
fn server_actions_are_single_request_and_disconnect_does_not_reset() {
    let server = ChildServer::start().unwrap();
    let port = server.port;
    let mut oracle = PlaygroundSession::new();
    for (name, action) in [
        ("CorrectOfferPrice", PlaygroundAction::CorrectOfferPrice),
        (
            "PromoteAcmeToCustomer",
            PlaygroundAction::PromoteAcmeToCustomer,
        ),
        ("ShowReportingSearch", PlaygroundAction::ShowReportingSearch),
        ("Reset", PlaygroundAction::Reset),
    ] {
        oracle.apply(action);
        assert_eq!(post(port, name), expected(&oracle));
        assert_eq!(state(port), expected(&oracle));
    }
    let mut pipeline = action_bytes(port, "CorrectOfferPrice");
    pipeline.extend(action_bytes(port, "PromoteAcmeToCustomer"));
    oracle.apply(PlaygroundAction::CorrectOfferPrice);
    let delivered = request(port, &pipeline);
    let subsequent = get(port, STATE);
    match delivered {
        Ok(response) => {
            // Closing with an unread pipelined request may reset the socket
            // after a partial response. Every delivered byte must still be the
            // prefix of the one valid response; delivery is not guaranteed.
            assert!(subsequent.raw.starts_with(&response.raw));
        }
        // Which name the platform gives a vanished peer is the host's
        // business, not the server's: Linux says ENOTCONN where macOS says
        // ECONNRESET. The property under test is that the connection went
        // away without the state changing, asserted just below.
        Err(error) => assert!(peer_is_gone(&error), "unexpected error: {error:?}"),
    }
    assert_eq!(snapshot(subsequent), expected(&oracle));
    for complete in [false, true] {
        post(port, "Reset");
        let before = expected(&PlaygroundSession::new());
        let mut applied = PlaygroundSession::new();
        applied.apply(PlaygroundAction::PromoteAcmeToCustomer);
        let after = expected(&applied);
        let mut bytes = action_bytes(port, "PromoteAcmeToCustomer");
        if !complete {
            bytes.pop();
        }
        let mut stream = connect(port).unwrap();
        stream.write_all(&bytes).unwrap();
        stream.shutdown(Shutdown::Both).unwrap();
        drop(stream);
        let observed = state(port);
        if complete {
            assert!(observed == before || observed == after);
        } else {
            assert_eq!(observed, before);
        }
        assert_eq!(state(port), observed, "no retry or automatic reset");
    }
}

#[test]
fn server_public_run_process_stop_and_restart_restore_fixture() {
    let mut first = ChildServer::start().unwrap();
    let initial = state(first.port);
    assert_eq!(initial, expected(&PlaygroundSession::new()));
    assert!(TcpListener::bind(("127.0.0.1", first.port)).is_err());
    let held = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    assert!(sovereign_consultant_playground::run(held.local_addr().unwrap().port()).is_err());
    assert_ne!(post(first.port, "CorrectOfferPrice"), initial);
    post(first.port, "PromoteAcmeToCustomer");
    let mut partial = connect(first.port).unwrap();
    partial
        .write_all(&raw_request(
            first.port,
            "POST",
            ACTION,
            "Content-Type: application/json\r\nContent-Length: 32\r\n",
            b"{",
        ))
        .unwrap();
    let started = Instant::now();
    assert!(!first.stop().unwrap().success());
    assert!(started.elapsed() < Duration::from_secs(2));
    drop(partial);
    drop(first);
    let second = ChildServer::start().unwrap();
    assert_eq!(state(second.port), initial);
}

#[test]
fn transport_helper_rejects_malformed_response_headers() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let mut peer = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    let (mut writer, _) = listener.accept().unwrap();
    writer
        .write_all(b"HTTP/1.1 200 OK\r\nmalformed\r\n\r\n")
        .unwrap();
    drop(writer);
    peer.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
    assert!(read_response(&mut peer).is_err());
}

#[test]
fn transport_child_startup_failures_are_bounded() {
    // A child that reports something unusable is an error the reader sees at
    // once; waiting out a timeout for it would be a defect, so these must
    // fail well inside the startup budget rather than merely within it.
    for mode in ["bad-port", "oversized"] {
        let started = Instant::now();
        assert!(ChildServer::start_mode(mode).is_err(), "{mode}");
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "{mode} should fail on the invalid line, not on the timeout"
        );
    }

    // A child that reports nothing must still end in an error rather than a
    // hung suite. Two things can end it: the child exiting, which closes the
    // pipe and is the better signal, or the startup budget. The fixture
    // child exits on its own after fifteen seconds, so in practice the
    // parent notices the death first — which is why the bound here is stated
    // against the budget and nothing asserts a lower one. Asserting an exact
    // duration would only pin the fixture's sleep.
    let started = Instant::now();
    assert!(ChildServer::start_mode("no-line").is_err());
    let elapsed = started.elapsed();
    assert!(
        elapsed < ChildServer::STARTUP_BUDGET + Duration::from_secs(10),
        "a silent child must not hang the suite: {elapsed:?}"
    );
}

#[test]
fn transport_child_guard_releases_listener_on_panic() {
    let server = ChildServer::start().unwrap();
    let port = server.port;
    assert!(panic::catch_unwind(panic::AssertUnwindSafe(move || {
        let _guard = server;
        panic!("intentional fixture unwinding");
    }))
    .is_err());
    // No connection was made, so no TIME_WAIT socket can mask listener cleanup.
    let rebound = TcpListener::bind(("127.0.0.1", port)).unwrap();
    assert_eq!(rebound.local_addr().unwrap().port(), port);
}

#[test]
fn child_server() {
    match std::env::var("S1_07_CHILD_SERVER").as_deref() {
        Ok("1") => sovereign_consultant_playground::run(0).unwrap(),
        Ok(mode) => {
            if mode == "bad-port" {
                println!("Playground: http://127.0.0.1:invalid");
            }
            if mode == "oversized" {
                println!("{}", "x".repeat(4097));
            }
            // Finite fallback even if parent cleanup regresses; parent must kill
            // and join well before this guard ends.
            thread::sleep(Duration::from_secs(15));
        }
        Err(_) => {}
    }
}

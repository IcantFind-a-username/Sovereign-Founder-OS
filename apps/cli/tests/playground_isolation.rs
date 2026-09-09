#[path = "../../../crates/consultant-playground/src/catalog.rs"]
mod catalog;
#[path = "../../../crates/consultant-playground/src/domain.rs"]
mod domain;
#[path = "support/isolation.rs"]
mod isolation;
#[path = "../../../crates/consultant-playground/tests/support/transport.rs"]
mod transport;

use domain::{PlaygroundAction, PlaygroundSession};
use isolation::FakeRoot;
use serde_json::{json, Value};
use std::{
    fs,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
use transport::{raw_request, request, ChildServer, Response};

const STATE: &str = "/api/playground/consultant";
const ACTION: &str = "/api/playground/consultant/action";
const JSON: &str = "application/json; charset=utf-8";
const CANARY_A: &[u8] = b"S1-10-A-CANARY-7a9e";
const CANARY_B: &[u8] = b"S1-10-B-CANARY-c2d4";
const CORRECT: &[u8] = br#"{"action":"CorrectOfferPrice"}"#;
const PROMOTE: &[u8] = br#"{"action":"PromoteAcmeToCustomer"}"#;
const SEARCH: &[u8] = br#"{"action":"ShowReportingSearch"}"#;
const RESET: &[u8] = br#"{"action":"Reset"}"#;

fn get(port: u16) -> Vec<u8> {
    raw_request(port, "GET", STATE, "", b"")
}
fn get_target(port: u16, target: &str) -> Vec<u8> {
    raw_request(port, "GET", target, "", b"")
}
fn post(port: u16, body: &[u8]) -> Vec<u8> {
    raw_request(
        port,
        "POST",
        ACTION,
        &format!("Content-Type: {JSON}\r\nContent-Length: {}\r\n", body.len()),
        body,
    )
}
fn expected(session: &PlaygroundSession) -> Value {
    json!({"profile":"synthetic_playground", "real_data_enabled":false, "persistence":"none", "state":session.read_model(), "catalog":catalog::CATALOG, "teaching":session.teaching_read_model()})
}
fn parse(response: &Response, status: u16) -> Value {
    assert_eq!(response.status, status);
    assert_eq!(response.headers[0], ("Content-Type".into(), JSON.into()));
    serde_json::from_slice(&response.body).unwrap()
}

struct RunOutput {
    requests: Vec<Vec<u8>>,
    responses: Vec<Vec<u8>>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    pid: u32,
    port: u16,
    initial: Value,
}

fn run(root: &FakeRoot, requested: Option<u16>, label: &str) -> RunOutput {
    let log = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(".harness/s1-10/fallback")
        .join(format!(
            "{label}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
    fs::create_dir_all(&log).unwrap();
    fs::write(
        log.join("before.snapshot"),
        format!("{:?}", root.snapshot().unwrap()),
    )
    .unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_sovereign"));
    command.args(["playground", "--port", &requested.unwrap_or(0).to_string()]);
    root.configure(&mut command);
    let mut server = ChildServer::start_command(command).unwrap_or_else(|e| {
        panic!("same-port startup failed; environmental diagnostic (requested {requested:?}): {e}")
    });
    let port = server.port;
    let pid = server.id();
    let mut requests = Vec::new();
    let mut responses = Vec::new();
    let mut step = 0usize;
    let mut exchange = |bytes: Vec<u8>| {
        let response = request(port, &bytes).unwrap();
        fs::write(log.join(format!("{step:02}-request.bin")), &bytes).unwrap();
        fs::write(log.join(format!("{step:02}-response.bin")), &response.raw).unwrap();
        step += 1;
        requests.push(bytes);
        responses.push(response.raw.clone());
        response
    };
    for (target, body) in [
        (
            "/",
            include_bytes!("../../../crates/consultant-playground/assets/index.html").as_slice(),
        ),
        (
            "/assets/styles.css",
            include_bytes!("../../../crates/consultant-playground/assets/styles.css").as_slice(),
        ),
        (
            "/assets/i18n.js",
            include_bytes!("../../../crates/consultant-playground/assets/i18n.js").as_slice(),
        ),
        (
            "/assets/app.js",
            include_bytes!("../../../crates/consultant-playground/assets/app.js").as_slice(),
        ),
        (
            "/assets/consultant-ui.js",
            include_bytes!("../../../crates/consultant-playground/assets/consultant-ui.js")
                .as_slice(),
        ),
        (
            "/favicon.svg",
            include_bytes!("../../../crates/consultant-playground/assets/favicon.svg").as_slice(),
        ),
    ] {
        let response = exchange(get_target(port, target));
        assert_eq!(response.status, 200);
        assert_eq!(response.body, body);
    }
    let mut oracle = PlaygroundSession::new();
    let initial = exchange(get(port));
    assert_eq!(parse(&initial, 200), expected(&oracle));
    let initial_state = serde_json::from_slice::<Value>(&initial.body).unwrap();
    for (body, action) in [
        (SEARCH, PlaygroundAction::ShowReportingSearch),
        (CORRECT, PlaygroundAction::CorrectOfferPrice),
        (CORRECT, PlaygroundAction::CorrectOfferPrice),
        (PROMOTE, PlaygroundAction::PromoteAcmeToCustomer),
        (PROMOTE, PlaygroundAction::PromoteAcmeToCustomer),
        (SEARCH, PlaygroundAction::ShowReportingSearch),
    ] {
        let response = exchange(post(port, body));
        oracle.apply(action);
        assert_eq!(parse(&response, 200), expected(&oracle));
        assert_eq!(parse(&exchange(get(port)), 200), expected(&oracle));
    }
    for (body, action) in [
        (RESET, PlaygroundAction::Reset),
        (PROMOTE, PlaygroundAction::PromoteAcmeToCustomer),
        (CORRECT, PlaygroundAction::CorrectOfferPrice),
        (RESET, PlaygroundAction::Reset),
    ] {
        let response = exchange(post(port, body));
        oracle.apply(action);
        assert_eq!(parse(&response, 200), expected(&oracle));
        assert_eq!(parse(&exchange(get(port)), 200), expected(&oracle));
    }
    let errors: [(Vec<u8>, u16, &str); 7] = [(post(port, br#"{"action":"NoSuchAction"}"#), 400, "invalid_action_request"), (post(port, br#"{"action":"CorrectOfferPrice","extra":1}"#), 400, "invalid_action_request"), (post(port, br#"{"action":"CorrectOfferPrice""#), 400, "invalid_action_request"), (get_target(port, "/unknown"), 404, "not_found"), (raw_request(port, "GET", ACTION, "", b""), 405, "method_not_allowed"), (format!("POST {ACTION} HTTP/1.1\r\nHost: wrong.example\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}", CORRECT.len(), std::str::from_utf8(CORRECT).unwrap()).into_bytes(), 400, "invalid_host"), (raw_request(port, "POST", ACTION, &format!("Content-Type: application/json\r\nContent-Length: {}\r\nOrigin: https://example.test\r\n", CORRECT.len()), CORRECT), 403, "origin_forbidden")];
    for (bytes, status, code) in errors {
        let response = exchange(bytes);
        let body = parse(&response, status);
        assert_eq!(body["error"], code);
        assert_eq!(parse(&exchange(get(port)), 200), expected(&oracle));
    }
    let response = exchange(post(port, CORRECT));
    oracle.apply(PlaygroundAction::CorrectOfferPrice);
    assert_eq!(parse(&response, 200), expected(&oracle));
    let response = exchange(post(port, PROMOTE));
    oracle.apply(PlaygroundAction::PromoteAcmeToCustomer);
    assert_eq!(parse(&response, 200), expected(&oracle));
    assert_eq!(parse(&exchange(get(port)), 200), expected(&oracle));
    let status = server.stop().unwrap();
    assert!(!status.success());
    let (stdout, stderr) = server.captured_output().unwrap();
    fs::write(log.join("stdout.bin"), stdout).unwrap();
    fs::write(log.join("stderr.bin"), stderr).unwrap();
    fs::write(
        log.join("after.snapshot"),
        format!("{:?}", root.snapshot().unwrap()),
    )
    .unwrap();
    fs::write(
        log.join("meta.txt"),
        format!(
            "platform={} pid={pid} port={port} responses={} cleanup=complete atime=excluded\n",
            std::env::consts::OS,
            responses.len()
        ),
    )
    .unwrap();
    RunOutput {
        requests,
        responses,
        stdout: stdout.to_vec(),
        stderr: stderr.to_vec(),
        pid,
        port,
        initial: initial_state,
    }
}

#[test]
fn two_cli_roots_have_identical_complete_transcripts_and_no_writes() {
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    panic!("unsupported host for fake-root isolation proof");
    let root_a = FakeRoot::new(CANARY_A).unwrap();
    let root_b = FakeRoot::new(CANARY_B).unwrap();
    assert_ne!(root_a.snapshot().unwrap(), root_b.snapshot().unwrap());
    let before_a = root_a.snapshot().unwrap();
    let before_b = root_b.snapshot().unwrap();
    let a = run(&root_a, None, "run-a");
    let b = run(&root_b, Some(a.port), "run-b");
    assert_ne!(a.pid, b.pid);
    assert_eq!(a.port, b.port);
    assert_eq!(a.requests, b.requests);
    assert_eq!(a.responses, b.responses);
    assert_eq!(a.initial, b.initial);
    assert_eq!(
        a.stdout,
        format!("Playground: http://127.0.0.1:{}\n", a.port).as_bytes()
    );
    assert_eq!(b.stdout, a.stdout);
    assert!(a.stderr.is_empty() && b.stderr.is_empty());
    for bytes in a
        .responses
        .iter()
        .chain([&a.stdout, &a.stderr, &b.stdout, &b.stderr])
    {
        assert!(!bytes.windows(CANARY_A.len()).any(|w| w == CANARY_A));
        assert!(!bytes.windows(CANARY_B.len()).any(|w| w == CANARY_B));
    }
    assert_eq!(root_a.snapshot().unwrap(), before_a);
    assert_eq!(root_b.snapshot().unwrap(), before_b);
}

#[test]
fn fake_root_snapshot_detects_content_added_removed_and_mode_changes() {
    isolation::mutation_probe();
}

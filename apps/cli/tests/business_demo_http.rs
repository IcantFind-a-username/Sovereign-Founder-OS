#[path = "../../../crates/consultant-playground/tests/support/transport.rs"]
mod transport;
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::Shutdown;
use std::process::Command;
use transport::{raw_request, request, ChildServer};

const CSP: &str = "default-src 'none'; script-src 'self'; style-src 'unsafe-inline'; connect-src 'self'; img-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'";
fn start() -> ChildServer {
    let mut command = Command::new(env!("CARGO_BIN_EXE_sovereign"));
    command.args(["business-demo", "--port", "0"]);
    ChildServer::start_command(command).unwrap()
}
fn get(port: u16) -> Value {
    let response = exchange(port, "GET", "/api/demo", &[], b"");
    assert_eq!(response.status, 200);
    app_headers(&response, "application/json; charset=utf-8");
    let body: Value = serde_json::from_slice(&response.body).unwrap();
    assert_eq!(body["ok"], true);
    body["view"].clone()
}
fn exchange(
    port: u16,
    method: &str,
    path: &str,
    headers: &[(String, String)],
    body: &[u8],
) -> transport::Response {
    let mut lines = String::from("Connection: close\r\n");
    for (name, value) in headers {
        lines.push_str(&format!("{name}: {value}\r\n"));
    }
    request(port, &raw_request(port, method, path, &lines, body)).unwrap()
}
fn post_headers(port: u16, length: usize) -> Vec<(String, String)> {
    vec![
        ("Origin".into(), format!("http://127.0.0.1:{port}")),
        ("X-Founder-Demo".into(), "1".into()),
        ("Content-Type".into(), "application/json".into()),
        ("Content-Length".into(), length.to_string()),
    ]
}
fn header<'a>(response: &'a transport::Response, key: &str) -> Option<&'a str> {
    response
        .headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(key))
        .map(|(_, value)| value.as_str())
}
fn app_headers(response: &transport::Response, content_type: &str) {
    for (key, value) in [
        ("Content-Type", content_type),
        ("Cache-Control", "no-store"),
        ("X-Content-Type-Options", "nosniff"),
        ("Referrer-Policy", "no-referrer"),
        ("Content-Security-Policy", CSP),
    ] {
        assert_eq!(header(response, key), Some(value), "{key}");
    }
    assert_eq!(header(response, "Set-Cookie"), None);
    assert_eq!(header(response, "Access-Control-Allow-Origin"), None);
}
fn denied(response: &transport::Response, status: u16, code: &str, field: Option<&str>) {
    assert_eq!(
        response.status,
        status,
        "{}",
        String::from_utf8_lossy(&response.raw)
    );
    app_headers(response, "application/json; charset=utf-8");
    let body: Value = serde_json::from_slice(&response.body).unwrap();
    assert_eq!(
        body,
        json!({"ok":false,"error":{"code":code,"field":field}})
    );
}
fn save_body(view: &Value) -> Value {
    json!({"epoch":view["epoch"],"expected_revision":view["revision"],"command":{"type":"save_company","data":{
        "name":"New company","goal":"New goal","service_name":"New service","service_scope":"New scope",
        "currency":"SGD","unit_price":"3500.00"}}})
}
fn post(port: u16, body: &Value) -> transport::Response {
    let bytes = serde_json::to_vec(body).unwrap();
    exchange(
        port,
        "POST",
        "/api/demo/command",
        &post_headers(port, bytes.len()),
        &bytes,
    )
}
#[test]
fn business_demo_startup_and_routes() {
    let mut server = start();
    assert!(server.id() > 0);
    let seed = get(server.port);
    assert_eq!(seed["case"]["company"]["name"], "North Star Operations");
    assert_eq!(seed["available_commands"], json!(["save_company", "reset"]));
    assert_eq!(seed["summary"].as_object().unwrap().len(), 2);
    assert!(!seed.as_object().unwrap().contains_key("roles"));
    for path in ["/", "/app.js"] {
        assert_eq!(exchange(server.port, "GET", path, &[], b"").status, 200);
    }
    for path in [
        "/missing",
        "/api/demo?q=1",
        "/api/demo/",
        "/api/workspace",
        "/api/export",
        "/business-demo/index.html",
        "/api/business-demo",
        "/%61pp.js",
    ] {
        denied(
            &exchange(server.port, "GET", path, &[], b""),
            404,
            "not_found",
            None,
        );
        assert_eq!(get(server.port), seed);
    }
    let reset =
        json!({"epoch":seed["epoch"],"expected_revision":0,"command":{"type":"reset","data":{}}});
    assert_eq!(post(server.port, &reset).status, 200);
    server.stop().unwrap();
    let (stdout, stderr) = server.captured_output().unwrap();
    assert_eq!(String::from_utf8_lossy(stdout), format!("Playground: http://127.0.0.1:{}\nBusiness demo — synthetic exercise; memory only; no AI or external business actions.\n", server.port));
    assert!(stderr.is_empty());
}
#[test]
fn business_demo_origin_body_and_method_rejections() {
    let mut server = start();
    let port = server.port;
    let seed = get(port);
    let body = serde_json::to_vec(&save_body(&seed)).unwrap();
    for (name, status, code) in [
        ("Host", 403, "forbidden_origin"),
        ("Origin", 403, "forbidden_origin"),
        ("X-Founder-Demo", 403, "forbidden_origin"),
        ("Content-Type", 415, "unsupported_media_type"),
        ("Content-Length", 400, "invalid_request"),
    ] {
        let mut headers = post_headers(port, body.len());
        headers.push((
            name.into(),
            if name == "Content-Length" {
                body.len().to_string()
            } else {
                "bad".into()
            },
        ));
        denied(
            &exchange(port, "POST", "/api/demo/command", &headers, &body),
            status,
            code,
            None,
        );
        assert_eq!(get(port), seed);
    }
    for (name, status, code) in [
        ("Origin", 403, "forbidden_origin"),
        ("X-Founder-Demo", 403, "forbidden_origin"),
        ("Content-Type", 415, "unsupported_media_type"),
        ("Content-Length", 411, "length_required"),
    ] {
        let mut headers = post_headers(port, body.len());
        headers.retain(|(key, _)| key != name);
        denied(
            &exchange(
                port,
                "POST",
                "/api/demo/command",
                &headers,
                if name == "Content-Length" { b"" } else { &body },
            ),
            status,
            code,
            None,
        );
        assert_eq!(get(port), seed);
    }
    for (name, value, status, code) in [
        ("Origin", "http://localhost:7789", 403, "forbidden_origin"),
        ("X-Founder-Demo", "2", 403, "forbidden_origin"),
        ("Content-Type", "text/plain", 415, "unsupported_media_type"),
        (
            "Content-Type",
            "application/jsonx",
            415,
            "unsupported_media_type",
        ),
        (
            "Content-Type",
            "application/json; charset=latin1",
            415,
            "unsupported_media_type",
        ),
        (
            "Content-Type",
            "application/json; charset=utf-8; boundary=x",
            415,
            "unsupported_media_type",
        ),
    ] {
        let mut headers = post_headers(port, body.len());
        headers.iter_mut().find(|(key, _)| key == name).unwrap().1 = value.into();
        denied(
            &exchange(port, "POST", "/api/demo/command", &headers, &body),
            status,
            code,
            None,
        );
        assert_eq!(get(port), seed);
    }
    let missing_host =
        request(port, b"GET /api/demo HTTP/1.1\r\nConnection: close\r\n\r\n").unwrap();
    denied(&missing_host, 403, "forbidden_origin", None);
    assert_eq!(get(port), seed);
    for value in [
        "localhost",
        "localhost:7789",
        "127.0.0.1",
        "attacker.example",
    ] {
        let raw = format!("GET /api/demo HTTP/1.1\r\nHost: {value}\r\nConnection: close\r\n\r\n");
        denied(
            &request(port, raw.as_bytes()).unwrap(),
            403,
            "forbidden_origin",
            None,
        );
        assert_eq!(get(port), seed);
    }
    for (method, path, allow) in [
        ("POST", "/", "GET"),
        ("POST", "/app.js", "GET"),
        ("POST", "/api/demo", "GET"),
        ("GET", "/api/demo/command", "POST"),
        ("HEAD", "/api/demo", "GET"),
    ] {
        let response = exchange(port, method, path, &post_headers(port, 0), b"");
        assert_eq!(response.status, 405);
        assert_eq!(header(&response, "Allow"), Some(allow));
        app_headers(&response, "application/json; charset=utf-8");
        if method != "HEAD" {
            denied(&response, 405, "method_not_allowed", None);
        }
        assert_eq!(get(port), seed);
    }
    denied(
        &exchange(
            port,
            "GET",
            "/api/demo",
            &[("Content-Length".into(), "1".into())],
            b"x",
        ),
        400,
        "invalid_request",
        None,
    );
    assert_eq!(
        exchange(
            port,
            "GET",
            "/api/demo",
            &[("Content-Length".into(), "0".into())],
            b""
        )
        .status,
        200
    );
    let mut chunked = post_headers(port, 0);
    chunked.retain(|(key, _)| key != "Content-Length");
    chunked.push(("Transfer-Encoding".into(), "chunked".into()));
    denied(
        &exchange(port, "POST", "/api/demo/command", &chunked, b"0\r\n\r\n"),
        400,
        "invalid_request",
        None,
    );
    let mut upgrade = post_headers(port, body.len());
    upgrade.push(("Connection".into(), "upgrade".into()));
    denied(
        &exchange(port, "POST", "/api/demo/command", &upgrade, &body),
        400,
        "invalid_request",
        None,
    );
    let oversized = vec![b' '; 32_769];
    denied(
        &exchange(
            port,
            "POST",
            "/api/demo/command",
            &post_headers(port, oversized.len()),
            &oversized,
        ),
        413,
        "payload_too_large",
        None,
    );
    assert_eq!(get(port), seed);
    let raw = String::from_utf8(body.clone()).unwrap();
    let invalid = vec![
        b"{".to_vec(),
        vec![255],
        b"{}".to_vec(),
        format!("{raw} {{}}").into_bytes(),
        raw.replacen("{", "{\"extra\":true,", 1).into_bytes(),
        raw.replacen("\"type\":", "\"extra\":true,\"type\":", 1)
            .into_bytes(),
        raw.replacen("\"name\":", "\"extra\":true,\"name\":", 1)
            .into_bytes(),
        raw.replacen("\"epoch\":", "\"epoch\":\"duplicate\",\"epoch\":", 1)
            .into_bytes(),
        raw.replacen("\"name\":", "\"name\":\"duplicate\",\"name\":", 1)
            .into_bytes(),
        raw.replacen("\"expected_revision\":0", "\"expected_revision\":1.5", 1)
            .into_bytes(),
        raw.replacen("save_company", "other", 1).into_bytes(),
    ];
    for bytes in invalid {
        denied(
            &exchange(
                port,
                "POST",
                "/api/demo/command",
                &post_headers(port, bytes.len()),
                &bytes,
            ),
            400,
            "invalid_json",
            None,
        );
        assert_eq!(get(port), seed);
    }
    let mut invalid_domain = save_body(&seed);
    invalid_domain["command"]["data"]["currency"] = json!("USD");
    denied(
        &post(port, &invalid_domain),
        422,
        "unsupported_currency",
        Some("currency"),
    );
    assert_eq!(get(port), seed);
    server.stop().unwrap();
}
#[test]
fn business_demo_save_conflict_and_reset() {
    let mut server = start();
    let seed = get(server.port);
    let save = save_body(&seed);
    let response = post(server.port, &save);
    assert_eq!(response.status, 200);
    let edited = get(server.port);
    assert_eq!(edited["revision"], 1);
    assert_eq!(edited["case"]["service"]["unit_price_cents"], 350000);
    assert_eq!(
        edited["case"]["company"]["origin"],
        "unverified_experiment_input"
    );
    denied(&post(server.port, &save), 409, "stale_revision", None);
    assert_eq!(get(server.port), edited);
    let reset =
        json!({"epoch":edited["epoch"],"expected_revision":1,"command":{"type":"reset","data":{}}});
    assert_eq!(post(server.port, &reset).status, 200);
    let fresh = get(server.port);
    assert_ne!(fresh["epoch"], seed["epoch"]);
    let mut expected = seed.clone();
    expected["epoch"] = fresh["epoch"].clone();
    assert_eq!(fresh, expected);
    denied(&post(server.port, &reset), 409, "stale_epoch", None);
    assert_eq!(get(server.port), fresh);
    let mut allowed = post_headers(server.port, 0);
    let bytes = serde_json::to_vec(&save_body(&fresh)).unwrap();
    allowed
        .iter_mut()
        .find(|(key, _)| key == "Content-Type")
        .unwrap()
        .1 = "Application/JSON; charset=utf-8".into();
    allowed
        .iter_mut()
        .find(|(key, _)| key == "Content-Length")
        .unwrap()
        .1 = bytes.len().to_string();
    assert_eq!(
        exchange(server.port, "POST", "/api/demo/command", &allowed, &bytes).status,
        200
    );
    server.stop().unwrap();
}
#[test]
fn business_demo_assets_are_local() {
    let mut server = start();
    for (path, media) in [
        ("/", "text/html; charset=utf-8"),
        ("/app.js", "application/javascript; charset=utf-8"),
    ] {
        let response = exchange(server.port, "GET", path, &[], b"");
        assert_eq!(response.status, 200);
        app_headers(&response, media);
        let text = String::from_utf8(response.body).unwrap();
        if path == "/" {
            assert!(text.contains("src=\"/app.js\""));
            assert!(text.contains("Synthetic exercise"));
        } else {
            for forbidden in [
                "innerHTML",
                "document.write",
                "localStorage",
                "sessionStorage",
                "WebSocket",
            ] {
                assert!(!text.contains(forbidden), "{forbidden}");
            }
            assert!(text.contains("fetch('/api/demo'"));
            assert!(text.contains("fetch('/api/demo/command'"));
        }
        assert!(!text.contains("https://"));
    }
    server.stop().unwrap();
}
#[test]
fn business_demo_library_prehandler_responses() {
    let mut server = start();
    let seed = get(server.port);
    let port = server.port;
    let response = exchange(
        port,
        "GET",
        "/api/demo",
        &[("Expect".into(), "unsupported-demo".into())],
        b"",
    );
    assert_eq!(response.status, 417);
    assert!(response.body.is_empty());
    assert_eq!(get(port), seed);
    let body = serde_json::to_vec(&save_body(&seed)).unwrap();
    let mut headers = post_headers(port, body.len());
    headers.push(("Expect".into(), "100-continue".into()));
    let response = exchange(port, "POST", "/api/demo/command", &headers, &body);
    denied(&response, 400, "invalid_request", None);
    assert!(response.raw.starts_with(b"HTTP/1.1 400"));
    assert_eq!(get(port), seed);
    let response = request(port, b"GET / HTTP/1.1\r\nBadHeader\r\n\r\n").unwrap();
    assert_eq!(response.status, 400);
    assert!(response.body.is_empty());
    assert_eq!(get(port), seed);
    let mut stream = transport::connect(port).unwrap();
    stream.write_all(b"GET / HTTP/1.1\r\n").unwrap();
    stream.shutdown(Shutdown::Write).unwrap();
    let mut partial = Vec::new();
    stream.read_to_end(&mut partial).unwrap();
    assert!(partial.is_empty());
    assert_eq!(get(port), seed);
    server.stop().unwrap();
}

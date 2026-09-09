//! The Ollama provider against a fake daemon on a loopback port: health,
//! completion, framing, failure modes, and the gateway's Red-data routing.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;
use std::time::{Duration, Instant};

use sovereign_contracts::DataClass;
use sovereign_model::{
    DeterministicProvider, Health, ModelGateway, ModelProvider, ModelRequest, OllamaConfigError,
    OllamaProvider, ProviderTrust, SkipCause,
};

fn http(status: &str, body: &str) -> Vec<u8> {
    format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .into_bytes()
}

fn content_length(head: &str) -> usize {
    head.lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.trim()
                .eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse().ok())
                .flatten()
        })
        .unwrap_or(0)
}

/// Serve each canned response to one connection, in order, and return the
/// raw requests that were received.
fn fake_daemon(responses: Vec<Vec<u8>>) -> (u16, thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let handle = thread::spawn(move || {
        let mut seen = Vec::new();
        for response in responses {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut request = Vec::new();
            let mut chunk = [0u8; 4096];
            loop {
                let read = stream.read(&mut chunk).unwrap_or(0);
                if read == 0 {
                    break;
                }
                request.extend_from_slice(&chunk[..read]);
                if let Some(split) = request.windows(4).position(|window| window == b"\r\n\r\n") {
                    let head = String::from_utf8_lossy(&request[..split]).to_string();
                    if request.len() >= split + 4 + content_length(&head) {
                        break;
                    }
                }
            }
            seen.push(String::from_utf8_lossy(&request).to_string());
            stream.write_all(&response).unwrap();
            stream.flush().unwrap();
        }
        seen
    });
    (port, handle)
}

fn provider(port: u16) -> OllamaProvider {
    OllamaProvider::new(
        "ollama:test-model",
        &format!("http://127.0.0.1:{port}"),
        "test-model",
    )
    .unwrap()
    .with_timeout(Duration::from_secs(5))
}

fn request(data_class: DataClass) -> ModelRequest {
    ModelRequest {
        task: "draft_outreach".into(),
        prompt: "Draft a short note to Acme about the reporting sprint.".into(),
        data_class,
        max_output_chars: 2048,
    }
}

#[test]
fn healthy_when_the_tags_endpoint_answers() {
    let (port, daemon) = fake_daemon(vec![http("200 OK", r#"{"models":[]}"#)]);
    assert_eq!(provider(port).health(), Health::Healthy);
    let seen = daemon.join().unwrap();
    assert!(seen[0].starts_with("GET /api/tags HTTP/1.1\r\n"));
    assert!(seen[0].contains(&format!("Host: 127.0.0.1:{port}")));
}

#[test]
fn down_when_nothing_listens() {
    let port = {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap().port()
    };
    let provider = provider(port).with_timeout(Duration::from_millis(500));
    assert_eq!(provider.health(), Health::Down);
    assert!(provider.complete(&request(DataClass::Amber)).is_err());
}

#[test]
fn down_when_the_daemon_hangs_past_the_timeout() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let hang = thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        thread::sleep(Duration::from_millis(1500));
        drop(stream);
    });
    let started = Instant::now();
    let provider = provider(port).with_timeout(Duration::from_millis(300));
    assert_eq!(provider.health(), Health::Down);
    assert!(
        started.elapsed() < Duration::from_secs(3),
        "must give up at the timeout"
    );
    hang.join().unwrap();
}

#[test]
fn complete_returns_the_response_text_from_a_non_streaming_request() {
    let (port, daemon) = fake_daemon(vec![http(
        "200 OK",
        r#"{"model":"test-model","response":"  Hello Acme, here is the note.  ","done":true}"#,
    )]);
    let text = provider(port).complete(&request(DataClass::Amber)).unwrap();
    assert_eq!(text, "Hello Acme, here is the note.");
    let seen = daemon.join().unwrap();
    assert!(seen[0].starts_with("POST /api/generate HTTP/1.1\r\n"));
    let body = seen[0].split("\r\n\r\n").nth(1).unwrap();
    let json: serde_json::Value = serde_json::from_str(body).unwrap();
    assert_eq!(json["model"], "test-model");
    assert_eq!(json["stream"], false);
    assert_eq!(
        json["prompt"],
        "Draft a short note to Acme about the reporting sprint."
    );
    assert!(json["options"]["num_predict"].as_u64().unwrap() >= 64);
}

#[test]
fn malformed_json_non_2xx_and_empty_text_are_provider_errors() {
    let (port, daemon) = fake_daemon(vec![http("200 OK", "not json")]);
    assert!(provider(port).complete(&request(DataClass::Amber)).is_err());
    daemon.join().unwrap();

    let (port, daemon) = fake_daemon(vec![http(
        "500 Internal Server Error",
        r#"{"error":"boom"}"#,
    )]);
    assert!(provider(port).complete(&request(DataClass::Amber)).is_err());
    daemon.join().unwrap();

    let (port, daemon) = fake_daemon(vec![http("200 OK", r#"{"response":"   ","done":true}"#)]);
    assert!(provider(port).complete(&request(DataClass::Amber)).is_err());
    daemon.join().unwrap();
}

#[test]
fn non_loopback_base_url_is_refused_at_construction() {
    assert_eq!(
        OllamaProvider::new("o", "http://192.168.1.20:11434", "m").unwrap_err(),
        OllamaConfigError::NotLoopback
    );
    assert_eq!(
        OllamaProvider::new("o", "https://localhost:11434", "m").unwrap_err(),
        OllamaConfigError::NotLoopback
    );
    assert!(OllamaProvider::new("o", "http://localhost:11434", "m").is_ok());
    assert_eq!(
        OllamaProvider::new("o", "http://localhost:11434", "m")
            .unwrap()
            .trust(),
        ProviderTrust::Local
    );
}

#[test]
fn red_data_may_be_served_by_the_local_ollama_provider_and_never_by_cloud() {
    let (port, daemon) = fake_daemon(vec![
        http("200 OK", r#"{"models":[]}"#),
        http("200 OK", r#"{"response":"Local answer.","done":true}"#),
    ]);
    let gateway = ModelGateway::new(vec![
        Box::new(DeterministicProvider::cloud("cloud-first", Health::Healthy)),
        Box::new(provider(port)),
    ]);
    let (response, disclosure) = gateway.complete(&request(DataClass::Red)).unwrap();
    assert_eq!(response.provider_id, "ollama:test-model");
    assert_eq!(response.provider_trust, ProviderTrust::Local);
    assert_eq!(response.text, "Local answer.");
    assert_eq!(disclosure.skipped.len(), 1);
    assert_eq!(disclosure.skipped[0].provider_id, "cloud-first");
    assert_eq!(
        disclosure.skipped[0].reason,
        SkipCause::RedDataConfidentiality
    );
    daemon.join().unwrap();
}

#[test]
fn a_down_daemon_fails_over_to_the_deterministic_stand_in() {
    let port = {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap().port()
    };
    let gateway = ModelGateway::new(vec![
        Box::new(provider(port).with_timeout(Duration::from_millis(300))),
        Box::new(DeterministicProvider::local_echo(
            "local-drafter",
            Health::Healthy,
        )),
    ]);
    let (response, disclosure) = gateway.complete(&request(DataClass::Amber)).unwrap();
    assert_eq!(response.provider_id, "local-drafter");
    assert_eq!(disclosure.skipped[0].provider_id, "ollama:test-model");
    assert_eq!(disclosure.skipped[0].reason, SkipCause::Unhealthy);
}

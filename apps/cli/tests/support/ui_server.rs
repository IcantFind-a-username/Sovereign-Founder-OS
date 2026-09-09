//! A minimal harness that runs the real loopback UI server over real HTTP.
//!
//! Every approval test in this repository so far drives `Store::decide`
//! directly, so the network surface — the one Program 1C0 must change — has
//! never been exercised in either direction. This starts the shipped binary
//! on an ephemeral loopback port against a throwaway state directory and
//! speaks HTTP to it, so a test can assert what the server actually accepts.
//!
//! State is redirected by `HOME`/`XDG_DATA_HOME` because `sovereign ui` has no
//! `--root` flag yet (tracked in the backlog); `dirs::data_local_dir()` follows
//! them on both macOS and Linux. A test must never touch the owner's real data.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::process::{Child, ChildStdout, Command, Stdio};
use std::time::Duration;

/// How long to wait for the child to report its bound port. Generous on
/// purpose: nothing here asserts that the server starts quickly, and the
/// budget exists only so a child that reports nothing fails instead of
/// hanging the suite. A child that dies closes the pipe and fails at once.
const STARTUP_BUDGET: Duration = Duration::from_secs(30);

const READY_PREFIX: &str = "sovereign-ui listening on http://127.0.0.1:";

pub struct UiServer {
    child: Child,
    port: u16,
    // Held so the directory outlives the server; dropping it removes the state.
    _home: tempfile::TempDir,
}

pub struct Response {
    pub status: u16,
    pub body: Vec<u8>,
}

impl Response {
    pub fn json(&self) -> serde_json::Value {
        serde_json::from_slice(&self.body).expect("response body is JSON")
    }
}

impl UiServer {
    pub fn start() -> Self {
        let home = tempfile::tempdir().expect("temp home");
        let mut child = Command::new(env!("CARGO_BIN_EXE_sovereign"))
            .args(["ui", "--port", "0", "--no-open"])
            .env("HOME", home.path())
            .env("XDG_DATA_HOME", home.path().join(".local/share"))
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn sovereign ui");
        let stdout = child.stdout.take().expect("child stdout");
        let port = read_port(stdout);
        Self {
            child,
            port,
            _home: home,
        }
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    /// A POST the app's own frontend could have sent: correct `Host`, correct
    /// `Content-Type`, and no credential of any kind, because none exists.
    pub fn post(&self, path: &str, body: &serde_json::Value) -> Response {
        let body = serde_json::to_vec(body).expect("serialize body");
        self.exchange("POST", path, &self.default_headers(body.len()), &body)
    }

    pub fn default_headers(&self, length: usize) -> Vec<(String, String)> {
        vec![
            ("Host".into(), format!("127.0.0.1:{}", self.port)),
            ("Content-Type".into(), "application/json".into()),
            ("Content-Length".into(), length.to_string()),
            ("Connection".into(), "close".into()),
        ]
    }

    pub fn exchange(
        &self,
        method: &str,
        path: &str,
        headers: &[(String, String)],
        body: &[u8],
    ) -> Response {
        let mut raw = format!("{method} {path} HTTP/1.1\r\n");
        for (name, value) in headers {
            raw.push_str(&format!("{name}: {value}\r\n"));
        }
        raw.push_str("\r\n");
        let mut stream = TcpStream::connect(("127.0.0.1", self.port)).expect("connect");
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .expect("read timeout");
        stream.write_all(raw.as_bytes()).expect("write head");
        stream.write_all(body).expect("write body");
        stream.flush().expect("flush");
        let mut raw = Vec::new();
        stream.read_to_end(&mut raw).expect("read response");
        parse(&raw)
    }
}

impl Drop for UiServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn read_port(stdout: ChildStdout) -> u16 {
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            let Ok(line) = line else { return };
            if let Some(port) = line.trim_end().strip_prefix(READY_PREFIX) {
                let _ = sender.send(port.parse::<u16>());
                return;
            }
        }
        // EOF without the line: the child died. Dropping the sender
        // disconnects the receiver, which fails immediately rather than
        // waiting out the budget.
    });
    receiver
        .recv_timeout(STARTUP_BUDGET)
        .expect("server reported no port before it exited or the budget expired")
        .expect("server reported an unparsable port")
}

fn parse(raw: &[u8]) -> Response {
    let mut headers = [httparse::EMPTY_HEADER; 32];
    let mut response = httparse::Response::new(&mut headers);
    let parsed = response.parse(raw).expect("parse response");
    let body_at = match parsed {
        httparse::Status::Complete(offset) => offset,
        httparse::Status::Partial => panic!("incomplete HTTP response"),
    };
    Response {
        status: response.code.expect("status code"),
        body: raw[body_at..].to_vec(),
    }
}

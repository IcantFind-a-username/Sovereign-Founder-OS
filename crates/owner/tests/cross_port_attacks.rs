//! The attacks a second port on this host can mount, run against the real
//! guard over a real socket.
//!
//! The origin preflight established what a *browser* does here: cookies are
//! not isolated by port, and a page on another port can obtain a
//! user-verified assertion over the same credential. Those are facts about
//! Chrome and they are recorded in the mechanism matrix.
//!
//! This is the other half, and it is the half that is ours. Given that a
//! hostile page can reach this port with a stolen cookie, a spoofed origin,
//! or no credentials at all, what does the surface do? The guard is pure
//! logic, so it could be tested by calling it — but a guard that is correct
//! as a function and wired up wrongly protects nothing, so it is stood up as
//! a server and attacked over TCP.
//!
//! No browser is driven here on purpose. Browser behaviour was measured once,
//! by the preflight, and repeating it in every test would buy flakiness
//! rather than confidence.

#![cfg(feature = "owner-effect-fixture")]

use sovereign_owner::config::ORIGIN;
use sovereign_owner::http_guard::{check, Method, Reject, Request};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};

/// A loopback server that applies the real guard and reports its verdict.
struct Guarded {
    port: u16,
    shutdown: std::sync::Arc<std::sync::atomic::AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl Guarded {
    fn start() -> Self {
        let listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0))).unwrap();
        listener.set_nonblocking(true).unwrap();
        let port = listener.local_addr().unwrap().port();
        let shutdown = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let stop = shutdown.clone();

        let handle = std::thread::spawn(move || {
            while !stop.load(std::sync::atomic::Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, _)) => serve_one(stream),
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(std::time::Duration::from_millis(5));
                    }
                    Err(_) => break,
                }
            }
        });

        Self {
            port,
            shutdown,
            handle: Some(handle),
        }
    }

    /// Send raw request bytes and read the status line back.
    fn send(&self, raw: &str) -> String {
        let mut stream = TcpStream::connect(("127.0.0.1", self.port)).unwrap();
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .unwrap();
        stream.write_all(raw.as_bytes()).unwrap();
        stream.flush().unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        response
            .lines()
            .next()
            .unwrap_or_default()
            .split_once(' ')
            .map(|(_, rest)| rest.to_owned())
            .unwrap_or_default()
    }
}

impl Drop for Guarded {
    fn drop(&mut self) {
        self.shutdown
            .store(true, std::sync::atomic::Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

/// Parse one request and answer with the guard's verdict. Headers are kept in
/// order with repeats, because that is what the guard needs to see.
fn serve_one(mut stream: TcpStream) {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut request_line = String::new();
    if reader.read_line(&mut request_line).is_err() {
        return;
    }
    let mut parts = request_line.split_whitespace();
    let method = match parts.next() {
        Some("GET") => Method::Get,
        Some("POST") => Method::Post,
        _ => return,
    };
    let target = parts.next().unwrap_or("/").to_owned();

    let mut raw_headers = Vec::new();
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap_or(0) == 0 {
            break;
        }
        let line = line.trim_end().to_owned();
        if line.is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            raw_headers.push((name.trim().to_owned(), value.trim().to_owned()));
        }
    }

    let body_len = raw_headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, value)| value.parse::<usize>().ok())
        .unwrap_or(0);

    let headers: Vec<(&str, &str)> = raw_headers
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect();
    let verdict = check(&Request {
        method,
        target: &target,
        headers,
        body_len,
    });

    let status = match verdict {
        Ok(_) => "200 OK",
        Err(Reject::UnknownRoute) => "404 Not Found",
        Err(_) => "403 Forbidden",
    };
    let _ = write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    );
    let _ = stream.flush();
}

/// A request the guard accepts.
///
/// The `Host` header names the compiled origin's port, which is not the port
/// this test's socket listens on. That is deliberate and worth saying: the
/// guard judges the headers a browser sends, not the socket it arrived on. A
/// guard that checked the socket would pass every test here and protect
/// nothing in production, where a reverse proxy or a port forward changes the
/// socket and not the headers.
fn legitimate(extra: &str) -> String {
    format!(
        "POST /fixture/logout HTTP/1.1\r\n\
         Host: localhost:7787\r\n\
         Origin: {ORIGIN}\r\n\
         Sec-Fetch-Site: same-origin\r\n\
         Sec-Fetch-Mode: cors\r\n\
         Content-Type: application/json\r\n\
         Cookie: __Host-sfo_fixture_session=stolen\r\n\
         X-SFO-CSRF: paired\r\n\
         Content-Length: 0\r\n\
         {extra}\r\n"
    )
}

#[test]
fn a_well_formed_request_is_accepted_over_a_real_socket() {
    let server = Guarded::start();
    assert_eq!(server.send(&legitimate("")), "200 OK");
}

/// The attack the preflight makes possible. Another port on this host set the
/// session cookie, so a page there holds one — and a cookie is all a browser
/// will send for it. Without the CSRF value the page never learned, the
/// request is refused.
#[test]
fn a_stolen_cookie_without_the_csrf_value_is_refused() {
    let server = Guarded::start();
    let raw = format!(
        "POST /fixture/logout HTTP/1.1\r\n\
         Host: localhost:7787\r\n\
         Origin: {ORIGIN}\r\n\
         Sec-Fetch-Site: same-origin\r\n\
         Sec-Fetch-Mode: cors\r\n\
         Content-Type: application/json\r\n\
         Cookie: __Host-sfo_fixture_session=stolen\r\n\
         Content-Length: 0\r\n\r\n"
    );
    assert_eq!(server.send(&raw), "403 Forbidden");
}

/// A page on another port sends its own origin. It cannot forge this header —
/// the browser writes it — which is why the exact comparison is the boundary.
#[test]
fn a_request_from_another_port_is_refused_by_its_origin() {
    let server = Guarded::start();
    for origin in [
        "http://localhost:9999",
        "http://127.0.0.1:7787",
        "https://localhost:7787",
        "null",
    ] {
        let raw = format!(
            "POST /fixture/logout HTTP/1.1\r\n\
             Host: localhost:7787\r\n\
             Origin: {origin}\r\n\
             Sec-Fetch-Site: same-origin\r\n\
             Sec-Fetch-Mode: cors\r\n\
             Content-Type: application/json\r\n\
             Cookie: __Host-sfo_fixture_session=stolen\r\n\
             X-SFO-CSRF: paired\r\n\
             Content-Length: 0\r\n\r\n"
        );
        assert_eq!(
            server.send(&raw),
            "403 Forbidden",
            "a request from {origin} was accepted"
        );
    }
}

/// A cross-site navigation or form post carries different fetch metadata, and
/// cannot set a JSON content type. Two independent barriers, each tested with
/// the other left correct so it is clear which one fired.
#[test]
fn a_cross_site_form_post_is_refused_twice_over() {
    let server = Guarded::start();

    let cross_site = format!(
        "POST /fixture/logout HTTP/1.1\r\n\
         Host: localhost:7787\r\n\
         Origin: {ORIGIN}\r\n\
         Sec-Fetch-Site: cross-site\r\n\
         Sec-Fetch-Mode: cors\r\n\
         Content-Type: application/json\r\n\
         Cookie: __Host-sfo_fixture_session=x\r\n\
         X-SFO-CSRF: y\r\n\
         Content-Length: 0\r\n\r\n"
    );
    assert_eq!(server.send(&cross_site), "403 Forbidden");

    let form = format!(
        "POST /fixture/logout HTTP/1.1\r\n\
         Host: localhost:7787\r\n\
         Origin: {ORIGIN}\r\n\
         Sec-Fetch-Site: same-origin\r\n\
         Sec-Fetch-Mode: cors\r\n\
         Content-Type: application/x-www-form-urlencoded\r\n\
         Cookie: __Host-sfo_fixture_session=x\r\n\
         X-SFO-CSRF: y\r\n\
         Content-Length: 0\r\n\r\n"
    );
    assert_eq!(server.send(&form), "403 Forbidden");
}

/// A second `Origin` header is what a confused proxy or a crafted request
/// delivers, and a server that read the first would judge one while honouring
/// the other.
#[test]
fn a_duplicated_origin_header_is_refused_over_the_wire() {
    let server = Guarded::start();
    let raw = format!(
        "POST /fixture/logout HTTP/1.1\r\n\
         Host: localhost:7787\r\n\
         Origin: {ORIGIN}\r\n\
         Origin: http://evil.test\r\n\
         Sec-Fetch-Site: same-origin\r\n\
         Sec-Fetch-Mode: cors\r\n\
         Content-Type: application/json\r\n\
         Cookie: __Host-sfo_fixture_session=x\r\n\
         X-SFO-CSRF: y\r\n\
         Content-Length: 0\r\n\r\n"
    );
    assert_eq!(server.send(&raw), "403 Forbidden");
}

/// Nothing outside the closed route list exists, including the near misses
/// somebody probing would try first.
#[test]
fn probing_for_routes_finds_nothing() {
    let server = Guarded::start();
    for target in [
        "/fixture/admin",
        "/api/fixture/logout",
        "/fixture/logout/",
        "/fixture/logout?force=1",
        "/.env",
    ] {
        let raw = format!(
            "POST {target} HTTP/1.1\r\n\
             Host: localhost:7787\r\n\
             Origin: {ORIGIN}\r\n\
             Sec-Fetch-Site: same-origin\r\n\
             Sec-Fetch-Mode: cors\r\n\
             Content-Type: application/json\r\n\
             Cookie: __Host-sfo_fixture_session=x\r\n\
             X-SFO-CSRF: y\r\n\
             Content-Length: 0\r\n\r\n"
        );
        let status = server.send(&raw);
        assert!(
            status.starts_with("404") || status.starts_with("403"),
            "{target} returned {status}"
        );
    }
}

/// Logout needs credentials; registration and login must not, or there would
/// be no way to obtain any.
#[test]
fn only_the_pre_session_routes_answer_without_credentials() {
    let server = Guarded::start();
    let bare = |target: &str| {
        format!(
            "POST {target} HTTP/1.1\r\n\
             Host: localhost:7787\r\n\
             Origin: {ORIGIN}\r\n\
             Sec-Fetch-Site: same-origin\r\n\
             Sec-Fetch-Mode: cors\r\n\
             Content-Type: application/json\r\n\
             Content-Length: 0\r\n\r\n"
        )
    };
    assert_eq!(server.send(&bare("/fixture/login/start")), "200 OK");
    assert_eq!(server.send(&bare("/fixture/register/start")), "200 OK");
    assert_eq!(server.send(&bare("/fixture/logout")), "403 Forbidden");
}

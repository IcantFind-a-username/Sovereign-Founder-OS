//! Closed v2 auth and effect-prepare/preview/approve routes.
//!
//! Dispatch and reconcile are D05/D06 and remain unknown. Unknown paths are
//! refused before headers are trusted.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::time::Instant;

use sovereign_owner::http_guard::{Method, Reject, Request, MAX_BODY_BYTES};
use sovereign_owner::session::{SessionTokens, TOKEN_LEN};

use crate::listener::ORIGIN;
use crate::owner_surface::{
    FixtureRoute, OwnerResponse, OwnerSurface, CSRF_HEADER, SESSION_COOKIE,
};

const ROUTES: &[(FixtureRoute, &str, Method)] = &[
    (FixtureRoute::Shell, "/", Method::Get),
    (
        FixtureRoute::RegisterStart,
        "/api/fixture/auth/register/start",
        Method::Post,
    ),
    (
        FixtureRoute::RegisterFinish,
        "/api/fixture/auth/register/finish",
        Method::Post,
    ),
    (
        FixtureRoute::LoginStart,
        "/api/fixture/auth/login/start",
        Method::Post,
    ),
    (
        FixtureRoute::LoginFinish,
        "/api/fixture/auth/login/finish",
        Method::Post,
    ),
    (
        FixtureRoute::Logout,
        "/api/fixture/auth/logout",
        Method::Post,
    ),
    (
        FixtureRoute::Prepare,
        "/api/fixture/effect/prepare",
        Method::Post,
    ),
    (
        FixtureRoute::Preview,
        "/api/fixture/effect/preview",
        Method::Post,
    ),
    (
        FixtureRoute::ApproveStart,
        "/api/fixture/effect/approve/start",
        Method::Post,
    ),
    (
        FixtureRoute::ApproveFinish,
        "/api/fixture/effect/approve/finish",
        Method::Post,
    ),
];

fn lookup_route(method: Method, path: &str) -> Option<FixtureRoute> {
    ROUTES
        .iter()
        .find(|(_, route_path, route_method)| *route_path == path && *route_method == method)
        .map(|(route, _, _)| *route)
}

pub fn route_path(route: FixtureRoute) -> &'static str {
    ROUTES
        .iter()
        .find(|(candidate, _, _)| *candidate == route)
        .map(|(_, path, _)| *path)
        .expect("every route has a path")
}

fn expected_host() -> &'static str {
    ORIGIN.trim_start_matches("http://")
}

fn header<'a>(request: &'a Request<'a>, name: &str) -> Option<&'a str> {
    request
        .headers
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(name))
        .map(|(_, value)| *value)
}

fn count(request: &Request<'_>, name: &str) -> usize {
    request
        .headers
        .iter()
        .filter(|(key, _)| key.eq_ignore_ascii_case(name))
        .count()
}

/// Judge one request against the closed v2 route table and compiled origin.
pub fn check_request(request: &Request<'_>) -> Result<FixtureRoute, Reject> {
    let (path, query) = match request.target.split_once('?') {
        Some((path, query)) => (path, Some(query)),
        None => (request.target, None),
    };
    let route = lookup_route(request.method, path).ok_or(Reject::UnknownRoute)?;
    if query.is_some() {
        return Err(Reject::QueryNotAllowed);
    }

    for name in ["host", "origin", "cookie", CSRF_HEADER, "content-type"] {
        if count(request, name) > 1 {
            return Err(Reject::DuplicateHeader);
        }
    }

    if header(request, "host") != Some(expected_host()) {
        return Err(Reject::WrongHost);
    }
    if header(request, "origin") != Some(ORIGIN) {
        return Err(Reject::WrongOrigin);
    }
    if header(request, "sec-fetch-site") != Some("same-origin") {
        return Err(Reject::WrongFetchMetadata);
    }
    if request.method == Method::Post && header(request, "sec-fetch-mode") != Some("cors") {
        return Err(Reject::WrongFetchMetadata);
    }

    if request.method == Method::Post {
        if header(request, "content-type") != Some("application/json") {
            return Err(Reject::WrongContentType);
        }
        if request.body_len > MAX_BODY_BYTES {
            return Err(Reject::BodyTooLarge);
        }
    }

    let has_cookie = header(request, "cookie").is_some();
    let has_csrf = header(request, CSRF_HEADER).is_some();
    if has_cookie != has_csrf {
        return Err(Reject::IncompleteCredentials);
    }
    if !has_cookie && !route.is_pre_session() {
        return Err(Reject::IncompleteCredentials);
    }

    Ok(route)
}

pub fn parse_session_tokens(request: &Request<'_>) -> Option<SessionTokens> {
    let cookie_header = header(request, "cookie")?;
    let csrf_hex = header(request, CSRF_HEADER)?;
    let cookie_hex = cookie_header.split(';').find_map(|part| {
        let part = part.trim();
        part.split_once('=')
            .filter(|(name, _)| name.trim() == SESSION_COOKIE)
            .map(|(_, value)| value.trim())
    })?;
    let cookie = decode_token(cookie_hex)?;
    let csrf = decode_token(csrf_hex)?;
    Some(SessionTokens { cookie, csrf })
}

fn decode_token(hex_value: &str) -> Option<[u8; TOKEN_LEN]> {
    let bytes = hex::decode(hex_value.trim()).ok()?;
    bytes.try_into().ok()
}

fn os_random() -> [u8; TOKEN_LEN] {
    use rand::rngs::OsRng;
    use rand::RngCore;
    let mut bytes = [0u8; TOKEN_LEN];
    OsRng.fill_bytes(&mut bytes);
    bytes
}

pub fn handle_checked(
    surface: &mut OwnerSurface,
    route: FixtureRoute,
    request: &Request<'_>,
    body: &[u8],
    now: Instant,
) -> HttpOutcome {
    let presented = parse_session_tokens(request);
    let response = surface.dispatch(route, body, presented.as_ref(), now, os_random);
    HttpOutcome::from_owner(response)
}

#[derive(Debug)]
pub struct HttpOutcome {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl HttpOutcome {
    fn from_owner(response: OwnerResponse) -> Self {
        let body = serde_json::to_vec(&response.body).unwrap_or_else(|_| b"{}".to_vec());
        let mut headers = vec![
            ("Content-Type".to_owned(), "application/json".to_owned()),
            ("Content-Length".to_owned(), body.len().to_string()),
            ("Connection".to_owned(), "close".to_owned()),
        ];
        if let Some(tokens) = response.issued {
            headers.push((
                "Set-Cookie".to_owned(),
                format!(
                    "{SESSION_COOKIE}={}; Path=/; Secure; HttpOnly; SameSite=Strict",
                    hex::encode(tokens.cookie)
                ),
            ));
        }
        Self {
            status: response.status,
            headers,
            body,
        }
    }

    fn from_reject(reject: Reject) -> Self {
        let status = match reject {
            Reject::UnknownRoute => 404,
            _ => 403,
        };
        let body = serde_json::to_vec(&serde_json::json!({ "error": format!("{reject:?}") }))
            .unwrap_or_else(|_| b"{}".to_vec());
        Self {
            status,
            headers: vec![
                ("Content-Type".to_owned(), "application/json".to_owned()),
                ("Content-Length".to_owned(), body.len().to_string()),
                ("Connection".to_owned(), "close".to_owned()),
            ],
            body,
        }
    }

    pub fn write_to(&self, stream: &mut TcpStream) -> std::io::Result<()> {
        let reason = match self.status {
            200 => "OK",
            403 => "Forbidden",
            404 => "Not Found",
            _ => "Error",
        };
        write!(stream, "HTTP/1.1 {} {reason}\r\n", self.status)?;
        for (name, value) in &self.headers {
            write!(stream, "{name}: {value}\r\n")?;
        }
        write!(stream, "\r\n")?;
        stream.write_all(&self.body)?;
        stream.flush()
    }
}

/// Parse one HTTP/1.1 request from an already-accepted stream. Does not bind.
pub fn handle_stream(
    stream: &mut TcpStream,
    surface: &mut OwnerSurface,
    now: Instant,
) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;
    let mut parts = request_line.split_whitespace();
    let method = match parts.next() {
        Some("GET") => Method::Get,
        Some("POST") => Method::Post,
        _ => return Ok(()),
    };
    let target = parts.next().unwrap_or("/").to_owned();

    let mut raw_headers = Vec::new();
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {}
            Err(_) => return Ok(()),
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
    if body_len > MAX_BODY_BYTES {
        let outcome = HttpOutcome::from_reject(Reject::BodyTooLarge);
        return outcome.write_to(stream);
    }
    let mut body = vec![0u8; body_len];
    if body_len > 0 {
        reader.read_exact(&mut body)?;
    }

    let headers: Vec<(&str, &str)> = raw_headers
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect();
    let request = Request {
        method,
        target: &target,
        headers,
        body_len,
    };
    let outcome = match check_request(&request) {
        Ok(route) => handle_checked(surface, route, &request, &body, now),
        Err(reject) => HttpOutcome::from_reject(reject),
    };
    outcome.write_to(stream)
}

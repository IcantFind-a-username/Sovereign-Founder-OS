//! Cross-port residuals against the v2 auth surface over a real socket.

#![cfg(feature = "owner-effect-fixture")]

use std::sync::{Arc, Mutex};

use sovereign_synthetic_owner_effect::{OwnerSurface, ORIGIN, SESSION_COOKIE};

#[path = "support/http.rs"]
mod http;

fn legitimate(extra: &str) -> String {
    format!(
        "POST /api/fixture/auth/logout HTTP/1.1\r\n\
         Host: localhost:7787\r\n\
         Origin: {ORIGIN}\r\n\
         Sec-Fetch-Site: same-origin\r\n\
         Sec-Fetch-Mode: cors\r\n\
         Content-Type: application/json\r\n\
         Cookie: {SESSION_COOKIE}=aa\r\n\
         X-SFO-CSRF: bb\r\n\
         Content-Length: 2\r\n\
         {extra}\r\n\
         {{}}"
    )
}

#[test]
fn cross_port_cookie_and_origin_residuals_are_refused() {
    let server = http::Guarded::start(Arc::new(Mutex::new(OwnerSurface::new())));

    let stolen_cookie = format!(
        "POST /api/fixture/auth/logout HTTP/1.1\r\n\
         Host: localhost:7787\r\n\
         Origin: {ORIGIN}\r\n\
         Sec-Fetch-Site: same-origin\r\n\
         Sec-Fetch-Mode: cors\r\n\
         Content-Type: application/json\r\n\
         Cookie: {SESSION_COOKIE}=stolen\r\n\
         Content-Length: 2\r\n\r\n\
         {{}}"
    );
    assert_eq!(server.send(&stolen_cookie), "403 Forbidden");

    for origin in [
        "http://localhost:9999",
        "http://127.0.0.1:7787",
        "https://localhost:7787",
        "null",
    ] {
        let raw = format!(
            "POST /api/fixture/auth/logout HTTP/1.1\r\n\
             Host: localhost:7787\r\n\
             Origin: {origin}\r\n\
             Sec-Fetch-Site: same-origin\r\n\
             Sec-Fetch-Mode: cors\r\n\
             Content-Type: application/json\r\n\
             Cookie: {SESSION_COOKIE}=stolen\r\n\
             X-SFO-CSRF: paired\r\n\
             Content-Length: 2\r\n\r\n\
             {{}}"
        );
        assert_eq!(
            server.send(&raw),
            "403 Forbidden",
            "a request from {origin} was accepted"
        );
    }

    let well_formed = legitimate("");
    let status = server.send(&well_formed);
    assert!(
        status.starts_with("403") || status.starts_with("200"),
        "a well-formed logout reached the handler, got {status}"
    );
}

#[test]
fn a_request_from_another_port_cannot_use_the_shared_rp() {
    let server = http::Guarded::start(Arc::new(Mutex::new(OwnerSurface::new())));
    let raw = "POST /api/fixture/auth/login/start HTTP/1.1\r\n\
         Host: localhost:7787\r\n\
         Origin: http://localhost:9999\r\n\
         Sec-Fetch-Site: same-origin\r\n\
         Sec-Fetch-Mode: cors\r\n\
         Content-Type: application/json\r\n\
         Content-Length: 2\r\n\r\n\
         {}";
    assert_eq!(server.send(raw), "403 Forbidden");
}

#[test]
fn probing_for_effect_and_product_routes_finds_nothing() {
    let server = http::Guarded::start(Arc::new(Mutex::new(OwnerSurface::new())));
    for target in [
        "/api/fixture/effect/dispatch",
        "/api/workspace/decide",
        "/fixture/logout",
        "/api/fixture/auth/logout/",
    ] {
        let raw = format!(
            "POST {target} HTTP/1.1\r\n\
             Host: localhost:7787\r\n\
             Origin: {ORIGIN}\r\n\
             Sec-Fetch-Site: same-origin\r\n\
             Sec-Fetch-Mode: cors\r\n\
             Content-Type: application/json\r\n\
             Cookie: {SESSION_COOKIE}=x\r\n\
             X-SFO-CSRF: y\r\n\
             Content-Length: 2\r\n\r\n\
             {{}}"
        );
        let status = server.send(&raw);
        assert!(
            status.starts_with("404") || status.starts_with("403"),
            "{target} returned {status}"
        );
    }
}

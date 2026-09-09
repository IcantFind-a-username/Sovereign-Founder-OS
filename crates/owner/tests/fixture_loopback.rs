//! What the fixture's HTTP surface accepts, and everything it refuses.
//!
//! The origin preflight measured three facts about loopback. Cookies are not
//! isolated by port. A WebAuthn RP ID is a host, so every port shares one.
//! And an IP address cannot be an RP ID at all. These tests are where those
//! measurements are held to.

#![cfg(feature = "owner-effect-fixture")]

use sovereign_owner::config::ORIGIN;
use sovereign_owner::http_guard::{check, Method, Reject, Request, Route, MAX_BODY_BYTES};

const HOST: &str = "localhost:7787";

fn post(target: &str) -> Request<'static> {
    Request {
        method: Method::Post,
        target: Box::leak(target.to_owned().into_boxed_str()),
        headers: vec![
            ("Host", HOST),
            ("Origin", ORIGIN),
            ("Sec-Fetch-Site", "same-origin"),
            ("Sec-Fetch-Mode", "cors"),
            ("Content-Type", "application/json"),
        ],
        body_len: 16,
    }
}

fn with(
    mut request: Request<'static>,
    name: &'static str,
    value: &'static str,
) -> Request<'static> {
    request
        .headers
        .retain(|(key, _)| !key.eq_ignore_ascii_case(name));
    request.headers.push((name, value));
    request
}

#[test]
fn the_known_routes_are_accepted() {
    assert_eq!(
        check(&post("/fixture/register/start")),
        Ok(Route::RegistrationStart)
    );
    assert_eq!(
        check(&post("/fixture/login/finish")),
        Ok(Route::LoginFinish)
    );
}

/// The route list is closed. Anything not on it is refused before a single
/// header is trusted, so there is no path that exists by accident.
#[test]
fn an_unknown_route_is_refused_before_anything_else() {
    for target in [
        "/fixture/register",
        "/fixture/register/start/",
        "/FIXTURE/register/start",
        "/api/fixture/register/start",
        "/fixture/admin",
        "/",
    ] {
        let mut request = post(target);
        // Everything else about this request is wrong too — the route check
        // must still be the one that fires.
        request.headers.clear();
        assert_eq!(
            check(&request),
            Err(Reject::UnknownRoute),
            "{target} was not refused as an unknown route"
        );
    }
}

/// The one GET is the synthetic warning shell, and it is not reachable by
/// POST — nor are the POST routes reachable by GET.
#[test]
fn methods_are_not_interchangeable() {
    let mut shell = post("/");
    shell.method = Method::Get;
    shell
        .headers
        .retain(|(k, _)| !k.eq_ignore_ascii_case("content-type"));
    shell
        .headers
        .retain(|(k, _)| !k.eq_ignore_ascii_case("sec-fetch-mode"));
    assert_eq!(check(&shell), Ok(Route::Shell));

    let mut as_get = post("/fixture/login/start");
    as_get.method = Method::Get;
    assert_eq!(check(&as_get), Err(Reject::UnknownRoute));
}

/// No route here takes a parameter, so a query string is either a mistake or
/// an attempt to pass something where it will not be looked at.
#[test]
fn a_query_string_is_refused() {
    for target in [
        "/fixture/login/start?token=abc",
        "/fixture/login/start?",
        "/fixture/login/start?csrf=stolen",
    ] {
        assert_eq!(
            check(&post(target)),
            Err(Reject::QueryNotAllowed),
            "{target} was accepted"
        );
    }
}

/// Exact, not by suffix or prefix, and not scheme-insensitively. Every one of
/// these is a near miss that a looser check would accept, and each is a
/// different origin.
#[test]
fn the_origin_must_match_exactly() {
    for origin in [
        "http://127.0.0.1:7787",
        "https://localhost:7787",
        "http://localhost:7788",
        "http://localhost",
        "http://localhost:7787/",
        "http://evil.localhost:7787",
        "http://localhost:7787.evil.test",
        "null",
        "",
    ] {
        assert_eq!(
            check(&with(post("/fixture/login/start"), "Origin", origin)),
            Err(Reject::WrongOrigin),
            "origin {origin:?} was accepted"
        );
    }
}

#[test]
fn a_missing_origin_is_refused() {
    let mut request = post("/fixture/login/start");
    request
        .headers
        .retain(|(k, _)| !k.eq_ignore_ascii_case("origin"));
    assert_eq!(check(&request), Err(Reject::WrongOrigin));
}

#[test]
fn the_host_must_match_the_compiled_origin() {
    for host in ["127.0.0.1:7787", "localhost:7788", "localhost", "evil.test"] {
        assert_eq!(
            check(&with(post("/fixture/login/start"), "Host", host)),
            Err(Reject::WrongHost),
            "host {host:?} was accepted"
        );
    }
}

/// Fetch metadata a same-origin fetch from our own page sends, and a
/// cross-site navigation or a form post does not.
#[test]
fn cross_site_fetch_metadata_is_refused() {
    for site in ["cross-site", "same-site", "none", ""] {
        assert_eq!(
            check(&with(post("/fixture/login/start"), "Sec-Fetch-Site", site)),
            Err(Reject::WrongFetchMetadata),
            "sec-fetch-site {site:?} was accepted"
        );
    }
    for mode in ["navigate", "no-cors", "same-origin"] {
        assert_eq!(
            check(&with(post("/fixture/login/start"), "Sec-Fetch-Mode", mode)),
            Err(Reject::WrongFetchMetadata),
            "sec-fetch-mode {mode:?} was accepted"
        );
    }
}

/// A form can be submitted cross-origin and cannot set a JSON content type,
/// so requiring one is a second, independent barrier.
#[test]
fn a_non_json_content_type_is_refused() {
    for content_type in [
        "application/x-www-form-urlencoded",
        "multipart/form-data",
        "text/plain",
        "application/json; charset=utf-8",
        "",
    ] {
        assert_eq!(
            check(&with(
                post("/fixture/login/start"),
                "Content-Type",
                content_type
            )),
            Err(Reject::WrongContentType),
            "content type {content_type:?} was accepted"
        );
    }
}

#[test]
fn an_oversized_body_is_refused() {
    let mut request = post("/fixture/login/start");
    request.body_len = MAX_BODY_BYTES + 1;
    assert_eq!(check(&request), Err(Reject::BodyTooLarge));

    request.body_len = MAX_BODY_BYTES;
    assert!(check(&request).is_ok(), "the cap itself must be allowed");
}

/// A guard that read the first of two `Origin` headers would be judging one
/// and letting a proxy or a browser quirk deliver the other.
#[test]
fn a_duplicated_security_header_is_refused() {
    for name in ["Host", "Origin", "Cookie", "X-SFO-CSRF", "Content-Type"] {
        let mut request = post("/fixture/login/start");
        // The header must be present *twice*. Pushing onto a request that
        // lacks it entirely — as Cookie and X-SFO-CSRF do — would add a
        // first one, and a different check would fire.
        request.headers.push((name, "a-value"));
        request.headers.push((name, "second-value"));
        assert_eq!(
            check(&request),
            Err(Reject::DuplicateHeader),
            "a duplicated {name} was accepted"
        );
    }
}

/// The preflight's finding, encoded. A cookie alone is what a cross-origin
/// page can cause a browser to send — and another port on this host can set
/// that cookie. The CSRF value is the half it cannot learn.
#[test]
fn a_cookie_without_its_csrf_partner_is_refused() {
    let cookie_only = with(
        post("/fixture/logout"),
        "Cookie",
        "__Host-sfo_fixture_session=x",
    );
    assert_eq!(check(&cookie_only), Err(Reject::IncompleteCredentials));

    let csrf_only = with(post("/fixture/logout"), "X-SFO-CSRF", "y");
    assert_eq!(check(&csrf_only), Err(Reject::IncompleteCredentials));

    let both = with(
        with(
            post("/fixture/logout"),
            "Cookie",
            "__Host-sfo_fixture_session=x",
        ),
        "X-SFO-CSRF",
        "y",
    );
    assert_eq!(check(&both), Ok(Route::Logout));
}

/// Registration and login must work without a session — there would be no way
/// to obtain one otherwise — and nothing else may.
#[test]
fn only_the_pre_session_routes_run_without_credentials() {
    for (target, route) in [
        ("/fixture/register/start", Route::RegistrationStart),
        ("/fixture/register/finish", Route::RegistrationFinish),
        ("/fixture/login/start", Route::LoginStart),
        ("/fixture/login/finish", Route::LoginFinish),
    ] {
        assert_eq!(check(&post(target)), Ok(route));
        assert!(route.is_pre_session());
    }

    assert_eq!(
        check(&post("/fixture/logout")),
        Err(Reject::IncompleteCredentials),
        "logout ran without a session"
    );
    assert!(!Route::Logout.is_pre_session());
}

/// There is no way to change the compiled origin. An origin a caller can set
/// is not a boundary.
#[test]
fn no_api_changes_the_origin() {
    let source = include_str!("../src/http_guard.rs");
    let code: String = source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    for setter in [
        "fn set_origin",
        "fn with_origin",
        "origin: String",
        "pub static mut",
    ] {
        assert!(
            !code.contains(setter),
            "the guard exposes {setter:?}, so the origin is configurable"
        );
    }
    assert!(
        code.contains("use crate::config::ORIGIN"),
        "the origin must be the compiled one"
    );
}

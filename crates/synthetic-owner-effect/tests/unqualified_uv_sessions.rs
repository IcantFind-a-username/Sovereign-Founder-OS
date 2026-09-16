//! v01-D03: unqualified UV, session, CSRF, origin/RP, ceremonies.
//!
//! Design Accept ≠ product Current. Not owner admission.

#![cfg(feature = "owner-effect-fixture")]

use std::time::{Duration, Instant};

use serde_json::{json, Value};
use sovereign_synthetic_owner_effect::{
    check_request, FixtureOwner, FixtureRoute, Method, OwnerError, OwnerSurface, Reject, Request,
    SessionTokens, ABSOLUTE_LIFETIME, CSRF_HEADER, IDLE_TIMEOUT, MAX_BODY_BYTES, ORIGIN, RP_ID,
    SESSION_COOKIE, TOKEN_LEN,
};
use uuid::Uuid;

#[path = "support/root.rs"]
mod root;

fn counter() -> impl FnMut() -> [u8; TOKEN_LEN] {
    let mut next = 0u8;
    move || {
        next = next.wrapping_add(1);
        [next; TOKEN_LEN]
    }
}

fn post(target: &'static str) -> Request<'static> {
    Request {
        method: Method::Post,
        target,
        headers: vec![
            ("Host", "localhost:7787"),
            ("Origin", ORIGIN),
            ("Sec-Fetch-Site", "same-origin"),
            ("Sec-Fetch-Mode", "cors"),
            ("Content-Type", "application/json"),
        ],
        body_len: 16,
    }
}

fn with<'a>(mut request: Request<'a>, name: &'a str, value: &'a str) -> Request<'a> {
    request
        .headers
        .retain(|(key, _)| !key.eq_ignore_ascii_case(name));
    request.headers.push((name, value));
    request
}

fn finish_body(ceremony_id: &str, credential: &str, handle: Uuid, user_verified: bool) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "ceremony_id": ceremony_id,
        "credential_id": hex::encode(credential.as_bytes()),
        "user_handle": handle.to_string(),
        "user_verified": user_verified,
    }))
    .unwrap()
}

fn register(
    surface: &mut OwnerSurface,
    now: Instant,
    random: impl FnMut() -> [u8; TOKEN_LEN],
    credential: &str,
    handle: Uuid,
    user_verified: bool,
) -> (u16, Value, Option<SessionTokens>) {
    let start = surface.dispatch(FixtureRoute::RegisterStart, b"{}", None, now, counter());
    let Some(ceremony) = start.body["ceremony_id"].as_str().map(str::to_owned) else {
        return (start.status, start.body, None);
    };
    let body = finish_body(&ceremony, credential, handle, user_verified);
    let finish = surface.dispatch(FixtureRoute::RegisterFinish, &body, None, now, random);
    (finish.status, finish.body, finish.issued)
}

/// First valid UV wins an empty registry. The outcome says it is not owner
/// admission: a same-account caller is indistinguishable from the founder.
#[test]
fn first_writer_wins_is_non_admission() {
    let mut surface = OwnerSurface::new();
    let now = Instant::now();
    let handle = Uuid::new_v4();
    let (status, body, issued) = register(&mut surface, now, counter(), "first", handle, true);
    assert_eq!(status, 200);
    let summary = body["enrolment"].as_str().unwrap();
    assert!(
        summary.contains("not owner admission"),
        "the outcome must say what it is not: {summary}"
    );
    assert!(issued.is_some());

    let second = register(&mut surface, now, counter(), "second", Uuid::new_v4(), true);
    assert_eq!(second.0, 403);
    assert_eq!(
        second.1["error"].as_str(),
        Some(OwnerError::AlreadyRegistered.code())
    );
}

#[test]
fn registration_and_login_require_user_verification() {
    let mut surface = OwnerSurface::new();
    let now = Instant::now();
    let (status, body, issued) = register(&mut surface, now, counter(), "c", Uuid::new_v4(), false);
    assert_eq!(status, 403);
    assert_eq!(
        body["error"].as_str(),
        Some(OwnerError::UserVerificationMissing.code())
    );
    assert!(issued.is_none());
    assert!(surface.registry().is_empty());

    let handle = Uuid::new_v4();
    let (status, _, _) = register(&mut surface, now, counter(), "c", handle, true);
    assert_eq!(status, 200);

    let start = surface.dispatch(FixtureRoute::LoginStart, b"{}", None, now, counter());
    let ceremony = start.body["ceremony_id"].as_str().unwrap().to_owned();
    let body = finish_body(&ceremony, "c", handle, false);
    let finish = surface.dispatch(FixtureRoute::LoginFinish, &body, None, now, counter());
    assert_eq!(finish.status, 403);
    assert_eq!(
        finish.body["error"].as_str(),
        Some(OwnerError::UserVerificationMissing.code())
    );
}

#[test]
fn exact_origin_and_rp_are_compiled() {
    assert_eq!(ORIGIN, "http://localhost:7787");
    assert_eq!(RP_ID, "localhost");

    for origin in [
        "http://127.0.0.1:7787",
        "https://localhost:7787",
        "http://localhost:7788",
        "http://localhost",
        "http://localhost:7787/",
        "null",
    ] {
        assert_eq!(
            check_request(&with(
                post("/api/fixture/auth/login/start"),
                "Origin",
                origin
            )),
            Err(Reject::WrongOrigin),
            "origin {origin:?} was accepted"
        );
    }

    let ok = check_request(&post("/api/fixture/auth/login/start"));
    assert_eq!(ok, Ok(FixtureRoute::LoginStart));
}

#[test]
fn ceremonies_are_one_use_and_expire_at_300s() {
    let mut surface = OwnerSurface::new();
    let start_at = Instant::now();
    let started = surface.dispatch(
        FixtureRoute::RegisterStart,
        b"{}",
        None,
        start_at,
        counter(),
    );
    let ceremony = started.body["ceremony_id"].as_str().unwrap().to_owned();
    let handle = Uuid::new_v4();

    let expired = surface.dispatch(
        FixtureRoute::RegisterFinish,
        &finish_body(&ceremony, "c", handle, true),
        None,
        start_at + Duration::from_secs(300),
        counter(),
    );
    assert_eq!(
        expired.body["error"].as_str(),
        Some(OwnerError::CeremonyExpired.code())
    );
    assert!(surface.registry().is_empty());

    let started = surface.dispatch(
        FixtureRoute::RegisterStart,
        b"{}",
        None,
        start_at,
        counter(),
    );
    let ceremony = started.body["ceremony_id"].as_str().unwrap().to_owned();
    let missing_uv = surface.dispatch(
        FixtureRoute::RegisterFinish,
        &finish_body(&ceremony, "c", handle, false),
        None,
        start_at,
        counter(),
    );
    assert_eq!(
        missing_uv.body["error"].as_str(),
        Some(OwnerError::UserVerificationMissing.code())
    );
    let retry = surface.dispatch(
        FixtureRoute::RegisterFinish,
        &finish_body(&ceremony, "c", handle, true),
        None,
        start_at,
        counter(),
    );
    assert_eq!(
        retry.body["error"].as_str(),
        Some(OwnerError::UnknownCeremony.code()),
        "a failed finish left its challenge available"
    );
}

#[test]
fn cookie_and_csrf_are_independent() {
    let mut surface = OwnerSurface::new();
    let now = Instant::now();
    let (_, _, issued) = register(&mut surface, now, counter(), "c", Uuid::new_v4(), true);
    let tokens = issued.unwrap();
    assert_ne!(tokens.cookie, tokens.csrf);

    let cookie_only = SessionTokens {
        cookie: tokens.cookie,
        csrf: [0xFF; TOKEN_LEN],
    };
    assert_eq!(
        surface
            .sessions_mut()
            .authenticate(&cookie_only, now)
            .unwrap_err(),
        sovereign_synthetic_owner_effect::SessionError::CsrfMismatch
    );

    let cookie_header = format!("{}={}", SESSION_COOKIE, hex::encode(tokens.cookie));
    let logout = with(post("/api/fixture/auth/logout"), "Cookie", &cookie_header);
    assert_eq!(check_request(&logout), Err(Reject::IncompleteCredentials));
    let both = with(logout, CSRF_HEADER, "00");
    // Header presence is the middleware check; the values are judged later.
    assert_eq!(check_request(&both), Ok(FixtureRoute::Logout));
}

#[test]
fn absolute_and_idle_expiry_are_enforced() {
    let mut surface = OwnerSurface::new();
    let start = Instant::now();
    let (_, _, issued) = register(&mut surface, start, counter(), "c", Uuid::new_v4(), true);
    let tokens = issued.unwrap();

    let step = IDLE_TIMEOUT / 2;
    let mut now = start;
    while now.duration_since(start) + step < ABSOLUTE_LIFETIME {
        now += step;
        assert!(surface.sessions_mut().authenticate(&tokens, now).is_ok());
    }
    assert_eq!(
        surface
            .sessions_mut()
            .authenticate(&tokens, start + ABSOLUTE_LIFETIME)
            .unwrap_err(),
        sovereign_synthetic_owner_effect::SessionError::Expired
    );

    let mut fresh = OwnerSurface::new();
    let (_, _, issued) = register(&mut fresh, start, counter(), "c", Uuid::new_v4(), true);
    let tokens = issued.unwrap();
    assert_eq!(
        fresh
            .sessions_mut()
            .authenticate(&tokens, start + IDLE_TIMEOUT)
            .unwrap_err(),
        sovereign_synthetic_owner_effect::SessionError::Idle
    );
}

#[test]
fn logout_and_restart_invalidate_sessions() {
    let mut surface = OwnerSurface::new();
    let now = Instant::now();
    let handle = Uuid::new_v4();
    let (_, _, issued) = register(&mut surface, now, counter(), "c", handle, true);
    let tokens = issued.unwrap();

    let login = surface.dispatch(FixtureRoute::LoginStart, b"{}", None, now, counter());
    let ceremony = login.body["ceremony_id"].as_str().unwrap().to_owned();

    let logout = surface.dispatch(FixtureRoute::Logout, b"{}", Some(&tokens), now, counter());
    assert_eq!(logout.status, 200);
    assert!(surface.sessions_mut().authenticate(&tokens, now).is_err());

    let after_logout = surface.dispatch(
        FixtureRoute::LoginFinish,
        &finish_body(&ceremony, "c", handle, true),
        None,
        now,
        counter(),
    );
    assert_eq!(
        after_logout.body["error"].as_str(),
        Some(OwnerError::UnknownCeremony.code()),
        "logout left a pending ceremony"
    );

    let dir = tempfile::tempdir().unwrap();
    let root = root::marked_root(dir.path());
    let mut first = FixtureOwner::boot(&root).unwrap();
    let (_, _, issued) = register(
        first.surface_mut(),
        now,
        counter(),
        "c",
        Uuid::new_v4(),
        true,
    );
    let tokens = issued.unwrap();
    drop(first);
    let mut restarted = FixtureOwner::boot(&root).unwrap();
    assert!(
        restarted
            .surface_mut()
            .sessions_mut()
            .authenticate(&tokens, now)
            .is_err(),
        "a session survived a restart"
    );
}

#[test]
fn stored_credential_allowlist_and_destructive_loss() {
    let mut surface = OwnerSurface::new();
    let now = Instant::now();
    let handle = Uuid::new_v4();
    let (status, _, _) = register(&mut surface, now, counter(), "the-one", handle, true);
    assert_eq!(status, 200);
    let login = surface.dispatch(FixtureRoute::LoginStart, b"{}", None, now, counter());
    let expected = hex::encode("the-one".as_bytes());
    assert_eq!(
        login.body["allow_credential"].as_str(),
        Some(expected.as_str())
    );

    let source = include_str!("../src/owner_surface.rs");
    for escape in ["pub fn reset", "pub fn clear", "pub fn remove_credential"] {
        assert!(
            !source.contains(escape),
            "the surface exposes {escape}, a second way to win"
        );
    }
}

#[test]
fn empty_real_mechanism_matrix_is_allowed() {
    let matrix = include_str!("../../../docs/security/owner-auth-mechanism-matrix.md");
    assert!(matrix.contains("protocol_fixture_only"));
    assert!(matrix.contains("An empty real matrix is allowed"));
    assert!(matrix.contains("*Empty.*"));
    assert!(
        !matrix.contains("mechanism_qualified_only` | darwin")
            && !matrix.contains("mechanism_qualified_only` | linux"),
        "a real matrix row appeared; qualification is still a separate ticket"
    );
}

#[test]
fn v2_auth_routes_are_closed() {
    assert_eq!(
        check_request(&post("/api/fixture/auth/register/start")),
        Ok(FixtureRoute::RegisterStart)
    );
    for target in [
        "/fixture/register/start",
        "/api/fixture/effect/dispatch",
        "/api/workspace/decide",
        "/api/fixture/auth/register/start/",
    ] {
        let mut request = post(Box::leak(target.to_owned().into_boxed_str()));
        request.headers.clear();
        assert_eq!(
            check_request(&request),
            Err(Reject::UnknownRoute),
            "{target} was not refused"
        );
    }
    let mut oversized = post("/api/fixture/auth/login/start");
    oversized.body_len = MAX_BODY_BYTES + 1;
    assert_eq!(check_request(&oversized), Err(Reject::BodyTooLarge));
}

//! What the fixture's HTTP surface accepts, and everything it refuses.
//!
//! The origin preflight measured three facts about loopback that a design
//! cannot assume away, and this module is where they turn into code.
//!
//! Cookies are not isolated by port: another server on this host can set and
//! overwrite the session cookie. So a cookie alone is never a session, and a
//! request must also carry the CSRF value the page was given — something a
//! cross-origin page can cause a browser to *send* but cannot cause it to
//! *learn*.
//!
//! A WebAuthn RP ID is a host, so every port shares one. What separates them
//! is the origin the browser records, which means the `Origin` header is
//! checked exactly and not by suffix, prefix or scheme-insensitively. A
//! near-miss origin is a different origin.
//!
//! And an IP address cannot be an RP ID at all, so the compiled origin names
//! `localhost` while the socket binds loopback. There is no API to change it:
//! an origin a caller can set is not a boundary.
//!
//! The route list is closed. A request that does not name one of these routes
//! is refused before anything else is considered, so there is no path that
//! exists by accident, no alias, and no way to smuggle a route in a query
//! parameter.

use crate::config::ORIGIN;

/// Every route, and there is no other. An enum rather than a string match, so
/// adding one is a change a reviewer sees.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    /// The synthetic warning shell. The only GET.
    Shell,
    RegistrationStart,
    RegistrationFinish,
    LoginStart,
    LoginFinish,
    Logout,
}

impl Route {
    const ALL: &'static [(Route, &'static str, Method)] = &[
        (Route::Shell, "/", Method::Get),
        (
            Route::RegistrationStart,
            "/fixture/register/start",
            Method::Post,
        ),
        (
            Route::RegistrationFinish,
            "/fixture/register/finish",
            Method::Post,
        ),
        (Route::LoginStart, "/fixture/login/start", Method::Post),
        (Route::LoginFinish, "/fixture/login/finish", Method::Post),
        (Route::Logout, "/fixture/logout", Method::Post),
    ];

    fn lookup(method: Method, path: &str) -> Option<Route> {
        Self::ALL
            .iter()
            .find(|(_, route_path, route_method)| *route_path == path && *route_method == method)
            .map(|(route, _, _)| *route)
    }

    /// Whether this route runs before a session exists. Registration and
    /// login must, or there would be no way to obtain one; everything else
    /// must not.
    pub fn is_pre_session(self) -> bool {
        matches!(
            self,
            Route::Shell
                | Route::RegistrationStart
                | Route::RegistrationFinish
                | Route::LoginStart
                | Route::LoginFinish
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Get,
    Post,
}

/// The bytes a caller may send. Small on purpose: nothing this surface
/// accepts is large, and a generous cap is a place to put a payload.
pub const MAX_BODY_BYTES: usize = 8 * 1024;

/// One request, reduced to what the guard judges.
#[derive(Debug, Clone)]
pub struct Request<'a> {
    pub method: Method,
    /// The raw target, query string included. A guard that saw only the path
    /// could not refuse a token smuggled in a query parameter.
    pub target: &'a str,
    /// Every header, in order, including repeats — a guard given a map has
    /// already lost the duplicate it needs to refuse.
    pub headers: Vec<(&'a str, &'a str)>,
    pub body_len: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reject {
    UnknownRoute,
    /// A query string on a surface that has no use for one.
    QueryNotAllowed,
    WrongHost,
    WrongOrigin,
    /// `Sec-Fetch-Site` or `Sec-Fetch-Mode` is not what a same-origin fetch
    /// from our own page sends.
    WrongFetchMetadata,
    WrongContentType,
    BodyTooLarge,
    /// The same header appears twice.
    DuplicateHeader,
    /// A cookie with no CSRF value, or the reverse.
    IncompleteCredentials,
}

/// The value `Host` must equal. Derived from the compiled origin rather than
/// written twice, so the two cannot drift.
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

/// Judge one request.
///
/// The order is deliberate: the route first, so an unknown path is refused
/// before any header is trusted; then the headers that say who is calling;
/// then the body. A guard that read the body before deciding the request was
/// even addressed to it would have accepted work from a stranger.
pub fn check(request: &Request<'_>) -> Result<Route, Reject> {
    let (path, query) = match request.target.split_once('?') {
        Some((path, query)) => (path, Some(query)),
        None => (request.target, None),
    };
    let route = Route::lookup(request.method, path).ok_or(Reject::UnknownRoute)?;
    // No route here takes a parameter, so a query string is either a mistake
    // or an attempt to pass something where it will not be looked at.
    if query.is_some() {
        return Err(Reject::QueryNotAllowed);
    }

    // Duplicates first: a guard that read the first of two `Origin` headers
    // would be judging one and letting a proxy or a browser quirk deliver the
    // other.
    for name in ["host", "origin", "cookie", "x-sfo-csrf", "content-type"] {
        if count(request, name) > 1 {
            return Err(Reject::DuplicateHeader);
        }
    }

    if header(request, "host") != Some(expected_host()) {
        return Err(Reject::WrongHost);
    }
    // Exact, not by suffix or prefix, and not scheme-insensitively.
    if header(request, "origin") != Some(ORIGIN) {
        return Err(Reject::WrongOrigin);
    }
    // What our own page's fetch sends, and what a cross-site navigation or a
    // form post does not.
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

    // A cookie without its CSRF partner is what a cross-origin page can
    // cause; a CSRF value without a cookie names no session. Both or neither.
    let has_cookie = header(request, "cookie").is_some();
    let has_csrf = header(request, "x-sfo-csrf").is_some();
    if has_cookie != has_csrf {
        return Err(Reject::IncompleteCredentials);
    }
    // Routes that exist to create a session are the only ones that may arrive
    // without one.
    if !has_cookie && !route.is_pre_session() {
        return Err(Reject::IncompleteCredentials);
    }

    Ok(route)
}

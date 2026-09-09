use std::sync::Mutex;

use crate::catalog::{CatalogEntry, CATALOG};
use crate::domain::{
    PlaygroundAction, PlaygroundReadModel, PlaygroundSession, PlaygroundTeachingReadModel,
};

pub(crate) struct PlaygroundHttpHandler {
    session: Mutex<PlaygroundSession>,
}

pub(crate) struct HttpRequest<'a> {
    pub(crate) method: &'a str,
    pub(crate) target: &'a str,
    pub(crate) headers: &'a [(&'a str, &'a str)],
    pub(crate) body: &'a [u8],
}

pub(crate) enum AssetRoute {
    Index,
    Styles,
    I18n,
    App,
    ConsultantUi,
    Favicon,
}

// Keep the bounded, fixed DTO snapshot inline, as required by the handler contract.
#[allow(clippy::large_enum_variant)]
pub(crate) enum HandlerOutcome {
    Json(HttpResponse),
    Asset(AssetRoute),
}

pub(crate) struct HttpResponse {
    pub(crate) status: u16,
    pub(crate) headers: &'static [(&'static str, &'static str)],
    pub(crate) body: ResponseBody,
}

#[derive(serde::Serialize)]
#[serde(untagged)]
// Keep the bounded, fixed DTO snapshot inline, as required by the handler contract.
#[allow(clippy::large_enum_variant)]
pub(crate) enum ResponseBody {
    State(StateResponse),
    Error(ErrorResponse),
}

#[derive(serde::Serialize)]
pub(crate) struct StateResponse {
    profile: &'static str,
    real_data_enabled: bool,
    persistence: &'static str,
    state: PlaygroundReadModel,
    catalog: &'static [CatalogEntry; 32],
    teaching: PlaygroundTeachingReadModel,
}

#[derive(serde::Serialize)]
pub(crate) struct ErrorResponse {
    profile: &'static str,
    real_data_enabled: bool,
    persistence: &'static str,
    error: ErrorCode,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum ErrorCode {
    InvalidConfiguration,
    InvalidHost,
    OriginForbidden,
    InvalidTarget,
    NotFound,
    MethodNotAllowed,
    PayloadTooLarge,
    UnexpectedBody,
    UnsupportedMediaType,
    InvalidActionRequest,
    SessionUnavailable,
}

const JSON_HEADERS: &[(&str, &str)] = &[
    ("Content-Type", "application/json; charset=utf-8"),
    ("Cache-Control", "no-store"),
    ("X-Content-Type-Options", "nosniff"),
];
const ALLOW_GET_HEADERS: &[(&str, &str)] = &[
    ("Content-Type", "application/json; charset=utf-8"),
    ("Cache-Control", "no-store"),
    ("X-Content-Type-Options", "nosniff"),
    ("Allow", "GET"),
];
const ALLOW_POST_HEADERS: &[(&str, &str)] = &[
    ("Content-Type", "application/json; charset=utf-8"),
    ("Cache-Control", "no-store"),
    ("X-Content-Type-Options", "nosniff"),
    ("Allow", "POST"),
];

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ActionRequest {
    action: WireAction,
}

#[derive(serde::Deserialize)]
#[serde(try_from = "String")]
enum WireAction {
    CorrectOfferPrice,
    PromoteAcmeToCustomer,
    ShowReportingSearch,
    Reset,
}

impl TryFrom<String> for WireAction {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "CorrectOfferPrice" => Ok(Self::CorrectOfferPrice),
            "PromoteAcmeToCustomer" => Ok(Self::PromoteAcmeToCustomer),
            "ShowReportingSearch" => Ok(Self::ShowReportingSearch),
            "Reset" => Ok(Self::Reset),
            _ => Err("invalid action"),
        }
    }
}

impl WireAction {
    fn into_domain(self) -> PlaygroundAction {
        match self {
            Self::CorrectOfferPrice => PlaygroundAction::CorrectOfferPrice,
            Self::PromoteAcmeToCustomer => PlaygroundAction::PromoteAcmeToCustomer,
            Self::ShowReportingSearch => PlaygroundAction::ShowReportingSearch,
            Self::Reset => PlaygroundAction::Reset,
        }
    }
}

enum Route {
    Asset(AssetRoute),
    State,
    Action,
}

impl Route {
    fn from_target(target: &str) -> Option<Self> {
        match target {
            "/" => Some(Self::Asset(AssetRoute::Index)),
            "/assets/styles.css" => Some(Self::Asset(AssetRoute::Styles)),
            "/assets/i18n.js" => Some(Self::Asset(AssetRoute::I18n)),
            "/assets/app.js" => Some(Self::Asset(AssetRoute::App)),
            "/assets/consultant-ui.js" => Some(Self::Asset(AssetRoute::ConsultantUi)),
            "/favicon.svg" => Some(Self::Asset(AssetRoute::Favicon)),
            "/api/playground/consultant" => Some(Self::State),
            "/api/playground/consultant/action" => Some(Self::Action),
            _ => None,
        }
    }

    fn method(&self) -> &'static str {
        match self {
            Self::Action => "POST",
            Self::Asset(_) | Self::State => "GET",
        }
    }
}

impl PlaygroundHttpHandler {
    pub(crate) fn new() -> Self {
        Self {
            session: Mutex::new(PlaygroundSession::new()),
        }
    }

    pub(crate) fn handle(&self, request: HttpRequest<'_>, bound_port: u16) -> HandlerOutcome {
        if bound_port == 0 {
            return reject(ErrorCode::InvalidConfiguration);
        }
        let expected_host = format!("127.0.0.1:{bound_port}");
        match unique_header(request.headers, "Host") {
            Ok(Some(host))
                if host == expected_host || (bound_port == 80 && host == "127.0.0.1") => {}
            _ => return reject(ErrorCode::InvalidHost),
        }
        let expected_origin = format!("http://127.0.0.1:{bound_port}");
        match unique_header(request.headers, "Origin") {
            Ok(None) => {}
            Ok(Some(origin))
                if origin == expected_origin
                    || (bound_port == 80 && origin == "http://127.0.0.1") => {}
            _ => return reject(ErrorCode::OriginForbidden),
        }
        if !request.target.starts_with('/')
            || request
                .target
                .contains(['?', '#', '%', '\\', '\r', '\n', '\0'])
            || request.target.contains("//")
            || request
                .target
                .split('/')
                .any(|part| part == "." || part == "..")
        {
            return reject(ErrorCode::InvalidTarget);
        }
        let Some(route) = Route::from_target(request.target) else {
            return reject(ErrorCode::NotFound);
        };
        let method = route.method();
        if request.method != method {
            return HandlerOutcome::Json(HttpResponse {
                status: 405,
                headers: if method == "POST" {
                    ALLOW_POST_HEADERS
                } else {
                    ALLOW_GET_HEADERS
                },
                body: ErrorCode::MethodNotAllowed.body(),
            });
        }
        if request.body.len() > 256 {
            return reject(ErrorCode::PayloadTooLarge);
        }
        if request
            .headers
            .iter()
            .any(|(name, _)| name.eq_ignore_ascii_case("Content-Encoding"))
        {
            return reject(ErrorCode::UnsupportedMediaType);
        }
        let action = if method == "GET" {
            if !request.body.is_empty() {
                return reject(ErrorCode::UnexpectedBody);
            }
            None
        } else {
            match unique_header(request.headers, "Content-Type") {
                Ok(Some(media_type))
                    if media_type.eq_ignore_ascii_case("application/json")
                        || media_type.eq_ignore_ascii_case("application/json; charset=utf-8") => {}
                _ => return reject(ErrorCode::UnsupportedMediaType),
            }
            // Serde structs also accept sequences; this endpoint requires a JSON object.
            let first_json_byte = request
                .body
                .iter()
                .copied()
                .find(|byte| !matches!(*byte, b' ' | b'\t' | b'\r' | b'\n'));
            if first_json_byte != Some(b"{"[0]) {
                return reject(ErrorCode::InvalidActionRequest);
            }
            let parsed: ActionRequest = match serde_json::from_slice(request.body) {
                Ok(parsed) => parsed,
                Err(_) => return reject(ErrorCode::InvalidActionRequest),
            };
            Some(parsed.action.into_domain())
        };
        if let Route::Asset(asset) = route {
            return HandlerOutcome::Asset(asset);
        }
        let mut session = match self.session.lock() {
            Ok(session) => session,
            Err(_) => return reject(ErrorCode::SessionUnavailable),
        };
        if let Some(action) = action {
            session.apply(action);
        }
        HandlerOutcome::Json(HttpResponse {
            status: 200,
            headers: JSON_HEADERS,
            body: ResponseBody::State(StateResponse::snapshot(&session)),
        })
    }
}

fn unique_header<'a>(headers: &[(&'a str, &'a str)], name: &str) -> Result<Option<&'a str>, ()> {
    let mut values = headers
        .iter()
        .filter(|(header, _)| header.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.trim_matches([' ', '\t']));
    let first = values.next();
    if values.next().is_some() {
        Err(())
    } else {
        Ok(first)
    }
}

impl StateResponse {
    fn snapshot(session: &PlaygroundSession) -> Self {
        Self {
            profile: "synthetic_playground",
            real_data_enabled: false,
            persistence: "none",
            state: session.read_model(),
            catalog: &CATALOG,
            teaching: session.teaching_read_model(),
        }
    }
}

impl ErrorCode {
    fn body(self) -> ResponseBody {
        ResponseBody::Error(ErrorResponse {
            profile: "synthetic_playground",
            real_data_enabled: false,
            persistence: "none",
            error: self,
        })
    }
}

fn reject(error: ErrorCode) -> HandlerOutcome {
    let status = match error {
        ErrorCode::InvalidConfiguration => 500,
        ErrorCode::InvalidHost
        | ErrorCode::InvalidTarget
        | ErrorCode::UnexpectedBody
        | ErrorCode::InvalidActionRequest => 400,
        ErrorCode::OriginForbidden => 403,
        ErrorCode::NotFound => 404,
        ErrorCode::MethodNotAllowed => 405,
        ErrorCode::PayloadTooLarge => 413,
        ErrorCode::UnsupportedMediaType => 415,
        ErrorCode::SessionUnavailable => 503,
    };
    HandlerOutcome::Json(HttpResponse {
        status,
        headers: JSON_HEADERS,
        body: error.body(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};
    use std::sync::Arc;
    use std::thread;

    const PORT: u16 = 7788;
    const STATE: &str = "/api/playground/consultant";
    const ACTION: &str = "/api/playground/consultant/action";
    const HOST: &[(&str, &str)] = &[("Host", "127.0.0.1:7788")];
    const POST_HEADERS: &[(&str, &str)] = &[
        ("Host", "127.0.0.1:7788"),
        ("Content-Type", "application/json"),
    ];
    const RESET: &[u8] = br#"{"action":"Reset"}"#;
    const CORRECT: &[u8] = br#"{"action":"CorrectOfferPrice"}"#;
    const PROMOTE: &[u8] = br#"{"action":"PromoteAcmeToCustomer"}"#;
    const SEARCH: &[u8] = br#"{"action":"ShowReportingSearch"}"#;
    const LARGE: &[u8] = &[b'x'; 257];
    const ASSETS: [(&str, AssetRoute); 6] = [
        ("/", AssetRoute::Index),
        ("/assets/styles.css", AssetRoute::Styles),
        ("/assets/i18n.js", AssetRoute::I18n),
        ("/assets/app.js", AssetRoute::App),
        ("/assets/consultant-ui.js", AssetRoute::ConsultantUi),
        ("/favicon.svg", AssetRoute::Favicon),
    ];

    fn request<'a>(method: &'a str, target: &'a str, body: &'a [u8]) -> HttpRequest<'a> {
        HttpRequest {
            method,
            target,
            headers: POST_HEADERS,
            body,
        }
    }

    fn with_headers<'a>(headers: &'a [(&'a str, &'a str)]) -> HttpRequest<'a> {
        HttpRequest {
            headers,
            ..request("POST", ACTION, RESET)
        }
    }

    fn response(handler: &PlaygroundHttpHandler, req: HttpRequest<'_>, port: u16) -> HttpResponse {
        let HandlerOutcome::Json(response) = handler.handle(req, port) else {
            panic!("expected JSON, received asset dispatch")
        };
        response
    }

    fn assert_headers(response: &HttpResponse, allow: Option<&str>) {
        let mut expected = vec![
            ("Content-Type", "application/json; charset=utf-8"),
            ("Cache-Control", "no-store"),
            ("X-Content-Type-Options", "nosniff"),
        ];
        if let Some(method) = allow {
            expected.push(("Allow", method));
        }
        assert_eq!(response.headers, expected);
    }

    fn value(response: HttpResponse) -> Value {
        let bytes = serde_json::to_vec(&response.body).expect("detached response serializes");
        serde_json::from_slice(&bytes).expect("serialized JSON")
    }

    fn success(handler: &PlaygroundHttpHandler, req: HttpRequest<'_>, port: u16) -> Value {
        let response = response(handler, req, port);
        assert_eq!(response.status, 200);
        assert_headers(&response, None);
        let guard = handler
            .session
            .try_lock()
            .expect("handle released session lock");
        // Serialization must remain possible while the caller holds the session lock.
        let value = value(response);
        drop(guard);
        value
    }

    fn get(handler: &PlaygroundHttpHandler) -> Value {
        success(handler, request("GET", STATE, b""), PORT)
    }

    fn post(handler: &PlaygroundHttpHandler, body: &[u8]) -> Value {
        success(handler, request("POST", ACTION, body), PORT)
    }

    fn expected(session: &PlaygroundSession) -> Value {
        // The accepted domain DTO tests pin their complete content; reuse both
        // projections and the canonical catalog instead of copying catalog prose.
        json!({
            "profile": "synthetic_playground", "real_data_enabled": false,
            "persistence": "none", "state": session.read_model(),
            "catalog": CATALOG, "teaching": session.teaching_read_model(),
        })
    }

    fn modified_handler() -> PlaygroundHttpHandler {
        let handler = PlaygroundHttpHandler::new();
        post(&handler, CORRECT);
        post(&handler, PROMOTE);
        assert_ne!(get(&handler), expected(&PlaygroundSession::new()));
        handler
    }

    fn assert_error(response: HttpResponse, status: u16, code: &str, allow: Option<&str>) {
        assert_eq!(response.status, status, "{code}");
        assert_headers(&response, allow);
        assert_eq!(
            value(response),
            json!({
                "profile": "synthetic_playground", "real_data_enabled": false,
                "persistence": "none", "error": code,
            })
        );
    }

    fn fails_unchanged(
        handler: &PlaygroundHttpHandler,
        req: HttpRequest<'_>,
        port: u16,
        status: u16,
        code: &str,
        allow: Option<&str>,
    ) {
        let before = get(handler);
        let response = response(handler, req, port);
        let guard = handler
            .session
            .try_lock()
            .expect("error released session lock");
        assert_error(response, status, code, allow);
        drop(guard);
        assert_eq!(get(handler), before, "rejection changed state: {code}");
    }

    fn poison(handler: &Arc<PlaygroundHttpHandler>) {
        let shared = Arc::clone(handler);
        thread::spawn(move || {
            let _guard = shared.session.lock().expect("initially healthy");
            panic!("intentional real mutex poison");
        })
        .join()
        .expect_err("lock holder must panic");
        assert!(handler.session.is_poisoned());
    }

    #[test]
    fn http_get_and_actions_return_one_complete_typed_snapshot() {
        // Exercise search/reset in every reachable state and both mutation orders.
        for sequence in [
            vec![],
            vec![PROMOTE],
            vec![CORRECT],
            vec![CORRECT, PROMOTE],
            vec![PROMOTE, CORRECT],
        ] {
            let handler = PlaygroundHttpHandler::new();
            let other = PlaygroundHttpHandler::new();
            let mut oracle = PlaygroundSession::new();
            let initial = expected(&oracle);
            assert_eq!(get(&handler), initial);
            for body in sequence {
                let action = if body == CORRECT {
                    PlaygroundAction::CorrectOfferPrice
                } else {
                    PlaygroundAction::PromoteAcmeToCustomer
                };
                oracle.apply(action);
                let updated = post(&handler, body);
                assert_eq!(updated, expected(&oracle));
                assert_eq!(get(&handler), updated);
                assert_eq!(post(&handler, body), updated, "mutation is idempotent");
                assert_ne!(updated, initial);
                assert_eq!(
                    get(&other),
                    initial,
                    "other stays initial while first is modified"
                );
            }
            let before = get(&handler);
            for _ in 0..2 {
                assert_eq!(post(&handler, SEARCH), before);
                assert_eq!(get(&handler), before);
            }
            for _ in 0..2 {
                assert_eq!(post(&handler, RESET), initial);
                assert_eq!(get(&handler), initial);
            }
        }
    }

    #[test]
    fn http_routes_methods_queries_and_assets_are_closed() {
        let handler = modified_handler();
        // Holding the lock would deadlock an asset path that tried to acquire it.
        let guard = handler.session.lock().expect("healthy");
        for (target, expected) in ASSETS {
            let HandlerOutcome::Asset(actual) = handler.handle(request("GET", target, b""), PORT)
            else {
                panic!("asset must be a closed dispatch, never placeholder JSON")
            };
            assert_eq!(
                std::mem::discriminant(&actual),
                std::mem::discriminant(&expected)
            );
        }
        drop(guard);
        for (target, allow) in ASSETS
            .iter()
            .map(|(target, _)| (*target, "GET"))
            .chain([(STATE, "GET"), (ACTION, "POST")])
        {
            let opposite = if allow == "GET" { "POST" } else { "GET" };
            for method in [opposite, "HEAD", "OPTIONS", "get", "post", "DELETE"] {
                fails_unchanged(
                    &handler,
                    request(method, target, b""),
                    PORT,
                    405,
                    "method_not_allowed",
                    Some(allow),
                );
            }
        }
        for target in [
            "/missing",
            "/api/playground/consultant/",
            "/API/playground/consultant",
            "/api/consultant",
            "/assets/Styles.css",
            "/index.html",
        ] {
            for method in ["GET", "HEAD", "OPTIONS"] {
                fails_unchanged(
                    &handler,
                    request(method, target, b""),
                    PORT,
                    404,
                    "not_found",
                    None,
                );
            }
        }
        for target in [
            "",
            "api/playground/consultant",
            "http://127.0.0.1:7788/",
            "//api/playground/consultant",
            "/assets//app.js",
            "/?secret=sentinel",
            "/#sentinel",
            "/%61ssets/app.js",
            "/assets\\app.js",
            "/\r",
            "/\n",
            "/\0",
            "/.",
            "/..",
            "/./assets/app.js",
            "/assets/../app.js",
            "/assets/.",
            "/assets/..",
        ] {
            fails_unchanged(
                &handler,
                request("GET", target, b""),
                PORT,
                400,
                "invalid_target",
                None,
            );
        }
    }

    #[test]
    fn http_host_and_origin_use_only_trusted_bound_port() {
        let handler = modified_handler();
        for (port, host, origin) in [
            (7788, "127.0.0.1:7788", "http://127.0.0.1:7788"),
            (8123, "127.0.0.1:8123", "http://127.0.0.1:8123"),
            (80, "127.0.0.1:80", "http://127.0.0.1:80"),
            (80, "127.0.0.1", "http://127.0.0.1"),
            (80, "127.0.0.1", "http://127.0.0.1:80"),
            (80, "127.0.0.1:80", "http://127.0.0.1"),
        ] {
            let host = format!(" \t{host}\t ");
            let origin = format!("\t{origin} ");
            let headers = [
                ("hOsT", host.as_str()),
                ("oRiGiN", origin.as_str()),
                ("cOnTeNt-TyPe", " \tAPPLICATION/JSON; CHARSET=UTF-8 \t"),
            ];
            let before = get(&handler);
            assert_eq!(
                success(
                    &handler,
                    HttpRequest {
                        body: SEARCH,
                        ..with_headers(&headers)
                    },
                    port
                ),
                before
            );
        }
        for headers in [
            &[][..],
            &[(" Host", "127.0.0.1:7788")],
            &[("Host", "127.0.0.1:7788"), ("hOsT", "127.0.0.1:7788")],
        ] {
            fails_unchanged(
                &handler,
                with_headers(headers),
                PORT,
                400,
                "invalid_host",
                None,
            );
        }
        for host in [
            "127.0.0.1:8123",
            "127.0.0.1",
            "localhost:7788",
            "[::1]:7788",
            "0.0.0.0:7788",
            "127.0.0.1.:7788",
            "127.0.0.1:07788",
            "user@127.0.0.1:7788",
            "127.0.0.1:7788,127.0.0.1:7788",
            "127.0.0.1:7788\r",
            "127.0.0.1:7788\n",
            "127.0.0.1:7788\0",
            "sentinel-host",
        ] {
            fails_unchanged(
                &handler,
                with_headers(&[("Host", host)]),
                PORT,
                400,
                "invalid_host",
                None,
            );
        }
        for origin in [
            "null",
            "https://127.0.0.1:7788",
            "http://localhost:7788",
            "http://[::1]:7788",
            "http://127.0.0.1:8123",
            "http://127.0.0.1",
            "http://external.invalid",
            "http://127.0.0.1:7788/",
            "http://127.0.0.1:07788",
            "http://127.0.0.1:7788\r",
            "http://127.0.0.1:7788\n",
            "http://127.0.0.1:7788\0",
        ] {
            fails_unchanged(
                &handler,
                with_headers(&[("Host", "127.0.0.1:7788"), ("Origin", origin)]),
                PORT,
                403,
                "origin_forbidden",
                None,
            );
        }
        fails_unchanged(
            &handler,
            with_headers(&[
                ("Host", "127.0.0.1:7788"),
                ("Origin", "http://127.0.0.1:7788"),
                ("oRiGiN", "http://127.0.0.1:7788"),
            ]),
            PORT,
            403,
            "origin_forbidden",
            None,
        );
        // Matching untrusted Host+Origin cannot change the transport's expected port.
        fails_unchanged(
            &handler,
            with_headers(&[
                ("Host", "127.0.0.1:8123"),
                ("Origin", "http://127.0.0.1:8123"),
            ]),
            PORT,
            400,
            "invalid_host",
            None,
        );
        fails_unchanged(
            &handler,
            request("POST", ACTION, RESET),
            0,
            500,
            "invalid_configuration",
            None,
        );
        assert_eq!(
            post(&handler, SEARCH),
            get(&handler),
            "Origin may be absent"
        );
    }

    #[test]
    fn http_action_body_is_bounded_and_strict() {
        let handler = modified_handler();
        let mut deep = br#"{"action":"#.to_vec();
        deep.extend(vec![b'['; 110]);
        deep.push(b'0');
        deep.extend(vec![b']'; 110]);
        deep.push(b"}"[0]);
        assert!(deep.len() <= 256);
        assert!(serde_json::from_slice::<Value>(&deep).is_ok());
        for body in [
            b"".as_slice(),
            b" \t\r\n",
            &[0xff],
            b"{",
            br#"{}"#,
            b"null",
            b"42",
            b"[]",
            br#"["Reset"]"#,
            br#"["CorrectOfferPrice"]"#,
            br#""Reset""#,
            br#"{"action":null}"#,
            br#"{"action":[]}"#,
            br#"{"action":["Reset"]}"#,
            br#"{"action":42}"#,
            br#"{"action":true}"#,
            br#"{"action":{"Reset":null}}"#,
            br#"{"action":{"CorrectOfferPrice":null}}"#,
            br#"{"action":"reset"}"#,
            br#"{"Action":"Reset"}"#,
            br#"{"action":"UnknownSentinel"}"#,
            br#"{"action":"Reset","price":1}"#,
            br#"{"action":"Reset","action":"Reset"}"#,
            br#"{"action":"Reset","a\u0063tion":"Reset"}"#,
            br#"{"action":"Reset"}{"action":"Reset"}"#,
            br#"{"action":"Reset"} null"#,
            br#"{"action":"Reset",}"#,
            &deep,
        ] {
            fails_unchanged(
                &handler,
                request("POST", ACTION, body),
                PORT,
                400,
                "invalid_action_request",
                None,
            );
        }
        let before = get(&handler);
        let mut padded = SEARCH.to_vec();
        padded.resize(256, b' ');
        assert_eq!(post(&handler, &padded), before);
        padded.push(b' ');
        fails_unchanged(
            &handler,
            request("POST", ACTION, &padded),
            PORT,
            413,
            "payload_too_large",
            None,
        );
        // A complete JSON value beyond serde's depth limit necessarily exceeds
        // this endpoint's byte limit too; the earlier size error must win.
        let too_deep = format!("{}0{}", "[".repeat(130), "]".repeat(130));
        fails_unchanged(
            &handler,
            request("POST", ACTION, too_deep.as_bytes()),
            PORT,
            413,
            "payload_too_large",
            None,
        );
        assert_eq!(
            post(&handler, br#"{"a\u0063tion":"ShowReporting\u0053earch"}"#),
            before
        );
    }

    // Each row violates this rule and later rules: it pins the entire short-circuit
    // ordering, including when the same requests encounter a poisoned session.
    fn precedence_cases() -> Vec<(
        HttpRequest<'static>,
        u16,
        u16,
        &'static str,
        Option<&'static str>,
    )> {
        vec![
            (
                HttpRequest {
                    headers: &[],
                    ..request("HEAD", "/?sentinel", LARGE)
                },
                0,
                500,
                "invalid_configuration",
                None,
            ),
            (
                HttpRequest {
                    headers: &[("Host", "sentinel"), ("Origin", "null")],
                    ..request("HEAD", "/?sentinel", LARGE)
                },
                PORT,
                400,
                "invalid_host",
                None,
            ),
            (
                HttpRequest {
                    headers: &[("Host", "127.0.0.1:7788"), ("Origin", "null")],
                    ..request("HEAD", "/?sentinel", LARGE)
                },
                PORT,
                403,
                "origin_forbidden",
                None,
            ),
            (
                request("HEAD", "/?sentinel", LARGE),
                PORT,
                400,
                "invalid_target",
                None,
            ),
            (
                request("HEAD", "/sentinel", LARGE),
                PORT,
                404,
                "not_found",
                None,
            ),
            (
                request("GET", ACTION, LARGE),
                PORT,
                405,
                "method_not_allowed",
                Some("POST"),
            ),
            (
                request("POST", STATE, LARGE),
                PORT,
                405,
                "method_not_allowed",
                Some("GET"),
            ),
            (
                HttpRequest {
                    headers: &[("Host", "127.0.0.1:7788"), ("Content-Encoding", "gzip")],
                    ..request("POST", ACTION, LARGE)
                },
                PORT,
                413,
                "payload_too_large",
                None,
            ),
            (
                HttpRequest {
                    headers: &[("Host", "127.0.0.1:7788"), ("Content-Encoding", "gzip")],
                    ..request("GET", STATE, b"sentinel")
                },
                PORT,
                415,
                "unsupported_media_type",
                None,
            ),
            (
                HttpRequest {
                    headers: &[("Host", "127.0.0.1:7788"), ("Content-Type", "text/plain")],
                    ..request("GET", STATE, b"sentinel")
                },
                PORT,
                400,
                "unexpected_body",
                None,
            ),
            (
                HttpRequest {
                    headers: HOST,
                    ..request("POST", ACTION, b"sentinel")
                },
                PORT,
                415,
                "unsupported_media_type",
                None,
            ),
            (
                request("POST", ACTION, b"sentinel"),
                PORT,
                400,
                "invalid_action_request",
                None,
            ),
        ]
    }

    #[test]
    fn http_media_body_and_error_precedence_are_stable() {
        let handler = modified_handler();
        for (req, port, status, code, allow) in precedence_cases() {
            fails_unchanged(&handler, req, port, status, code, allow);
        }
        for media in [
            "",
            "text/plain",
            "application/json;charset=utf-8",
            "application/json; charset=UTF-16",
            "application/json ; charset=utf-8",
            "application/json; charset=\"utf-8\"",
            "application/json; x=y",
            "application/json\r",
            "application/json\n",
            "application/json\0",
        ] {
            fails_unchanged(
                &handler,
                with_headers(&[("Host", "127.0.0.1:7788"), ("Content-Type", media)]),
                PORT,
                415,
                "unsupported_media_type",
                None,
            );
        }
        for headers in [
            HOST,
            &[
                ("Host", "127.0.0.1:7788"),
                ("Content-Type", "application/json"),
                ("cOnTeNt-TyPe", "application/json"),
            ],
            &[
                ("Host", "127.0.0.1:7788"),
                ("Content-Type ", "application/json"),
            ],
        ] {
            fails_unchanged(
                &handler,
                with_headers(headers),
                PORT,
                415,
                "unsupported_media_type",
                None,
            );
            assert_eq!(
                success(
                    &handler,
                    HttpRequest {
                        headers,
                        ..request("GET", STATE, b"")
                    },
                    PORT
                ),
                get(&handler)
            );
        }
        for encoding in ["", "identity", "gzip", "sentinel"] {
            let headers = [
                ("Host", "127.0.0.1:7788"),
                ("Content-Type", "application/json"),
                ("cOnTeNt-EnCoDiNg", encoding),
            ];
            for (method, target, body) in [
                ("GET", STATE, b"".as_slice()),
                ("POST", ACTION, RESET),
                ("GET", "/", b""),
            ] {
                fails_unchanged(
                    &handler,
                    HttpRequest {
                        headers: &headers,
                        ..request(method, target, body)
                    },
                    PORT,
                    415,
                    "unsupported_media_type",
                    None,
                );
            }
        }
        for length in ["0", "1", "256", "257", "sentinel"] {
            let headers = [
                ("Host", "127.0.0.1:7788"),
                ("Content-Type", "application/json"),
                ("Content-Length", length),
            ];
            fails_unchanged(
                &handler,
                HttpRequest {
                    headers: &headers,
                    ..request("POST", ACTION, LARGE)
                },
                PORT,
                413,
                "payload_too_large",
                None,
            );
        }
        for media in [
            "application/json",
            "APPLICATION/JSON",
            " application/json; charset=utf-8\t",
        ] {
            let headers = [("Host", "127.0.0.1:7788"), ("Content-Type", media)];
            assert_eq!(
                success(
                    &handler,
                    HttpRequest {
                        headers: &headers,
                        ..request("POST", ACTION, SEARCH)
                    },
                    PORT
                ),
                get(&handler)
            );
        }
    }

    #[test]
    fn http_mutex_poison_fails_closed_without_recovery() {
        let handler = Arc::new(modified_handler());
        poison(&handler);
        for _ in 0..2 {
            for (method, target, body) in [
                ("GET", STATE, b"".as_slice()),
                ("POST", ACTION, RESET),
                ("POST", ACTION, CORRECT),
            ] {
                assert_error(
                    response(&handler, request(method, target, body), PORT),
                    503,
                    "session_unavailable",
                    None,
                );
                assert!(handler.session.is_poisoned());
            }
            for (req, port, status, code, allow) in precedence_cases() {
                assert_error(response(&handler, req, port), status, code, allow);
                assert!(handler.session.is_poisoned());
            }
            for (target, expected) in ASSETS {
                let HandlerOutcome::Asset(actual) =
                    handler.handle(request("GET", target, b""), PORT)
                else {
                    panic!("poison must not block assets")
                };
                assert_eq!(
                    std::mem::discriminant(&actual),
                    std::mem::discriminant(&expected)
                );
                assert!(handler.session.is_poisoned());
            }
        }
    }

    #[test]
    fn http_json_errors_and_headers_never_reflect_inputs() {
        let handler = modified_handler();
        let mut codes = Vec::new();
        // Shared literal assertions enforce all four keys, fixed metadata, and exact
        // headers: extra messages, labels, CORS, cookies, redirects or reflected
        // sentinel strings fail structural equality, not substring checks.
        for (req, port, status, code, allow) in precedence_cases() {
            fails_unchanged(&handler, req, port, status, code, allow);
            codes.push(code);
        }
        let poisoned = Arc::new(modified_handler());
        poison(&poisoned);
        assert_error(
            response(&poisoned, request("POST", ACTION, RESET), PORT),
            503,
            "session_unavailable",
            None,
        );
        codes.push("session_unavailable");
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(
            codes,
            [
                "invalid_action_request",
                "invalid_configuration",
                "invalid_host",
                "invalid_target",
                "method_not_allowed",
                "not_found",
                "origin_forbidden",
                "payload_too_large",
                "session_unavailable",
                "unexpected_body",
                "unsupported_media_type"
            ]
        );
        let before = get(&handler);
        let headers = [
            ("Host", "127.0.0.1:7788"),
            ("Cookie", "sentinel-cookie"),
            ("Authorization", "sentinel-auth"),
            ("X-Forwarded-Host", "sentinel-host"),
        ];
        assert_eq!(
            success(
                &handler,
                HttpRequest {
                    headers: &headers,
                    ..request("GET", STATE, b"")
                },
                PORT
            ),
            before
        );
    }
}

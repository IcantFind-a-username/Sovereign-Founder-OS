//! Separate, memory-only business exercise. No Workspace operation except its pure amount parser.
use serde::{Deserialize, Serialize};
use std::io::{Cursor, Read};
use tiny_http::{Header, Method, Request, Response, Server};

use crate::workspace::parse_amount_cents;

const MAX_BODY: usize = 32_768;
const MAX_VIEW: usize = 1_048_576;
const MAX_REVISION: u32 = 10_000;
const SEED_NAME: &str = "North Star Operations";
const SEED_GOAL: &str = "Make weekly reporting easier to run";
const CSP: &str = "default-src 'none'; script-src 'self'; style-src 'unsafe-inline'; connect-src 'self'; img-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'";

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(super) enum OriginKind {
    SeededSynthetic,
    UnverifiedExperimentInput,
}
#[derive(Clone, Debug, Serialize, PartialEq)]
pub(super) enum Currency {
    #[serde(rename = "SGD")]
    Sgd,
}
#[derive(Clone, Debug, Serialize, PartialEq)]
pub(super) struct Company {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) goal: String,
    pub(super) origin: OriginKind,
}
#[derive(Clone, Debug, Serialize, PartialEq)]
pub(super) struct Service {
    pub(super) id: String,
    pub(super) company_id: String,
    pub(super) name: String,
    pub(super) scope: String,
    pub(super) currency: Currency,
    pub(super) unit_price_cents: u64,
    pub(super) origin: OriginKind,
}
#[derive(Clone, Debug, Serialize, PartialEq)]
pub(super) struct Case {
    pub(super) id: String,
    pub(super) input_revision: u32,
    pub(super) company: Company,
    pub(super) service: Service,
}
#[derive(Clone, Debug, Serialize, PartialEq)]
pub(super) struct Summary {
    pub(super) currency: Currency,
    pub(super) next_step: String,
}
#[derive(Clone, Debug, Serialize, PartialEq)]
pub(super) struct DemoView {
    pub(super) epoch: String,
    pub(super) revision: u32,
    pub(super) demo_day: u16,
    pub(super) case: Case,
    pub(super) maturity: String,
    pub(super) available_commands: Vec<String>,
    pub(super) summary: Summary,
}
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CommandRequest {
    pub(super) epoch: String,
    pub(super) expected_revision: u32,
    pub(super) command: Command,
}
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(
    tag = "type",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub(super) enum Command {
    SaveCompany(CompanyInput),
    Reset(ResetInput),
}
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CompanyInput {
    pub(super) name: String,
    pub(super) goal: String,
    pub(super) service_name: String,
    pub(super) service_scope: String,
    pub(super) currency: String,
    pub(super) unit_price: String,
}
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ResetInput {}
#[derive(Clone, Debug, PartialEq)]
pub(super) struct DemoState {
    pub(super) epoch: String,
    pub(super) revision: u32,
    pub(super) demo_day: u16,
    pub(super) case: Case,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum ErrorCode {
    InvalidInput,
    StaleEpoch,
    StaleRevision,
    LimitReached,
    UnsupportedCurrency,
    InternalError,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub(super) struct DemoError {
    pub(super) code: ErrorCode,
    pub(super) field: Option<String>,
}
impl DemoError {
    fn new(code: ErrorCode) -> Self {
        Self { code, field: None }
    }
    fn invalid(field: &'static str) -> Self {
        Self {
            code: ErrorCode::InvalidInput,
            field: Some(field.into()),
        }
    }
    fn status(&self) -> u16 {
        match self.code {
            ErrorCode::StaleEpoch | ErrorCode::StaleRevision | ErrorCode::LimitReached => 409,
            ErrorCode::InternalError => 500,
            ErrorCode::InvalidInput | ErrorCode::UnsupportedCurrency => 422,
        }
    }
}
impl DemoState {
    pub(super) fn seeded(epoch: String) -> Self {
        Self {
            epoch,
            revision: 0,
            demo_day: 0,
            case: Case {
                id: "case-1".into(),
                input_revision: 0,
                company: Company {
                    id: "company-1".into(),
                    name: SEED_NAME.into(),
                    goal: SEED_GOAL.into(),
                    origin: OriginKind::SeededSynthetic,
                },
                service: Service {
                    id: "service-1".into(),
                    company_id: "company-1".into(),
                    name: "Reporting clarity sprint".into(),
                    scope: "Assess weekly reporting and produce an improvement plan".into(),
                    currency: Currency::Sgd,
                    unit_price_cents: 250_000,
                    origin: OriginKind::SeededSynthetic,
                },
            },
        }
    }
    pub(super) fn view(&self) -> DemoView {
        DemoView {
            epoch: self.epoch.clone(),
            revision: self.revision,
            demo_day: self.demo_day,
            case: self.case.clone(),
            maturity: "synthetic_experiment".into(),
            available_commands: vec!["save_company".into(), "reset".into()],
            summary: Summary {
                currency: Currency::Sgd,
                next_step: "Enter the practice customer and discovery in the next increment".into(),
            },
        }
    }
    pub(super) fn apply(&mut self, request: CommandRequest) -> Result<DemoView, DemoError> {
        self.apply_bounded(request, new_epoch, MAX_VIEW)
    }
    // Private injection points exercise collision exhaustion and response bounds without globals.
    fn apply_bounded(
        &mut self,
        request: CommandRequest,
        mut draw: impl FnMut() -> String,
        max_view: usize,
    ) -> Result<DemoView, DemoError> {
        if request.epoch != self.epoch {
            return Err(DemoError::new(ErrorCode::StaleEpoch));
        }
        if request.expected_revision != self.revision {
            return Err(DemoError::new(ErrorCode::StaleRevision));
        }
        let mut candidate = self.clone();
        match request.command {
            Command::SaveCompany(input) => {
                if candidate.revision >= MAX_REVISION {
                    return Err(DemoError::new(ErrorCode::LimitReached));
                }
                save_company(&mut candidate, input)?;
                candidate.revision += 1;
            }
            Command::Reset(_) => {
                let epoch = (0..4)
                    .map(|_| draw())
                    .find(|epoch| epoch != &self.epoch)
                    .ok_or_else(|| DemoError::new(ErrorCode::InternalError))?;
                candidate = Self::seeded(epoch);
            }
        }
        let view = candidate.view();
        let bytes =
            serde_json::to_vec(&view).map_err(|_| DemoError::new(ErrorCode::InternalError))?;
        if bytes.len() > max_view {
            return Err(DemoError::new(ErrorCode::LimitReached));
        }
        *self = candidate;
        Ok(view)
    }
}
pub(super) fn save_company(state: &mut DemoState, input: CompanyInput) -> Result<(), DemoError> {
    let name = clean("name", &input.name, 120, false)?;
    let goal = clean("goal", &input.goal, 2000, true)?;
    let service_name = clean("service_name", &input.service_name, 120, false)?;
    let scope = clean("service_scope", &input.service_scope, 2000, true)?;
    if input.currency != "SGD" {
        return Err(DemoError {
            code: ErrorCode::UnsupportedCurrency,
            field: Some("currency".into()),
        });
    }
    let cents =
        parse_amount_cents(&input.unit_price).map_err(|_| DemoError::invalid("unit_price"))?;
    if cents == 0 || cents > 100_000_000 {
        return Err(DemoError::invalid("unit_price"));
    }
    let next = state
        .case
        .input_revision
        .checked_add(1)
        .filter(|n| *n <= MAX_REVISION)
        .ok_or_else(|| DemoError::new(ErrorCode::LimitReached))?;
    state.case.company.name = name;
    state.case.company.goal = goal;
    state.case.company.origin = OriginKind::UnverifiedExperimentInput;
    state.case.service.name = service_name;
    state.case.service.scope = scope;
    state.case.service.currency = Currency::Sgd;
    state.case.service.unit_price_cents = cents;
    state.case.service.origin = OriginKind::UnverifiedExperimentInput;
    state.case.input_revision = next;
    Ok(())
}
fn clean(
    field: &'static str,
    value: &str,
    max: usize,
    multiline: bool,
) -> Result<String, DemoError> {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed.len() > max
        || value
            .chars()
            .any(|c| c.is_control() && !(multiline && c == '\n'))
    {
        return Err(DemoError::invalid(field));
    }
    Ok(trimmed.into())
}
pub(super) fn new_epoch() -> String {
    hex::encode(rand::random::<[u8; 16]>())
}

#[derive(Serialize)]
struct OkEnvelope<T> {
    ok: bool,
    view: T,
}
#[derive(Serialize)]
struct ErrorEnvelope<T> {
    ok: bool,
    error: T,
}
#[derive(Serialize)]
struct HttpError<'a> {
    code: &'a str,
    field: Option<&'a str>,
}
fn response(status: u16, bytes: Vec<u8>, content_type: &str) -> Response<Cursor<Vec<u8>>> {
    let mut response = Response::from_data(bytes).with_status_code(status);
    for (name, value) in [
        ("Content-Type", content_type),
        ("Cache-Control", "no-store"),
        ("X-Content-Type-Options", "nosniff"),
        ("Referrer-Policy", "no-referrer"),
        ("Content-Security-Policy", CSP),
    ] {
        response.add_header(Header::from_bytes(name, value).expect("fixed ASCII response header"));
    }
    response
}
fn json_response(status: u16, value: &impl Serialize) -> Response<Cursor<Vec<u8>>> {
    match serde_json::to_vec(value) {
        Ok(body) => response(status, body, "application/json; charset=utf-8"),
        Err(_) => response(
            500,
            br#"{"ok":false,"error":{"code":"internal_error","field":null}}"#.to_vec(),
            "application/json; charset=utf-8",
        ),
    }
}
fn error_response(status: u16, code: &'static str) -> Response<Cursor<Vec<u8>>> {
    json_response(
        status,
        &ErrorEnvelope {
            ok: false,
            error: HttpError { code, field: None },
        },
    )
}
fn headers<'a>(request: &'a Request, name: &'static str) -> Vec<&'a str> {
    request
        .headers()
        .iter()
        .filter(|h| h.field.equiv(name))
        .map(|h| h.value.as_str())
        .collect()
}
fn media_allowed(value: &str) -> bool {
    let parts: Vec<_> = value.split(';').map(str::trim).collect();
    match parts.as_slice() {
        [media] => media.eq_ignore_ascii_case("application/json"),
        [media, charset] => {
            media.eq_ignore_ascii_case("application/json")
                && charset.eq_ignore_ascii_case("charset=utf-8")
        }
        _ => false,
    }
}
pub(super) fn handle(
    state: &mut DemoState,
    request: &mut Request,
    bound_port: u16,
) -> Response<Cursor<Vec<u8>>> {
    let host = format!("127.0.0.1:{bound_port}");
    let origin = format!("http://{host}");
    let origins = headers(request, "Origin");
    if headers(request, "Host") != [host.as_str()]
        || origins.len() > 1
        || origins.first().is_some_and(|value| *value != origin)
        || (*request.method() == Method::Post && origins.is_empty())
    {
        return error_response(403, "forbidden_origin");
    }
    let allowed = match request.url() {
        "/" | "/app.js" | "/api/demo" => Method::Get,
        "/api/demo/command" => Method::Post,
        _ => return error_response(404, "not_found"),
    };
    if request.method() != &allowed {
        let mut result = error_response(405, "method_not_allowed");
        result.add_header(Header::from_bytes("Allow", allowed.as_str()).expect("fixed method"));
        return result;
    }
    let lengths = headers(request, "Content-Length");
    if lengths.len() > 1
        || lengths.first().is_some_and(|s| s.parse::<usize>().is_err())
        || !headers(request, "Transfer-Encoding").is_empty()
        || !headers(request, "Expect").is_empty()
        || headers(request, "Connection").iter().any(|h| {
            h.split(',')
                .any(|v| v.trim().eq_ignore_ascii_case("upgrade"))
        })
    {
        return error_response(400, "invalid_request");
    }
    if allowed == Method::Get {
        if request.body_length().is_some_and(|n| n > 0) {
            return error_response(400, "invalid_request");
        }
        return match request.url() {
            "/" => response(
                200,
                include_bytes!("../business-demo/index.html").to_vec(),
                "text/html; charset=utf-8",
            ),
            "/app.js" => response(
                200,
                include_bytes!("../business-demo/app.js").to_vec(),
                "application/javascript; charset=utf-8",
            ),
            _ => json_response(
                200,
                &OkEnvelope {
                    ok: true,
                    view: state.view(),
                },
            ),
        };
    }
    if headers(request, "X-Founder-Demo") != ["1"] {
        return error_response(403, "forbidden_origin");
    }
    let media = headers(request, "Content-Type");
    if media.len() != 1 || !media_allowed(media[0]) {
        return error_response(415, "unsupported_media_type");
    }
    let Some(length) = request.body_length() else {
        return error_response(411, "length_required");
    };
    if length > MAX_BODY {
        return error_response(413, "payload_too_large");
    }
    let mut body = Vec::new();
    if request
        .as_reader()
        .take((MAX_BODY + 1) as u64)
        .read_to_end(&mut body)
        .is_err()
    {
        return error_response(400, "invalid_request");
    }
    if body.len() > MAX_BODY {
        return error_response(413, "payload_too_large");
    }
    if body.len() != length {
        return error_response(400, "invalid_request");
    }
    let parsed = match serde_json::from_slice::<CommandRequest>(&body) {
        Ok(parsed) => parsed,
        Err(_) => return error_response(400, "invalid_json"),
    };
    match state.apply(parsed) {
        Ok(view) => json_response(200, &OkEnvelope { ok: true, view }),
        Err(error) => json_response(error.status(), &ErrorEnvelope { ok: false, error }),
    }
}
pub(crate) fn run(port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let server = Server::http(("127.0.0.1", port)).map_err(std::io::Error::other)?;
    let bound = server
        .server_addr()
        .to_ip()
        .ok_or("invalid listener")?
        .port();
    println!("Playground: http://127.0.0.1:{bound}");
    println!(
        "Business demo — synthetic exercise; memory only; no AI or external business actions."
    );
    let mut state = DemoState::seeded(new_epoch());
    for mut request in server.incoming_requests() {
        let result = handle(&mut state, &mut request, bound);
        let _ = request.respond(result);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> CompanyInput {
        CompanyInput {
            name: "New company".into(),
            goal: "New goal".into(),
            service_name: "New service".into(),
            service_scope: "New scope".into(),
            currency: "SGD".into(),
            unit_price: "3500.00".into(),
        }
    }
    fn request(state: &DemoState, command: Command) -> CommandRequest {
        CommandRequest {
            epoch: state.epoch.clone(),
            expected_revision: state.revision,
            command,
        }
    }
    fn save(state: &mut DemoState, input: CompanyInput) -> Result<DemoView, DemoError> {
        state.apply(request(state, Command::SaveCompany(input)))
    }
    #[test]
    fn business_demo_company_save_is_atomic() {
        let mut state = DemoState::seeded("0".repeat(32));
        let result = save(&mut state, input()).unwrap();
        assert_eq!(result.revision, 1);
        assert_eq!(result.case.input_revision, 1);
        assert_eq!(result.case.company.name, "New company");
        assert_eq!(result.case.company.goal, "New goal");
        assert_eq!(result.case.service.name, "New service");
        assert_eq!(result.case.service.scope, "New scope");
        assert_eq!(result.case.service.unit_price_cents, 350_000);
        assert_eq!(
            serde_json::to_value(&result.case.company.origin).unwrap(),
            "unverified_experiment_input"
        );
        let before = state.view();
        let mut bad = input();
        bad.unit_price = "invalid".into();
        assert!(save(&mut state, bad).is_err());
        assert_eq!(state.view(), before);
    }
    #[test]
    fn business_demo_amount_and_text_limits() {
        let cases = [
            ("name", "".to_owned()),
            ("name", "a".repeat(121)),
            ("name", "short\nname".into()),
            ("service_name", "bad\nname".into()),
            ("goal", "a".repeat(2001)),
            ("goal", "bad\tgoal".into()),
            ("service_scope", "bad\rtext".into()),
            ("unit_price", "0".into()),
            ("unit_price", "1000000.01".into()),
            ("unit_price", "1.001".into()),
            ("unit_price", "-1".into()),
            ("unit_price", "9999999999999999".into()),
            ("currency", "USD".into()),
            ("name", "界".repeat(41)),
        ];
        for (field, value) in cases {
            let mut state = DemoState::seeded("0".repeat(32));
            let before = state.view();
            let mut data = input();
            match field {
                "name" => data.name = value,
                "service_name" => data.service_name = value,
                "goal" => data.goal = value,
                "service_scope" => data.service_scope = value,
                "unit_price" => data.unit_price = value,
                "currency" => data.currency = value,
                _ => unreachable!(),
            }
            let error = save(&mut state, data).expect_err(field);
            assert_eq!(error.field.as_deref(), Some(field));
            assert_eq!(state.view(), before, "{field}");
        }
        let mut state = DemoState::seeded("0".repeat(32));
        let mut valid = input();
        valid.name = "界".repeat(40);
        valid.goal = "a".repeat(2000);
        valid.service_scope = "Line one\nLine two".into();
        valid.unit_price = "1000000.00".into();
        assert_eq!(
            save(&mut state, valid)
                .unwrap()
                .case
                .service
                .unit_price_cents,
            100_000_000
        );
    }
    #[test]
    fn business_demo_epoch_revision_and_reset() {
        let mut state = DemoState::seeded("0".repeat(32));
        for (epoch, revision) in [("1".repeat(32), 99), (state.epoch.clone(), 99)] {
            let before = state.view();
            assert!(state
                .apply(CommandRequest {
                    epoch,
                    expected_revision: revision,
                    command: Command::Reset(ResetInput {})
                })
                .is_err());
            assert_eq!(state.view(), before);
        }
        save(&mut state, input()).unwrap();
        save(&mut state, input()).unwrap();
        assert_eq!(state.revision, 2);
        let old = state.epoch.clone();
        let reset = state
            .apply(request(&state, Command::Reset(ResetInput {})))
            .unwrap();
        assert_ne!(reset.epoch, old);
        assert_eq!(reset.revision, 0);
        assert_eq!(reset.case.company.name, SEED_NAME);
        state.revision = 10_000;
        let before = state.view();
        assert!(save(&mut state, input()).is_err());
        assert_eq!(state.view(), before);
    }
    #[test]
    fn business_demo_detached_get_is_read_only() {
        let state = DemoState::seeded("0".repeat(32));
        let before = state.view();
        let mut detached = state.view();
        detached.case.company.name = "Different".into();
        detached.available_commands.clear();
        detached.epoch.clear();
        assert_eq!(state.view(), before);
        assert_ne!(detached, state.view());
    }
    #[test]
    fn business_demo_reset_collision_and_view_bound_are_atomic() {
        let mut state = DemoState::seeded("0".repeat(32));
        let before = state.view();
        let mut draws = 0;
        let request = request(&state, Command::Reset(ResetInput {}));
        let result = state.apply_bounded(
            request,
            || {
                draws += 1;
                "0".repeat(32)
            },
            MAX_VIEW,
        );
        assert_eq!(result.unwrap_err().code, ErrorCode::InternalError);
        assert_eq!(draws, 4);
        assert_eq!(state.view(), before);
        let command = CommandRequest {
            epoch: state.epoch.clone(),
            expected_revision: 0,
            command: Command::SaveCompany(input()),
        };
        let result = state.apply_bounded(command, || panic!("save must not draw an epoch"), 1);
        assert_eq!(result.unwrap_err().code, ErrorCode::LimitReached);
        assert_eq!(state.view(), before);
        let reset = CommandRequest {
            epoch: state.epoch.clone(),
            expected_revision: 0,
            command: Command::Reset(ResetInput {}),
        };
        draws = 0;
        let result = state
            .apply_bounded(
                reset,
                || {
                    draws += 1;
                    if draws < 4 {
                        "0".repeat(32)
                    } else {
                        "f".repeat(32)
                    }
                },
                MAX_VIEW,
            )
            .unwrap();
        assert_eq!(draws, 4);
        assert_eq!(result.epoch, "f".repeat(32));
    }
    #[test]
    fn business_demo_serialization_failure_is_not_success() {
        struct Broken;
        impl Serialize for Broken {
            fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
                Err(serde::ser::Error::custom("fixture failure"))
            }
        }
        let response = json_response(200, &Broken);
        assert_eq!(response.status_code().0, 500);
        let bytes = response.into_reader().into_inner();
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            value,
            serde_json::json!({"ok":false,"error":{"code":"internal_error","field":null}})
        );
    }
}

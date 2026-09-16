//! Memory-only owner surface: one-credential registry plus sessions.
//!
//! Reuses `crates/owner` as a library. Empty-registry enrolment is first
//! writer wins and is not owner admission. Restart drops this state.

use std::time::Instant;

use serde_json::{json, Value};
use sovereign_owner::bootstrap::Qualification;
use sovereign_owner::config::{CeremonyConfig, CEREMONY_TIMEOUT, ORIGIN, RP_ID};
use sovereign_owner::registry::{CeremonyError, Registry};
use sovereign_owner::session::{SessionError, SessionTokens, Sessions, TOKEN_LEN};
use uuid::Uuid;

pub const SESSION_COOKIE: &str = "__Host-sfo_fixture_session";
pub const CSRF_HEADER: &str = "x-sfo-csrf";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixtureRoute {
    Shell,
    RegisterStart,
    RegisterFinish,
    LoginStart,
    LoginFinish,
    Logout,
}

impl FixtureRoute {
    pub fn is_pre_session(self) -> bool {
        !matches!(self, FixtureRoute::Logout)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnerError {
    AlreadyRegistered,
    NotRegistered,
    UnknownCeremony,
    CeremonyExpired,
    HandleMismatch,
    CredentialMismatch,
    UserVerificationMissing,
    SessionUnknown,
    SessionExpired,
    SessionIdle,
    CsrfMismatch,
    BadBody,
}

impl OwnerError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::AlreadyRegistered => "E-ALREADY-REGISTERED",
            Self::NotRegistered => "E-NOT-REGISTERED",
            Self::UnknownCeremony => "E-UNKNOWN-CEREMONY",
            Self::CeremonyExpired => "E-CEREMONY-EXPIRED",
            Self::HandleMismatch => "E-HANDLE-MISMATCH",
            Self::CredentialMismatch => "E-CREDENTIAL-MISMATCH",
            Self::UserVerificationMissing => "E-UV-REQUIRED",
            Self::SessionUnknown => "E-SESSION-UNKNOWN",
            Self::SessionExpired => "E-SESSION-EXPIRED",
            Self::SessionIdle => "E-SESSION-IDLE",
            Self::CsrfMismatch => "E-CSRF-MISMATCH",
            Self::BadBody => "E-BAD-BODY",
        }
    }
}

impl From<CeremonyError> for OwnerError {
    fn from(error: CeremonyError) -> Self {
        match error {
            CeremonyError::AlreadyRegistered => Self::AlreadyRegistered,
            CeremonyError::NotRegistered => Self::NotRegistered,
            CeremonyError::UnknownCeremony => Self::UnknownCeremony,
            CeremonyError::CeremonyExpired => Self::CeremonyExpired,
            CeremonyError::HandleMismatch => Self::HandleMismatch,
            CeremonyError::CredentialMismatch => Self::CredentialMismatch,
            CeremonyError::UserVerificationMissing => Self::UserVerificationMissing,
        }
    }
}

impl From<SessionError> for OwnerError {
    fn from(error: SessionError) -> Self {
        match error {
            SessionError::Unknown => Self::SessionUnknown,
            SessionError::Expired => Self::SessionExpired,
            SessionError::Idle => Self::SessionIdle,
            SessionError::CsrfMismatch => Self::CsrfMismatch,
        }
    }
}

pub struct OwnerResponse {
    pub status: u16,
    pub body: Value,
    pub issued: Option<SessionTokens>,
}

impl OwnerResponse {
    fn json(status: u16, body: Value) -> Self {
        Self {
            status,
            body,
            issued: None,
        }
    }

    fn error(error: OwnerError) -> Self {
        Self::json(403, json!({ "error": error.code() }))
    }
}

pub struct OwnerSurface {
    registry: Registry,
    sessions: Sessions,
}

impl Default for OwnerSurface {
    fn default() -> Self {
        Self::new()
    }
}

impl OwnerSurface {
    pub fn new() -> Self {
        Self {
            registry: Registry::new(CeremonyConfig::frozen(), Qualification::ProtocolFixtureOnly),
            sessions: Sessions::new(),
        }
    }

    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    pub fn sessions(&self) -> &Sessions {
        &self.sessions
    }

    pub fn sessions_mut(&mut self) -> &mut Sessions {
        &mut self.sessions
    }

    pub fn dispatch(
        &mut self,
        route: FixtureRoute,
        body: &[u8],
        presented: Option<&SessionTokens>,
        now: Instant,
        random: impl FnMut() -> [u8; TOKEN_LEN],
    ) -> OwnerResponse {
        match route {
            FixtureRoute::Shell => OwnerResponse::json(
                200,
                json!({
                    "warning": "unqualified fixture; not owner admission — a same-account process can win an empty registry",
                    "origin": ORIGIN,
                    "rp_id": RP_ID,
                }),
            ),
            FixtureRoute::RegisterStart => self.register_start(now),
            FixtureRoute::RegisterFinish => self.register_finish(body, now, random),
            FixtureRoute::LoginStart => self.login_start(now),
            FixtureRoute::LoginFinish => self.login_finish(body, now, random),
            FixtureRoute::Logout => match presented {
                Some(tokens) => self.logout(tokens, now),
                None => OwnerResponse::error(OwnerError::SessionUnknown),
            },
        }
    }

    fn register_start(&mut self, now: Instant) -> OwnerResponse {
        let ceremony_id = Uuid::new_v4();
        match self.registry.begin_registration(now, ceremony_id) {
            Ok(id) => OwnerResponse::json(
                200,
                json!({
                    "ceremony_id": id.to_string(),
                    "origin": ORIGIN,
                    "rp_id": RP_ID,
                    "timeout_seconds": CEREMONY_TIMEOUT.as_secs(),
                }),
            ),
            Err(error) => OwnerResponse::error(error.into()),
        }
    }

    fn register_finish(
        &mut self,
        body: &[u8],
        now: Instant,
        mut random: impl FnMut() -> [u8; TOKEN_LEN],
    ) -> OwnerResponse {
        let facts = match parse_uv_facts(body) {
            Ok(facts) => facts,
            Err(error) => return OwnerResponse::error(error),
        };
        match self.registry.finish_registration(
            now,
            facts.ceremony_id,
            facts.credential_id,
            facts.user_handle,
            facts.user_verified,
        ) {
            Ok((_stored, bootstrap)) => {
                let tokens = self
                    .sessions
                    .issue(bootstrap.enrolment_winner(), now, &mut random);
                let mut response = OwnerResponse::json(
                    200,
                    json!({
                        "enrolment": bootstrap.honest_summary(),
                        "csrf": hex::encode(tokens.csrf),
                        "cookie_name": SESSION_COOKIE,
                    }),
                );
                response.issued = Some(tokens);
                response
            }
            Err(error) => OwnerResponse::error(error.into()),
        }
    }

    fn login_start(&mut self, now: Instant) -> OwnerResponse {
        let ceremony_id = Uuid::new_v4();
        match self.registry.begin_login(now, ceremony_id) {
            Ok(allow) => OwnerResponse::json(
                200,
                json!({
                    "ceremony_id": ceremony_id.to_string(),
                    "allow_credential": hex::encode(allow),
                    "origin": ORIGIN,
                    "rp_id": RP_ID,
                    "timeout_seconds": CEREMONY_TIMEOUT.as_secs(),
                }),
            ),
            Err(error) => OwnerResponse::error(error.into()),
        }
    }

    fn login_finish(
        &mut self,
        body: &[u8],
        now: Instant,
        mut random: impl FnMut() -> [u8; TOKEN_LEN],
    ) -> OwnerResponse {
        let facts = match parse_uv_facts(body) {
            Ok(facts) => facts,
            Err(error) => return OwnerResponse::error(error),
        };
        match self.registry.finish_login(
            now,
            facts.ceremony_id,
            &facts.credential_id,
            facts.user_handle,
            facts.user_verified,
        ) {
            Ok(subject) => {
                let tokens = self.sessions.issue(subject, now, &mut random);
                let mut response = OwnerResponse::json(
                    200,
                    json!({
                        "csrf": hex::encode(tokens.csrf),
                        "cookie_name": SESSION_COOKIE,
                    }),
                );
                response.issued = Some(tokens);
                response
            }
            Err(error) => OwnerResponse::error(error.into()),
        }
    }

    fn logout(&mut self, presented: &SessionTokens, now: Instant) -> OwnerResponse {
        match self.sessions.authenticate(presented, now) {
            Ok(_) => {
                self.sessions.log_out(&presented.cookie);
                self.registry.abort_pending();
                OwnerResponse::json(200, json!({ "ok": true }))
            }
            Err(error) => OwnerResponse::error(error.into()),
        }
    }
}

struct UvFacts {
    ceremony_id: Uuid,
    credential_id: Vec<u8>,
    user_handle: Uuid,
    user_verified: bool,
}

fn parse_uv_facts(body: &[u8]) -> Result<UvFacts, OwnerError> {
    let value: Value = serde_json::from_slice(body).map_err(|_| OwnerError::BadBody)?;
    let ceremony_id = value
        .get("ceremony_id")
        .and_then(Value::as_str)
        .and_then(|text| Uuid::parse_str(text).ok())
        .ok_or(OwnerError::BadBody)?;
    let credential_hex = value
        .get("credential_id")
        .and_then(Value::as_str)
        .ok_or(OwnerError::BadBody)?;
    let credential_id = hex::decode(credential_hex).map_err(|_| OwnerError::BadBody)?;
    let user_handle = value
        .get("user_handle")
        .and_then(Value::as_str)
        .and_then(|text| Uuid::parse_str(text).ok())
        .ok_or(OwnerError::BadBody)?;
    let user_verified = value
        .get("user_verified")
        .and_then(Value::as_bool)
        .ok_or(OwnerError::BadBody)?;
    Ok(UvFacts {
        ceremony_id,
        credential_id,
        user_handle,
        user_verified,
    })
}

//! Build closed-profile verified proofs for reservation tests.
//!
//! Pure verification is repeated independently; one-use is the redb
//! reservation. This helper never calls `authorize_and_consume*`.

use std::time::{Instant, SystemTime, UNIX_EPOCH};

use chrono::Duration;
use serde_json::json;
use sovereign_capability::approval::SignedApprovalV1;
use sovereign_capability::v2::{
    CapabilityIssuerV2, CapabilityTokenV2, CapabilityV2IssueOptions, CapabilityV2IssueRequest,
    CapabilityV2ValidationContext, CapabilityValidatorV2, TrustedClock, VerifiedApprovalV1,
    VerifiedCapabilityV2,
};
use sovereign_identity::{ApprovalRole, AuthorityRole, KeyValidity, RoleTrustStore, TypedSigner};
use sovereign_synthetic_owner_effect::{
    persist_prepared, EffectIntentId, FixtureOwner, FixtureRoute, IntentState, OwnerSurface,
    PreparedSnapshot, ReservationContext, ReservationView, ReserveError, SessionBinding,
    SessionTokens, FIXTURE_AUDIENCE, FIXTURE_ISSUER, FIXTURE_VENTURE, TOKEN_LEN,
};
use uuid::Uuid;

const AUTHORITY_ISSUER: &str = "fixture.unqualified.authority";
const AUTHORITY_SECRET: [u8; 32] = [0x41; 32];
const CREDENTIAL: &str = "cred";

#[derive(Clone, Copy)]
struct UnixClock(i64);

impl TrustedClock for UnixClock {
    fn now_unix(&self) -> i64 {
        self.0
    }
}

pub struct Harness {
    pub owner: FixtureOwner,
    pub session_tokens: SessionTokens,
    now: Instant,
}

pub struct Issued {
    pub intent_id: EffectIntentId,
    pub context: ReservationContext,
    pub token: CapabilityTokenV2,
    pub signed_approval: SignedApprovalV1,
    pub token_id: Uuid,
    pub approval_id: Uuid,
    pub idempotency_key: Uuid,
    pub approval_expires_at_unix: i64,
}

fn counter() -> impl FnMut() -> [u8; TOKEN_LEN] {
    let mut next = 0u8;
    move || {
        next = next.wrapping_add(1);
        [next; TOKEN_LEN]
    }
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

pub fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

pub fn register_session(
    surface: &mut OwnerSurface,
    now: Instant,
    credential: &str,
) -> SessionTokens {
    let handle = Uuid::new_v4();
    let start = surface.dispatch(FixtureRoute::RegisterStart, b"{}", None, now, counter());
    let ceremony = start.body["ceremony_id"].as_str().unwrap().to_owned();
    let body = finish_body(&ceremony, credential, handle, true);
    let finish = surface.dispatch(FixtureRoute::RegisterFinish, &body, None, now, counter());
    assert_eq!(finish.status, 200);
    finish.issued.expect("registration issues a session")
}

fn authority_signer() -> TypedSigner<AuthorityRole> {
    TypedSigner::<AuthorityRole>::from_secret_bytes(AUTHORITY_ISSUER, AUTHORITY_SECRET).unwrap()
}

impl Harness {
    pub fn boot(root: &std::path::Path) -> Self {
        let mut owner = FixtureOwner::boot(root).expect("boot fixture owner");
        let now = Instant::now();
        let session_tokens = register_session(owner.surface_mut(), now, CREDENTIAL);
        Self {
            owner,
            session_tokens,
            now,
        }
    }

    pub fn session(&mut self) -> SessionBinding {
        self.owner
            .surface_mut()
            .require_session(&self.session_tokens, self.now)
            .expect("live session")
    }

    #[allow(dead_code)]
    pub fn logout(&mut self) {
        let _ = self.owner.surface_mut().dispatch(
            FixtureRoute::Logout,
            b"{}",
            Some(&self.session_tokens),
            self.now,
            counter(),
        );
    }

    pub fn issue_for_new_intent(&mut self) -> Issued {
        let now_unix = unix_now();
        let (intent_id, _) = self
            .owner
            .surface_mut()
            .prepare_effect(&self.session_tokens, b"{}", self.now)
            .expect("prepare");
        let ceremony = self
            .owner
            .surface_mut()
            .start_effect_approval(&self.session_tokens, intent_id, self.now)
            .expect("start approval");
        let grant = self
            .owner
            .surface_mut()
            .finish_effect_approval(
                &self.session_tokens,
                &finish_body(&ceremony.to_string(), CREDENTIAL, Uuid::nil(), true),
                self.now,
            )
            .expect("fresh UV grant");
        let session = self.session();
        let signed_approval = {
            let (prepared, policy) = self
                .owner
                .surface()
                .effects()
                .bindings(intent_id)
                .expect("bindings");
            self.owner
                .bridge()
                .approve_invocation(grant, &session, prepared, policy, self.now, now_unix)
                .expect("RFC 0003 approval")
        };
        let policy_decision_id = {
            let (_, policy) = self
                .owner
                .surface()
                .effects()
                .bindings(intent_id)
                .expect("bindings");
            policy.decision_id()
        };
        let snapshot = PreparedSnapshot {
            intent_id,
            session_id: session.session_id,
            logout_epoch: session.logout_epoch,
            signer_epoch: self.owner.bridge().signer_epoch(),
            fixture_generation: session.fixture_generation,
            policy_decision_id,
        };
        {
            let store = self.owner.open_store().expect("store");
            persist_prepared(&store, &snapshot).expect("persist Prepared");
        }

        let token = self.issue_token(&session, intent_id, &signed_approval, now_unix);
        let (capability, approval) = self.verify(&token, &signed_approval, intent_id, now_unix);
        Issued {
            intent_id,
            context: ReservationContext {
                now_unix,
                session_id: session.session_id,
                logout_epoch: session.logout_epoch,
                signer_epoch: self.owner.bridge().signer_epoch(),
                fixture_generation: session.fixture_generation,
                intent_id,
            },
            token,
            signed_approval,
            token_id: capability.token_id(),
            approval_id: approval.approval_id(),
            idempotency_key: capability.idempotency_key(),
            approval_expires_at_unix: approval.expires_at_unix(),
        }
    }

    fn issue_token(
        &self,
        session: &SessionBinding,
        intent_id: EffectIntentId,
        signed_approval: &SignedApprovalV1,
        now_unix: i64,
    ) -> CapabilityTokenV2 {
        let record = self.owner.bridge().public_trust_record();
        let validity = KeyValidity::new(now_unix - 60, now_unix + 7_200).unwrap();
        let mut issuance_trust = RoleTrustStore::<ApprovalRole>::new();
        issuance_trust
            .add_key(
                record.issuer(),
                record.public_key_bytes().unwrap(),
                validity,
            )
            .unwrap();
        let issuer =
            CapabilityIssuerV2::new(authority_signer(), FIXTURE_AUDIENCE, UnixClock(now_unix))
                .unwrap()
                .with_approval_trust(issuance_trust, FIXTURE_ISSUER)
                .unwrap();
        let (prepared, policy) = self
            .owner
            .surface()
            .effects()
            .bindings(intent_id)
            .expect("bindings");
        issuer
            .issue_approved(
                CapabilityV2IssueRequest {
                    venture_id: FIXTURE_VENTURE,
                    subject_id: &session.subject.to_string(),
                    session_id: session.session_id,
                    policy_decision: policy,
                    prepared_invocation: prepared,
                    options: CapabilityV2IssueOptions {
                        ttl: Duration::seconds(60),
                        idempotency_key: policy.idempotency_key(),
                    },
                },
                signed_approval,
            )
            .expect("issue closed-profile token")
    }

    pub fn verify(
        &self,
        token: &CapabilityTokenV2,
        signed_approval: &SignedApprovalV1,
        intent_id: EffectIntentId,
        now_unix: i64,
    ) -> (VerifiedCapabilityV2, VerifiedApprovalV1) {
        let (prepared, policy) = self
            .owner
            .surface()
            .effects()
            .bindings(intent_id)
            .expect("bindings");
        let mut authority_trust = RoleTrustStore::<AuthorityRole>::new();
        authority_trust
            .trust_signer(
                &authority_signer(),
                KeyValidity::new(now_unix - 60, now_unix + 7_200).unwrap(),
            )
            .unwrap();
        let record = self.owner.bridge().public_trust_record();
        let mut approval_trust = RoleTrustStore::<ApprovalRole>::new();
        approval_trust
            .add_key(
                record.issuer(),
                record.public_key_bytes().unwrap(),
                KeyValidity::new(now_unix - 60, now_unix + 7_200).unwrap(),
            )
            .unwrap();
        let validator = CapabilityValidatorV2::new(
            authority_trust,
            AUTHORITY_ISSUER,
            FIXTURE_AUDIENCE,
            UnixClock(now_unix),
        )
        .unwrap()
        .with_approval_trust(approval_trust, FIXTURE_ISSUER)
        .unwrap();
        let subject = policy.subject_id().to_owned();
        let session_id = policy.session_id();
        let (capability, approval) = validator
            .verify_approved(
                token,
                CapabilityV2ValidationContext {
                    venture_id: FIXTURE_VENTURE,
                    subject_id: &subject,
                    session_id,
                    policy_decision: policy,
                    prepared_invocation: prepared,
                },
                Some(signed_approval),
            )
            .expect("pure verify");
        (
            capability,
            approval.expect("closed profile requires approval"),
        )
    }
}

#[allow(dead_code)]
pub fn refuse<T>(result: Result<T, ReserveError>, why: &str) -> ReserveError {
    match result {
        Err(error) => error,
        Ok(_) => panic!("{why}"),
    }
}

pub fn none_of_the_reservation(view: &ReservationView) -> bool {
    !view.approval_claimed
        && !view.token_claimed
        && view.idempotency_bound_to.is_none()
        && view.intent_state != Some(IntentState::AuthorityReserved)
}

pub fn all_of_the_reservation(
    view: &ReservationView,
    intent_id: EffectIntentId,
    approval_expires_at_unix: i64,
) -> bool {
    view.approval_claimed
        && view.token_claimed
        && view.idempotency_bound_to == Some(intent_id.as_uuid())
        && view.intent_state == Some(IntentState::AuthorityReserved)
        && view.approval_expires_at_unix == Some(approval_expires_at_unix)
        && view.authority_uses_remaining == 7
}

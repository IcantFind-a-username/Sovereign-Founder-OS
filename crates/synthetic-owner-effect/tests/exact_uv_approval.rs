//! v01-D04: seal intent and bridge fresh UV to RFC 0003.
//!
//! Design Accept ≠ product Current. Not 1C0, Exact Effect, or dispatch.

#![cfg(feature = "owner-effect-fixture")]

use std::time::Instant;

use serde_json::json;
use sovereign_synthetic_owner_effect::{
    ApprovalBridge, EffectIntentId, FixtureOwner, FixtureRoute, FreshUvGrant, OwnerError,
    OwnerSurface, SealedPayload, SessionTokens, TOKEN_LEN,
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

fn finish_body(ceremony_id: &str, credential: &str, handle: Uuid, user_verified: bool) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "ceremony_id": ceremony_id,
        "credential_id": hex::encode(credential.as_bytes()),
        "user_handle": handle.to_string(),
        "user_verified": user_verified,
    }))
    .unwrap()
}

fn register_session(surface: &mut OwnerSurface, now: Instant, credential: &str) -> SessionTokens {
    let handle = Uuid::new_v4();
    let start = surface.dispatch(FixtureRoute::RegisterStart, b"{}", None, now, counter());
    let ceremony = start.body["ceremony_id"].as_str().unwrap().to_owned();
    let body = finish_body(&ceremony, credential, handle, true);
    let finish = surface.dispatch(FixtureRoute::RegisterFinish, &body, None, now, counter());
    assert_eq!(finish.status, 200);
    finish.issued.expect("registration issues a session")
}

fn grant_for(
    surface: &mut OwnerSurface,
    tokens: &SessionTokens,
    now: Instant,
    credential: &str,
) -> (EffectIntentId, FreshUvGrant) {
    let (intent_id, _) = surface.prepare_effect(tokens, b"{}", now).unwrap();
    let ceremony = surface
        .start_effect_approval(tokens, intent_id, now)
        .unwrap();
    let grant = surface
        .finish_effect_approval(
            tokens,
            &finish_body(&ceremony.to_string(), credential, Uuid::nil(), true),
            now,
        )
        .unwrap();
    (intent_id, grant)
}

#[test]
fn intent_is_allocated_before_content_access() {
    let source = include_str!("../src/effect.rs");
    let allocate = source.find("EffectIntentId::allocate()").expect("allocate");
    let compose = source.find("SealedPayload::compose").expect("compose");
    assert!(
        allocate < compose,
        "content was read before the intent id existed"
    );
    let payload_source = include_str!("../src/sealed.rs");
    assert!(payload_source.contains("To: {RECIPIENT}"));
    assert!(
        !payload_source.contains("pub fn compose(")
            || payload_source.contains("intent_id: EffectIntentId")
    );
}

#[test]
fn header_injection_in_prepare_is_refused() {
    let mut surface = OwnerSurface::new();
    let now = Instant::now();
    let tokens = register_session(&mut surface, now, "cred");
    let injected = serde_json::to_vec(&json!({
        "to": "attacker@evil.example",
        "subject": "injected"
    }))
    .unwrap();
    let error = surface.prepare_effect(&tokens, &injected, now).unwrap_err();
    assert_eq!(error, OwnerError::HeaderInjection);

    let generic = serde_json::to_vec(&json!({ "extra": "field" })).unwrap();
    assert_eq!(
        surface.prepare_effect(&tokens, &generic, now).unwrap_err(),
        OwnerError::GenericInput
    );
}

#[test]
fn substitution_of_recipient_or_bytes_requires_a_new_intent() {
    let bridge = ApprovalBridge::generate().unwrap();
    let mut surface = OwnerSurface::new();
    surface.bind_live_signer(bridge.signer_epoch(), 1);
    let now = Instant::now();
    let tokens = register_session(&mut surface, now, "cred");
    let (intent_a, grant) = grant_for(&mut surface, &tokens, now, "cred");
    let (intent_b, _) = surface.prepare_effect(&tokens, b"{}", now).unwrap();
    assert_ne!(intent_a.file_stem(), intent_b.file_stem());

    let session = surface.require_session(&tokens, now).unwrap();
    let (prepared_b, policy_b) = surface.effects().bindings(intent_b).unwrap();
    let error = bridge
        .approve_invocation(grant, &session, prepared_b, policy_b, now, 1_800_000_000)
        .unwrap_err();
    assert_eq!(error, OwnerError::IntentMismatch);
}

#[test]
fn session_alone_cannot_produce_rfc0003_approval() {
    let mut surface = OwnerSurface::new();
    let now = Instant::now();
    let tokens = register_session(&mut surface, now, "cred");
    let (intent_id, _) = surface.prepare_effect(&tokens, b"{}", now).unwrap();
    let _ = intent_id;
    let error = match surface.finish_effect_approval(
        &tokens,
        &finish_body(&Uuid::new_v4().to_string(), "cred", Uuid::nil(), true),
        now,
    ) {
        Err(error) => error,
        Ok(_) => panic!("session-alone finish must not mint a grant"),
    };
    assert_eq!(error, OwnerError::UnknownCeremony);

    let source = include_str!("../src/approval_bridge.rs");
    assert!(source.contains("pub fn approve_invocation"));
    assert!(!source.contains("pub fn sign("));
    assert!(!source.contains("pub fn signer("));
}

#[test]
fn grant_replay_is_refused() {
    let mut surface = OwnerSurface::new();
    let now = Instant::now();
    let tokens = register_session(&mut surface, now, "cred");
    let (intent_id, _) = surface.prepare_effect(&tokens, b"{}", now).unwrap();
    let ceremony = surface
        .start_effect_approval(&tokens, intent_id, now)
        .unwrap();
    let body = finish_body(&ceremony.to_string(), "cred", Uuid::nil(), true);
    surface.finish_effect_approval(&tokens, &body, now).unwrap();
    let error = match surface.finish_effect_approval(&tokens, &body, now) {
        Err(error) => error,
        Ok(_) => panic!("a consumed challenge must not mint a second grant"),
    };
    assert_eq!(error, OwnerError::GrantReplay);
}

#[test]
fn signer_epoch_mismatch_is_refused() {
    let live = ApprovalBridge::generate().unwrap();
    let other = ApprovalBridge::generate().unwrap();
    let mut surface = OwnerSurface::new();
    surface.bind_live_signer(live.signer_epoch(), 1);
    let now = Instant::now();
    let tokens = register_session(&mut surface, now, "cred");
    let (intent_id, grant) = grant_for(&mut surface, &tokens, now, "cred");
    let session = surface.require_session(&tokens, now).unwrap();
    let (prepared, policy) = surface.effects().bindings(intent_id).unwrap();
    let error = other
        .approve_invocation(grant, &session, prepared, policy, now, 1_800_000_000)
        .unwrap_err();
    assert_eq!(error, OwnerError::EpochMismatch);
}

#[test]
fn restart_invalidates_in_memory_grants() {
    let dir = tempfile::tempdir().unwrap();
    let root = root::marked_root(dir.path());
    let mut first = FixtureOwner::boot(&root).unwrap();
    let now = Instant::now();
    let tokens = register_session(first.surface_mut(), now, "cred");
    let (_intent_id, grant) = grant_for(first.surface_mut(), &tokens, now, "cred");
    drop(first);

    let mut second = FixtureOwner::boot(&root).unwrap();
    let tokens_b = register_session(second.surface_mut(), now, "other");
    let (intent_b, _) = second
        .surface_mut()
        .prepare_effect(&tokens_b, b"{}", now)
        .unwrap();
    let session = second
        .surface_mut()
        .require_session(&tokens_b, now)
        .unwrap();
    let (prepared, policy) = second.surface().effects().bindings(intent_b).unwrap();
    let error = second
        .bridge()
        .approve_invocation(grant, &session, prepared, policy, now, 1_800_000_000)
        .unwrap_err();
    assert_eq!(error, OwnerError::EpochMismatch);
}

#[test]
fn compile_fail_access_to_private_proof_grant_payload_and_constructors() {
    let grant_source = include_str!("../src/grant.rs");
    assert!(grant_source.contains("pub struct FreshUvGrant {"));
    assert!(grant_source.contains("session_id: Uuid,"));
    assert!(!grant_source.contains("pub session_id"));
    assert!(!grant_source.contains("pub fn new"));
    assert!(grant_source.contains("assert_not_impl_any!"));

    let payload_source = include_str!("../src/sealed.rs");
    assert!(payload_source.contains("pub struct SealedPayload {"));
    assert!(payload_source.contains("bytes: Vec<u8>,"));
    assert!(!payload_source.contains("pub fn sealed_bytes"));
    assert!(!payload_source.contains("pub fn content"));
    assert!(!payload_source.contains("impl Clone for SealedPayload"));
    assert!(!payload_source.contains("impl Serialize for SealedPayload"));

    let bridge_source = include_str!("../src/approval_bridge.rs");
    assert!(bridge_source.contains("pub fn approve_invocation"));
    let matches = bridge_source.matches("sign_cose").count();
    assert!(
        matches >= 1,
        "the bridge must still persist public attestations"
    );
}

#[test]
fn fresh_uv_then_bridge_emits_rfc0003_approval() {
    let bridge = ApprovalBridge::generate().unwrap();
    let mut surface = OwnerSurface::new();
    surface.bind_live_signer(bridge.signer_epoch(), 1);
    let now = Instant::now();
    let tokens = register_session(&mut surface, now, "cred");
    let (intent_id, grant) = grant_for(&mut surface, &tokens, now, "cred");
    let session = surface.require_session(&tokens, now).unwrap();
    let (prepared, policy) = surface.effects().bindings(intent_id).unwrap();
    let approval = bridge
        .approve_invocation(grant, &session, prepared, policy, now, 1_800_000_000)
        .expect("fresh UV must mint RFC 0003 evidence");
    assert!(!approval.as_bytes().is_empty());
}

#[test]
fn sealed_payload_never_prints_its_message() {
    let payload = SealedPayload::compose(EffectIntentId::allocate());
    let rendered = format!("{payload:?}");
    assert!(rendered.contains("<protected>"));
    for fragment in ["fixture-recipient", "CANARY", "From:", "Subject:"] {
        assert!(
            !rendered.contains(fragment),
            "the rendering leaked {fragment:?}: {rendered}"
        );
    }
}

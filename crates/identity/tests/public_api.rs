//! Integration boundary for `sovereign-identity`: exercises the key lifecycle
//! through the crate's public re-exports only, with no access to private
//! internals. The in-crate `src/tests.rs` suite reaches module internals; this
//! file is the contract an external crate actually depends on, so a change
//! that breaks a downstream consumer fails here even when the in-crate tests
//! still pass.

use sovereign_identity::{
    device_id_from_public_key_b64, ApprovalRole, AuditRole, AuthorityRole, DeviceIdentity,
    IdentityError, KeyValidity, RoleTrustStore, TrustStatus, TypedSigner,
};

const NOW: i64 = 1_800_000_000;

fn window_around(now: i64) -> KeyValidity {
    KeyValidity::new(now - 60, now + 3_600).expect("valid interval")
}

// --- Device identity lifecycle -------------------------------------------

#[test]
fn a_device_identity_round_trips_through_save_and_load() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("device.json");

    let device = DeviceIdentity::generate();
    let device_id = device.device_id().to_string();
    let public_key_b64 = device.public_key_b64().to_string();
    device.save(&path).unwrap();

    let reloaded = DeviceIdentity::load(&path).unwrap();
    assert_eq!(reloaded.device_id(), device_id);
    assert_eq!(reloaded.public_key_b64(), public_key_b64);
}

#[test]
fn the_device_id_is_derivable_from_the_public_key_alone() {
    // A verifier holding only the exported public key can recompute the
    // device id and confirm a bundle really belongs to that device.
    let device = DeviceIdentity::generate();
    let recomputed = device_id_from_public_key_b64(device.public_key_b64()).unwrap();
    assert_eq!(recomputed, device.device_id());
}

#[test]
fn legacy_signatures_verify_and_reject_tampering_through_the_public_api() {
    let device = DeviceIdentity::generate();
    let message = b"transfer authority to the founder";
    let signature = device.sign_legacy_v1(message);

    DeviceIdentity::verify_legacy_v1(device.public_key_b64(), message, &signature).unwrap();

    let tampered = b"transfer authority to the attacker";
    assert!(matches!(
        DeviceIdentity::verify_legacy_v1(device.public_key_b64(), tampered, &signature),
        Err(IdentityError::VerificationFailed)
    ));
}

#[test]
fn loading_a_nonexistent_identity_is_an_error_not_a_panic() {
    let dir = tempfile::tempdir().unwrap();
    assert!(DeviceIdentity::load(&dir.path().join("absent.json")).is_err());
}

// --- Typed signer -------------------------------------------------------

#[test]
fn a_signer_from_the_same_secret_bytes_is_deterministic() {
    let secret = [7u8; 32];
    let first = TypedSigner::<ApprovalRole>::from_secret_bytes("owner.local", secret).unwrap();
    let second = TypedSigner::<ApprovalRole>::from_secret_bytes("owner.local", secret).unwrap();
    assert_eq!(first.key_id(), second.key_id());
    assert_eq!(first.public_key_bytes(), second.public_key_bytes());
    assert_eq!(first.issuer(), "owner.local");
}

// --- Trust store: the full verify lifecycle -----------------------------

#[test]
fn a_trusted_signer_verifies_and_exposes_its_bound_payload() {
    let signer = TypedSigner::<ApprovalRole>::generate("owner.local").unwrap();
    let mut store = RoleTrustStore::<ApprovalRole>::new();
    let key_id = store.trust_signer(&signer, window_around(NOW)).unwrap();

    let payload = b"approve exactly this invocation";
    let encoded = signer.sign_cose(payload).unwrap();

    let verified = store.verify(&encoded, "owner.local", NOW).unwrap();
    assert_eq!(verified.issuer(), "owner.local");
    assert_eq!(verified.key_id(), &key_id);
    assert_eq!(verified.payload(), payload);
    assert_eq!(verified.into_payload(), payload);
}

#[test]
fn a_tampered_payload_fails_verification() {
    let signer = TypedSigner::<ApprovalRole>::generate("owner.local").unwrap();
    let mut store = RoleTrustStore::<ApprovalRole>::new();
    store.trust_signer(&signer, window_around(NOW)).unwrap();

    let mut encoded = signer.sign_cose(b"original payload").unwrap();
    // Flip a byte late in the encoding (inside the signed payload/signature
    // region) and require the verifier to reject it, whatever the exact
    // structural or cryptographic failure.
    let last = encoded.len() - 1;
    encoded[last] ^= 0x01;
    assert!(store.verify(&encoded, "owner.local", NOW).is_err());
}

#[test]
fn an_unregistered_key_is_unknown() {
    let signer = TypedSigner::<ApprovalRole>::generate("owner.local").unwrap();
    let encoded = signer.sign_cose(b"payload").unwrap();
    // Empty store: the kid selects nothing.
    let store = RoleTrustStore::<ApprovalRole>::new();
    assert!(matches!(
        store.verify(&encoded, "owner.local", NOW),
        Err(IdentityError::UnknownKeyId)
    ));
}

#[test]
fn the_expected_issuer_must_match_the_trusted_record() {
    let signer = TypedSigner::<ApprovalRole>::generate("owner.local").unwrap();
    let mut store = RoleTrustStore::<ApprovalRole>::new();
    store.trust_signer(&signer, window_around(NOW)).unwrap();
    let encoded = signer.sign_cose(b"payload").unwrap();

    assert!(matches!(
        store.verify(&encoded, "someone-else.local", NOW),
        Err(IdentityError::IssuerMismatch)
    ));
}

#[test]
fn a_key_is_rejected_before_and_after_its_validity_window() {
    let signer = TypedSigner::<ApprovalRole>::generate("owner.local").unwrap();
    let mut store = RoleTrustStore::<ApprovalRole>::new();
    let validity = KeyValidity::new(NOW, NOW + 100).unwrap();
    store.trust_signer(&signer, validity).unwrap();
    let encoded = signer.sign_cose(b"payload").unwrap();

    assert!(matches!(
        store.verify(&encoded, "owner.local", NOW - 1),
        Err(IdentityError::KeyNotYetValid)
    ));
    // The upper bound is exclusive.
    assert!(matches!(
        store.verify(&encoded, "owner.local", NOW + 100),
        Err(IdentityError::KeyExpired)
    ));
    // Inside the window it verifies.
    store.verify(&encoded, "owner.local", NOW + 50).unwrap();
}

#[test]
fn a_revoked_key_stops_verifying() {
    let signer = TypedSigner::<ApprovalRole>::generate("owner.local").unwrap();
    let mut store = RoleTrustStore::<ApprovalRole>::new();
    let key_id = store.trust_signer(&signer, window_around(NOW)).unwrap();
    let encoded = signer.sign_cose(b"payload").unwrap();

    store.verify(&encoded, "owner.local", NOW).unwrap();
    store.revoke(&key_id).unwrap();
    assert!(matches!(
        store.verify(&encoded, "owner.local", NOW),
        Err(IdentityError::KeyRevoked)
    ));
}

#[test]
fn set_status_can_restore_a_revoked_key() {
    let signer = TypedSigner::<ApprovalRole>::generate("owner.local").unwrap();
    let mut store = RoleTrustStore::<ApprovalRole>::new();
    let key_id = store.trust_signer(&signer, window_around(NOW)).unwrap();
    let encoded = signer.sign_cose(b"payload").unwrap();

    store.set_status(&key_id, TrustStatus::Revoked).unwrap();
    assert!(store.verify(&encoded, "owner.local", NOW).is_err());
    store.set_status(&key_id, TrustStatus::Active).unwrap();
    store.verify(&encoded, "owner.local", NOW).unwrap();
}

#[test]
fn trusting_the_same_key_twice_is_a_duplicate() {
    let signer = TypedSigner::<ApprovalRole>::generate("owner.local").unwrap();
    let mut store = RoleTrustStore::<ApprovalRole>::new();
    store.trust_signer(&signer, window_around(NOW)).unwrap();
    assert!(matches!(
        store.trust_signer(&signer, window_around(NOW)),
        Err(IdentityError::DuplicateKeyId)
    ));
}

#[test]
fn an_inverted_validity_interval_is_rejected() {
    assert!(matches!(
        KeyValidity::new(NOW + 100, NOW),
        Err(IdentityError::InvalidValidity)
    ));
    assert!(matches!(
        KeyValidity::new(NOW, NOW),
        Err(IdentityError::InvalidValidity)
    ));
}

#[test]
fn revoking_an_unknown_key_is_an_error() {
    let mut store = RoleTrustStore::<ApprovalRole>::new();
    assert!(matches!(
        store.revoke(&[0u8; 32]),
        Err(IdentityError::UnknownKeyId)
    ));
}

// --- Role domain separation ---------------------------------------------

#[test]
fn a_signature_from_one_role_cannot_verify_under_another_roles_store() {
    // The same 32 secret bytes, signed under the authority role, must be
    // unverifiable by an approval-role store: the key id is role-scoped, so
    // an approval store never even finds the key. This is the security
    // property the sealed role set exists to enforce, checked through the
    // public boundary.
    let secret = [42u8; 32];
    let authority_signer =
        TypedSigner::<AuthorityRole>::from_secret_bytes("owner.local", secret).unwrap();
    let encoded = authority_signer.sign_cose(b"a capability payload").unwrap();

    let approval_signer =
        TypedSigner::<ApprovalRole>::from_secret_bytes("owner.local", secret).unwrap();
    let mut approval_store = RoleTrustStore::<ApprovalRole>::new();
    approval_store
        .trust_signer(&approval_signer, window_around(NOW))
        .unwrap();

    assert!(matches!(
        approval_store.verify(&encoded, "owner.local", NOW),
        Err(IdentityError::UnknownKeyId)
    ));

    // And it verifies under the correct role's store.
    let mut authority_store = RoleTrustStore::<AuthorityRole>::new();
    authority_store
        .trust_signer(&authority_signer, window_around(NOW))
        .unwrap();
    authority_store
        .verify(&encoded, "owner.local", NOW)
        .unwrap();
}

#[test]
fn a_device_key_can_become_an_audit_signer_and_verify_under_the_audit_role() {
    // The device → audit-signer bridge is a public lifecycle step: the same
    // device key signs audit events, and the derived public key still maps
    // back to the device id.
    let device = DeviceIdentity::generate();
    let public_key_b64 = device.public_key_b64().to_string();
    let device_id = device.device_id().to_string();

    let audit_signer = device.into_audit_signer("founder-device.local").unwrap();
    assert_eq!(audit_signer.public_key_b64(), public_key_b64);
    assert_eq!(
        device_id_from_public_key_b64(&audit_signer.public_key_b64()).unwrap(),
        device_id
    );

    let mut store = RoleTrustStore::<AuditRole>::new();
    store
        .trust_signer(&audit_signer, window_around(NOW))
        .unwrap();
    let encoded = audit_signer.sign_cose(b"an audit event body").unwrap();
    let verified = store.verify(&encoded, "founder-device.local", NOW).unwrap();
    assert_eq!(verified.payload(), b"an audit event body");
}

//! v01-D03: lock → signer → redb, epoch rotation, no secrets, historical verify-only.

#![cfg(feature = "owner-effect-fixture")]

use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use sovereign_authority::broker::process_lock::LOCK_FILE;
use sovereign_authority::broker::store::STORE_FILE;
use sovereign_identity::{ApprovalRole, TypedSigner};
use sovereign_synthetic_owner_effect::{
    persist_public_trust, ApprovalBridge, FixtureOwner, ProcessBoundary, PublicTrustRecord,
    FIXTURE_ISSUER, HISTORICAL_VERIFY_ONLY, SIGNER_NEEDLE, UNQUALIFIED_LABEL,
};

#[path = "support/root.rs"]
mod root;

#[test]
fn lock_then_signer_then_redb_open() {
    let dir = tempfile::tempdir().unwrap();
    let root = root::marked_root(dir.path());

    let locked = ProcessBoundary::acquire(&root)
        .unwrap()
        .with_signer()
        .unwrap();
    assert!(
        root.join(LOCK_FILE).is_file(),
        "the lock must exist before the signer is used"
    );
    assert!(
        !root.join(STORE_FILE).exists(),
        "redb opened before the live signer was generated"
    );
    let epoch = locked.bridge().signer_epoch();
    assert_ne!(epoch, [0u8; 16]);

    let owner = locked.persist().unwrap();
    assert!(root.join(STORE_FILE).is_file());
    let historical = owner.load_historical_trust().unwrap();
    assert_eq!(historical.len(), 1);
    let record = historical.records().next().unwrap();
    assert_eq!(record.label(), UNQUALIFIED_LABEL);
    assert_eq!(record.status(), HISTORICAL_VERIFY_ONLY);
    assert_eq!(record.signer_epoch_hex(), hex::encode(epoch));
}

#[test]
fn restart_rotates_signer_epoch() {
    let dir = tempfile::tempdir().unwrap();
    let root = root::marked_root(dir.path());
    let first = FixtureOwner::boot(&root).unwrap();
    let epoch_a = first.bridge().signer_epoch();
    let key_a = first.bridge().public_trust_record().key_id_hex().to_owned();
    drop(first);

    let second = FixtureOwner::boot(&root).unwrap();
    let epoch_b = second.bridge().signer_epoch();
    assert_ne!(epoch_a, epoch_b, "a restart reused the signer epoch");
    let historical = second.load_historical_trust().unwrap();
    assert_eq!(historical.len(), 2);
    let keys: Vec<_> = historical
        .records()
        .map(|record| record.key_id_hex().to_owned())
        .collect();
    assert!(keys.contains(&key_a));
    assert_ne!(
        second.bridge().public_trust_record().key_id_hex(),
        key_a.as_str()
    );
}

#[test]
fn historical_keys_are_verify_only() {
    let dir = tempfile::tempdir().unwrap();
    let root = root::marked_root(dir.path());
    let first = FixtureOwner::boot(&root).unwrap();
    drop(first);
    let second = FixtureOwner::boot(&root).unwrap();
    let now_unix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    second
        .load_historical_trust()
        .unwrap()
        .verify_stored_attestations(now_unix)
        .expect("historical public keys must verify their attestations");

    let source = include_str!("../src/approval_bridge.rs");
    for forbidden in [
        "pub fn signer(",
        "pub fn secret",
        "pub fn sign(",
        "pub fn approve_invocation",
        "pub fn from_secret",
        "impl Clone for ApprovalBridge",
        "activate_epoch",
    ] {
        assert!(
            !source.contains(forbidden),
            "ApprovalBridge exposes {forbidden}, which would export or reactivate the signer"
        );
    }
    let rendered = format!("{:?}", second.bridge());
    assert!(rendered.contains("redacted"), "{rendered}");
    assert!(
        !rendered.contains(&hex::encode(second.bridge().signer_epoch())),
        "debug printed the live epoch"
    );
}

#[test]
fn no_secret_bytes_in_redb_or_debug_log() {
    let dir = tempfile::tempdir().unwrap();
    let root = root::marked_root(dir.path());
    let secret = [0x5Au8; 32];
    let signer = TypedSigner::<ApprovalRole>::from_secret_bytes(FIXTURE_ISSUER, secret).unwrap();
    let owner = FixtureOwner::boot(&root).unwrap();
    let record = PublicTrustRecord::new(
        [0x11; 16],
        signer.public_key_bytes(),
        *signer.key_id(),
        FIXTURE_ISSUER,
    );
    {
        let store = owner.open_store().unwrap();
        persist_public_trust(&store, &record, &[]).unwrap();
    }
    let bytes = fs::read(root.join(STORE_FILE)).unwrap();
    assert!(
        !bytes.windows(32).any(|window| window == secret.as_slice()),
        "known secret bytes were persisted to redb"
    );
    let text = String::from_utf8_lossy(&bytes);
    assert!(text.contains(&hex::encode(signer.public_key_bytes())));
    for forbidden in ["signing_key", "secret_key", "\"secret\""] {
        assert!(
            !text.contains(forbidden),
            "redb carried secret-shaped field {forbidden}"
        );
    }

    let bridge = ApprovalBridge::generate().unwrap();
    let rendered = format!("{bridge:?}");
    assert!(rendered.contains("redacted"));
    assert!(rendered.contains(SIGNER_NEEDLE));
    let public_hex = bridge.public_trust_record().public_key_hex().to_owned();
    assert!(
        !rendered.contains(&public_hex),
        "debug printed the public key, which is adjacent to key material: {rendered}"
    );
}

#[test]
fn secret_key_zeroization_is_in_the_resolved_graph() {
    let output = std::process::Command::new("cargo")
        .args([
            "tree",
            "-p",
            "sovereign-synthetic-owner-effect",
            "--no-default-features",
            "--features",
            "owner-effect-fixture",
            "-e",
            "normal",
            "--locked",
        ])
        .output()
        .expect("cargo tree must run");
    assert!(output.status.success(), "cargo tree failed");
    let tree = String::from_utf8_lossy(&output.stdout);
    assert!(
        tree.contains("zeroize"),
        "the fixture graph dropped secret-key zeroization support:\n{tree}"
    );
    assert!(
        tree.contains("ed25519-dalek"),
        "the live signer must come from ed25519-dalek:\n{tree}"
    );
}

#[test]
fn approval_bridge_is_closed() {
    let source = include_str!("../src/approval_bridge.rs");
    assert!(source.contains("pub struct ApprovalBridge"));
    assert!(source.contains("signer: TypedSigner<ApprovalRole>"));
    assert!(source.contains("impl fmt::Debug for ApprovalBridge"));
    assert!(
        !source.contains("impl Clone for ApprovalBridge"),
        "ApprovalBridge must not be cloneable"
    );
    assert!(
        !source.contains("impl Serialize for ApprovalBridge"),
        "ApprovalBridge must not be serializable"
    );
}

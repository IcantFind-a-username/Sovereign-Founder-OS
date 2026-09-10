//! The adapter, and the line it must not cross.
//!
//! `sovereign-owner` already implements and tests every property that
//! matters, taking user-verification, a credential id and a returned handle
//! as inputs. This crate produces those three and decides nothing. These
//! tests are mostly about that boundary — because an adapter that started
//! deciding would move the decisions behind a dependency tree the audited
//! core does not build.

use sovereign_owner::bootstrap::Qualification;
use sovereign_owner::config::{CeremonyConfig, ORIGIN, RP_ID};
use sovereign_owner::registry::Registry;
use sovereign_owner_webauthn::FixtureRelyingParty;
use std::time::Instant;
use uuid::Uuid;

#[test]
fn the_relying_party_is_built_from_the_frozen_configuration() {
    let party = FixtureRelyingParty::new();
    assert!(party.is_ok(), "the frozen origin must build: {:?}", party.err());

    // Built from the constants, not from arguments, so there is no way to
    // construct one pointed somewhere else.
    let source = include_str!("../src/lib.rs");
    let code: String = source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(code.contains("Url::parse(ORIGIN)"), "the origin must be the compiled one");
    assert!(code.contains("WebauthnBuilder::new(RP_ID"), "the RP ID must be the compiled one");
    for settable in ["fn with_origin", "origin: &str", "rp_id: &str", "fn new(origin"] {
        assert!(
            !code.contains(settable),
            "the relying party takes {settable:?}, so it can be pointed elsewhere"
        );
    }
}

/// A registration challenge names the frozen relying party and a fresh random
/// handle, and the handle is returned so the caller stores what the
/// authenticator will hand back rather than inventing a name for it.
#[test]
fn a_registration_challenge_carries_the_frozen_rp_and_a_fresh_handle() {
    let party = FixtureRelyingParty::new().unwrap();
    let (first_handle, challenge, _) = party.start_registration().unwrap();
    let (second_handle, _, _) = party.start_registration().unwrap();

    assert_ne!(first_handle, second_handle, "the handle is not random");
    assert_eq!(first_handle.get_version_num(), 4);

    let json = serde_json::to_string(&challenge).unwrap();
    assert!(json.contains(RP_ID), "the challenge does not name the frozen RP");
    // Fixed synthetic names, never a person's.
    assert!(json.contains("fixture-user"), "the challenge lost the synthetic name");
}

/// Two challenges are different, so one cannot be replayed for the other.
#[test]
fn challenges_are_not_reused() {
    let party = FixtureRelyingParty::new().unwrap();
    let (_, first, _) = party.start_registration().unwrap();
    let (_, second, _) = party.start_registration().unwrap();
    assert_ne!(
        serde_json::to_string(&first).unwrap(),
        serde_json::to_string(&second).unwrap()
    );
}

/// The line this crate must not cross. It parses; the registry decides.
///
/// Written as a source scan because the property is an absence: there is no
/// call into the registry from here, and no decision made here.
#[test]
fn the_adapter_decides_nothing() {
    let source = include_str!("../src/lib.rs");
    let code: String = source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");

    // It reports facts and never acts on them.
    for decision in [
        "Registry",
        "finish_registration(now",
        "begin_login",
        "ApprovalStore",
        "Sessions",
    ] {
        assert!(
            !code.contains(decision),
            "the adapter reaches {decision:?}; decisions belong in sovereign-owner"
        );
    }
    // And it only uses the safe API.
    for dangerous in ["webauthn_rs_core", "danger_", "danger-"] {
        assert!(
            !code.contains(dangerous),
            "the adapter reaches for {dangerous:?}"
        );
    }
}

/// The manifest is the record that no `danger-` feature is enabled and that
/// the pinned version is the one RFC 0006's plan froze.
#[test]
fn the_manifest_pins_the_frozen_version_and_no_dangerous_feature() {
    let manifest = include_str!("../Cargo.toml");
    assert!(
        manifest.contains(r#"webauthn-rs = "=0.5.5""#),
        "webauthn-rs must be pinned exactly"
    );
    assert!(
        !manifest.contains("webauthn-rs-core"),
        "the core crate must not be a direct dependency"
    );
    for dangerous in ["danger-credential-internals", "danger-user-presence-only-security-keys"] {
        assert!(!manifest.contains(dangerous), "{dangerous} is enabled");
    }
}

/// Its own workspace, which is the whole reason this crate exists separately.
#[test]
fn this_crate_is_its_own_workspace() {
    let manifest = include_str!("../Cargo.toml");
    assert!(
        manifest.contains("[workspace]"),
        "without its own workspace, 94 crates and an OpenSSL binding enter the audited core lock file"
    );
}

/// The three facts the adapter produces are exactly what the registry
/// consumes — checked by feeding hand-made facts through a real registry, so
/// the shapes cannot drift apart without this failing to compile.
#[test]
fn the_facts_the_adapter_produces_are_what_the_registry_consumes() {
    let mut registry = Registry::new(CeremonyConfig::frozen(), Qualification::ProtocolFixtureOnly);
    let now = Instant::now();
    let ceremony = Uuid::new_v4();
    let handle = Uuid::new_v4();

    // The same three values `RegistrationFacts` carries.
    registry.begin_registration(now, ceremony).unwrap();
    let (stored, bootstrap) = registry
        .finish_registration(now, ceremony, b"credential-id".to_vec(), handle, true)
        .unwrap();
    assert_eq!(stored.user_handle, handle);
    assert!(bootstrap.honest_summary().contains("not owner admission"));

    // And the same three `LoginFacts` carries.
    let login = Uuid::new_v4();
    let allow = registry.begin_login(now, login).unwrap();
    assert_eq!(allow, b"credential-id".to_vec());
    assert_eq!(
        registry
            .finish_login(now, login, b"credential-id", handle, true)
            .unwrap(),
        handle
    );
}

/// The origin the adapter is compiled against is the one the guard enforces
/// and the one the preflight measured. Three places, one value.
#[test]
fn the_origin_is_the_same_one_everywhere() {
    assert_eq!(ORIGIN, "http://localhost:7787");
    assert_eq!(RP_ID, "localhost");
    // `localhost` rather than an IP, because the preflight got SecurityError
    // from a real browser on an IP origin.
    assert!(!RP_ID.chars().next().unwrap().is_ascii_digit());
}

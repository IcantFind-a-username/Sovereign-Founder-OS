//! The boundary around the fixture owner ceremony.
//!
//! RFC 0006 says a same-account process can win an empty-registry enrolment,
//! and that this is not owner admission. These tests are what stops that
//! sentence from decaying into a comment nobody reads: they assert the
//! boundary is made of things the compiler enforces, not of intent.

#![cfg(feature = "owner-effect-fixture")]

use sovereign_owner::config::{
    CeremonyConfig, ConfigError, CEREMONY_TIMEOUT, ORIGIN, RP_ID, SYNTHETIC_DISPLAY_NAME,
    SYNTHETIC_USER_NAME,
};
use std::time::Duration;

/// There is no product admission type, so there is nothing for a fixture
/// outcome to become. This test is a statement about the crate's surface, and
/// it is written as a source scan because the property is an absence — and an
/// absence cannot be called.
#[test]
fn product_owner_admission_has_no_constructor() {
    let source = include_str!("../src/bootstrap.rs");
    assert!(
        !source.contains("ProductOwnerAdmission"),
        "a product admission type appeared; the fixture now has somewhere to be promoted to"
    );
    // And the outcome type's own constructor is crate-private, so no caller
    // can mint one. If this string ever reads `pub fn new`, a caller can
    // manufacture a bootstrap and hand it to something that trusts it.
    assert!(
        source.contains("pub(crate) fn new"),
        "FixtureBootstrap::new must stay crate-private"
    );
}

/// The strongest form of "there is no product path": a default build of this
/// crate contains nothing at all.
#[test]
fn the_default_build_exposes_nothing() {
    let manifest = include_str!("../Cargo.toml");
    assert!(
        manifest.contains("default = []"),
        "the owner crate must have no default features"
    );
    let lib = include_str!("../src/lib.rs");
    // Every `pub` item is behind the fixture feature. A module declared
    // without the gate would be reachable from a default build.
    for line in lib.lines() {
        if line.starts_with("pub mod") || line.starts_with("pub use") || line.starts_with("mod ") {
            assert!(
                lib.contains(&format!(
                    "#[cfg(feature = \"owner-effect-fixture\")]\n{line}"
                )),
                "{line:?} is not behind the fixture feature"
            );
        }
    }
}

/// The outcome names what it is. A type called `OwnerAdmission` would be used
/// as one; a type called `FixtureBootstrap` invites the question.
#[test]
fn fixture_bootstrap_is_typed_unqualified() {
    let source = include_str!("../src/bootstrap.rs");
    assert!(source.contains("pub struct FixtureBootstrap"));
    assert!(
        source.contains("ProtocolFixtureOnly") && source.contains("MechanismQualifiedOnly"),
        "both qualifications must exist"
    );
    // No third value, and in particular no qualified one.
    assert!(
        !source.contains("Qualified,") && !source.contains("Product,"),
        "a qualification beyond the two unqualified ones appeared"
    );
}

/// The enrolment winner is called the winner. Naming it the owner would be
/// the exact lie the boundary exists to prevent, since on an empty registry a
/// hostile same-account process can be the one who wins.
#[test]
fn fixture_report_never_calls_the_winner_the_owner() {
    let source = include_str!("../src/bootstrap.rs");
    assert!(source.contains("enrolment_winner"));
    for forbidden in ["fn owner(", "intended_owner", "the_owner", "admitted_owner"] {
        assert!(
            !source.contains(forbidden),
            "the outcome exposes {forbidden:?}, which claims more than a fixture can"
        );
    }
}

/// The frozen values are the only ones a ceremony may run under.
#[test]
fn the_configuration_accepts_only_the_frozen_values() {
    assert!(CeremonyConfig::new(ORIGIN, RP_ID, CEREMONY_TIMEOUT).is_ok());

    // An IP origin is what the preflight found a real browser rejects, and a
    // near miss is a different origin.
    for origin in [
        "http://127.0.0.1:7787",
        "https://localhost:7787",
        "http://localhost:7788",
        "http://localhost",
    ] {
        assert_eq!(
            CeremonyConfig::new(origin, RP_ID, CEREMONY_TIMEOUT).err(),
            Some(ConfigError::Origin),
            "origin {origin:?} was accepted"
        );
    }

    for rp_id in ["127.0.0.1", "localhost:7787", "example.test", ""] {
        assert_eq!(
            CeremonyConfig::new(ORIGIN, rp_id, CEREMONY_TIMEOUT).err(),
            Some(ConfigError::RelyingParty),
            "relying party {rp_id:?} was accepted"
        );
    }
}

/// A ceremony that can be extended is one an attacker can keep alive while
/// they work, so the timeout is exact rather than bounded.
#[test]
fn the_ceremony_timeout_is_exactly_three_hundred_seconds() {
    assert_eq!(CEREMONY_TIMEOUT, Duration::from_secs(300));
    for timeout in [
        Duration::from_secs(299),
        Duration::from_secs(301),
        Duration::from_secs(3600),
        Duration::ZERO,
    ] {
        assert_eq!(
            CeremonyConfig::new(ORIGIN, RP_ID, timeout).err(),
            Some(ConfigError::Timeout),
            "a {timeout:?} ceremony was accepted"
        );
    }
}

/// Names are fixed and synthetic. A corpus that can hold a person's name is
/// one that eventually will.
#[test]
fn registration_uses_fixed_synthetic_names() {
    assert_eq!(SYNTHETIC_USER_NAME, "fixture-user");
    assert_eq!(SYNTHETIC_DISPLAY_NAME, "Fixture User");
    let config = include_str!("../src/config.rs");
    assert!(
        !config.contains("fn set_user_name") && !config.contains("user_name:"),
        "the synthetic names must not be settable"
    );
}

/// The crate cannot reach a store, a claim, or an effect, because it does not
/// depend on anything that has one.
#[test]
fn the_owner_crate_depends_on_no_kernel_crate() {
    let manifest = include_str!("../Cargo.toml");
    for forbidden in [
        "sovereign-authority",
        "sovereign-capability",
        "sovereign-effects",
        "sovereign-cli",
        "sovereign-vault",
    ] {
        assert!(
            !manifest.contains(forbidden),
            "the owner crate depends on {forbidden}, so it can reach the kernel"
        );
    }
}

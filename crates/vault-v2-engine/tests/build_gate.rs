//! The build script's ambient-override check, exercised directly.
//!
//! `build.rs` and this test `include!` the same source, so what is asserted
//! here is what actually runs at build time.

include!("../build_gate.rs");

fn env(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
        .collect()
}

#[test]
fn every_listed_dependency_shaping_variable_is_rejected() {
    for name in DEPENDENCY_SHAPING_VARIABLES {
        let rejected = rejected_variables(&env(&[(name, "/opt/attacker")]));
        assert_eq!(
            rejected,
            vec![name.to_string()],
            "{name} was set but did not stop the build"
        );
    }
}

#[test]
fn the_pkg_config_family_is_rejected_wholesale() {
    let rejected = rejected_variables(&env(&[
        ("PKG_CONFIG_PATH", "/opt/attacker/lib/pkgconfig"),
        ("PKG_CONFIG_SYSROOT_DIR", "/opt/attacker"),
    ]));
    assert_eq!(rejected.len(), 2, "rejected: {rejected:?}");
}

#[test]
fn a_target_prefixed_override_is_rejected_like_its_bare_form() {
    // openssl-sys honours <TARGET>_OPENSSL_DIR as readily as OPENSSL_DIR, so
    // matching only the bare name would leave the door open.
    let rejected = rejected_variables(&env(&[
        ("X86_64_UNKNOWN_LINUX_GNU_OPENSSL_DIR", "/opt/attacker"),
        ("AARCH64_APPLE_DARWIN_OPENSSL_LIB_DIR", "/opt/attacker/lib"),
    ]));
    assert_eq!(rejected.len(), 2, "rejected: {rejected:?}");
}

#[test]
fn an_unrelated_variable_is_left_alone() {
    let rejected = rejected_variables(&env(&[
        ("PATH", "/usr/bin"),
        ("HOME", "/home/owner"),
        ("CARGO_PKG_NAME", "sovereign-vault-v2-engine"),
        // Not a suffix match: the shaping name must be a whole `_`-delimited
        // tail, or every variable ending in "PERL" would trip the gate.
        ("MYPERL", "/usr/bin/perl"),
    ]));
    assert!(rejected.is_empty(), "false positives: {rejected:?}");
}

#[test]
fn an_empty_value_is_not_treated_as_an_override() {
    // Cargo always hands build scripts CARGO_ENCODED_RUSTFLAGS, empty when no
    // flags are set. Rejecting its presence would fail every ordinary build.
    let rejected = rejected_variables(&env(&[
        ("CARGO_ENCODED_RUSTFLAGS", ""),
        ("RUSTFLAGS", "   "),
    ]));
    assert!(rejected.is_empty(), "rejected: {rejected:?}");
}

#[test]
fn the_allowlist_is_closed_and_recorded() {
    // Shrinking this list is a reviewed decision, not a refactor: each entry
    // is a documented way to reshape the SQLCipher/OpenSSL build.
    assert_eq!(
        DEPENDENCY_SHAPING_VARIABLES.len(),
        23,
        "the closed allowlist changed — that needs RFC review, not a test edit"
    );
    let mut sorted = DEPENDENCY_SHAPING_VARIABLES.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(
        sorted.len(),
        DEPENDENCY_SHAPING_VARIABLES.len(),
        "duplicate entries in the allowlist"
    );
}

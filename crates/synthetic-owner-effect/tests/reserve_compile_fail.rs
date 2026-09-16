//! Compile-fail and source contracts for `AuthorityReservedEffect`.
//!
//! Design Accept ≠ product Current. Not a reconstruction path.

#![cfg(feature = "owner-effect-fixture")]

use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

use sovereign_synthetic_owner_effect::AuthorityReservedEffect;

const FIXTURES: &[(&str, &str, &[&str])] = &[
    (
        "tests/ui/cannot_construct_authority_reserved.rs",
        "trybuild000",
        &["AuthorityReservedEffect", "private"],
    ),
    (
        "tests/ui/cannot_clone_debug_or_serialize_authority_reserved.rs",
        "trybuild001",
        &["AuthorityReservedEffect", "Clone", "Debug", "Serialize"],
    ),
    (
        "tests/ui/cannot_destructure_authority_reserved.rs",
        "trybuild002",
        &["AuthorityReservedEffect", "private"],
    ),
    (
        "tests/ui/cannot_reconstruct_authority_reserved.rs",
        "trybuild003",
        &["from_intent_id"],
    ),
];

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn harness_dir() -> PathBuf {
    manifest_dir().join("tests/ui/harness")
}

fn workspace_lock() -> PathBuf {
    manifest_dir().join("../../Cargo.lock")
}

fn package_versions(text: &str) -> Vec<(String, String)> {
    let mut versions = Vec::new();
    let mut name = None;
    for line in text.lines() {
        if let Some(value) = line.strip_prefix("name = \"") {
            name = Some(value.trim_end_matches('"').to_owned());
        } else if let Some(value) = line.strip_prefix("version = \"") {
            if let Some(package) = name.take() {
                versions.push((package, value.trim_end_matches('"').to_owned()));
            }
        }
    }
    versions
}

fn cargo_check(bin: &str) -> Output {
    let mut command = Command::new(
        std::env::var_os("CARGO").unwrap_or_else(|| std::ffi::OsString::from("cargo")),
    );
    command
        .current_dir(harness_dir())
        .env("CARGO_INCREMENTAL", "0")
        .env_remove("RUSTFLAGS")
        .args([
            "check",
            "--bin",
            bin,
            "--offline",
            "--locked",
            "--color=never",
            "--config=build.rustflags=[\"--cfg\",\"trybuild\",\"--verbose\",\"-A\",\"dead_code\"]",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command.output().expect("run cargo check for UI fixture")
}

#[test]
fn compile_fail_construct_clone_serialize_debug_destructure_reconstruct() {
    let source = include_str!("../src/reserved.rs");
    assert!(source.contains("pub struct AuthorityReservedEffect {"));
    assert!(source.contains("intent_id: EffectIntentId,"));
    assert!(!source.contains("pub intent_id"));
    assert!(!source.contains("pub fn new"));
    assert!(!source.contains("pub fn from_intent_id"));
    assert!(!source.contains("pub fn from_parts"));
    assert!(source.contains("assert_not_impl_any!"));
    assert!(
        !source.contains("pub fn "),
        "AuthorityReservedEffect must not grow a public constructor or accessor"
    );

    let lock_path = harness_dir().join("Cargo.lock");
    assert!(
        lock_path.exists(),
        "nested UI harness Cargo.lock is required for --offline --locked cargo check"
    );
    let workspace = std::fs::read_to_string(workspace_lock()).expect("workspace Cargo.lock");
    let harness = std::fs::read_to_string(&lock_path).expect("harness Cargo.lock");
    let workspace_versions = package_versions(&workspace);
    for (name, version) in package_versions(&harness) {
        if name == "sovereign-synthetic-owner-effect-ui-harness" {
            continue;
        }
        assert!(
            workspace_versions
                .iter()
                .any(|(workspace_name, workspace_version)| {
                    workspace_name == &name && workspace_version == &version
                }),
            "harness lock drifted from the workspace graph: {name} {version}"
        );
    }

    for (fixture, bin, needles) in FIXTURES {
        let output = cargo_check(bin);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            !output.status.success(),
            "fixture {fixture} was expected not to compile; stdout:\n{stdout}\nstderr:\n{stderr}"
        );
        assert!(
            !stderr.contains("failed to select a version")
                && !stderr.contains("failed to download")
                && !stderr.contains("unresolved import")
                && !stderr.contains("cannot update the lock file"),
            "fixture {fixture} failed before reaching a privacy/trait-bound diagnostic; stderr:\n{stderr}"
        );
        for needle in *needles {
            assert!(
                stderr.contains(needle),
                "fixture {fixture} stderr missing `{needle}`:\n{stderr}"
            );
        }
    }

    fn require_not_reconstructable(_: &AuthorityReservedEffect) {}
    let _ = require_not_reconstructable;
}

#[test]
fn coordinator_source_has_no_raw_id_public_reservation_request() {
    let lib = include_str!("../src/lib.rs");
    assert!(!lib.contains("ReservationRequest"));
    assert!(lib.contains("reserve_exact_authority"));
    assert!(lib.contains("AuthorityReservedEffect"));
}

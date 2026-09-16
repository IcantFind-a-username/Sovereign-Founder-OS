//! Compile-fail harness for opaque Capability V2 proof types.
//!
//! `trybuild` is listed in `Cargo.toml` to pin 1.0.116, but this harness runs
//! `cargo check` without `--quiet`. trybuild 1.0.116 + Cargo 1.97 otherwise
//! treats every compile-fail case as a false success.

use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

const FIXTURES: &[(&str, &str, &[&str])] = &[
    (
        "tests/ui/cannot_construct_verified_capability.rs",
        "trybuild000",
        &["VerifiedCapabilityV2", "field `claims`", "private"],
    ),
    (
        "tests/ui/cannot_clone_debug_or_serialize_verified_capability.rs",
        "trybuild001",
        &["VerifiedCapabilityV2", "Clone", "Debug", "Serialize"],
    ),
    (
        "tests/ui/cannot_construct_verified_approval.rs",
        "trybuild002",
        &["VerifiedApprovalV1", "field `claims`", "private"],
    ),
    (
        "tests/ui/cannot_clone_debug_or_serialize_verified_approval.rs",
        "trybuild003",
        &["VerifiedApprovalV1", "Clone", "Debug", "Serialize"],
    ),
];

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn harness_dir() -> PathBuf {
    manifest_dir().join("tests/ui/harness")
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
fn proof_types_reject_forging_cloning_debug_and_serialize() {
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
                && !stderr.contains("unresolved import"),
            "fixture {fixture} failed before reaching a privacy/trait-bound diagnostic; stderr:\n{stderr}"
        );
        for needle in *needles {
            assert!(
                stderr.contains(needle),
                "fixture {fixture} stderr missing `{needle}`:\n{stderr}"
            );
        }
    }
}

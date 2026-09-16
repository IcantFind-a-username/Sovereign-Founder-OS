//! Compile-fail harness for the closed privacy broker API.
//!
//! `trybuild` is listed in `Cargo.toml` to pin 1.0.116, but this harness runs
//! `cargo check` without `--quiet`. trybuild 1.0.116 + Cargo 1.97 otherwise
//! treats every compile-fail case as a false success.

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

const FIXTURES: &[(&str, &str, &[&str])] = &[
    (
        "tests/ui/cannot_borrow_safe_request_or_transition_attempts.rs",
        "trybuild000",
        &["outbound_text", "Attempt", "private"],
    ),
    (
        "tests/ui/cannot_forge_route_evidence_from_caller_strings.rs",
        "trybuild001",
        &["RouteEvidence", "From", "ClosedProviderId"],
    ),
];

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn harness_dir() -> PathBuf {
    manifest_dir().join("tests/ui/harness")
}

fn cargo_check(bin: &str) -> Output {
    let mut command =
        Command::new(std::env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo")));
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
fn callers_cannot_borrow_safe_request_or_forge_route_evidence() {
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

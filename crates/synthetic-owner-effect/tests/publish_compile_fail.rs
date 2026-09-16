//! Compile-fail: reuse after move, and concurrent cloned handles.

#![cfg(feature = "owner-effect-fixture")]

use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

const FIXTURES: &[(&str, &str, &[&str])] = &[
    (
        "tests/ui/cannot_reuse_reserved_handle_after_move.rs",
        "trybuild004",
        &["moved", "AuthorityReservedEffect"],
    ),
    (
        "tests/ui/cannot_manufacture_concurrent_cloned_handles.rs",
        "trybuild005",
        &["Clone", "AuthorityReservedEffect"],
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
fn compile_fail_reuse_after_move_and_concurrent_cloned_handles() {
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
            "fixture {fixture} failed before reaching a move/clone diagnostic; stderr:\n{stderr}"
        );
        for needle in *needles {
            assert!(
                stderr.contains(needle),
                "fixture {fixture} stderr missing `{needle}`:\n{stderr}"
            );
        }
    }
}

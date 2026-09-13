//! Task 1 public-boundary compile-fail harness (Program 1A plan lines 579-586).
//!
//! Lists exactly five fixtures — no glob and no sixth case in this group.
//! Goldens live beside each fixture as `.stderr` files; `trybuild` is listed in
//! `Cargo.toml` per the plan and drives the reviewed dependency pin, while this
//! harness runs `cargo check` without `--quiet` (trybuild 1.0.116 + Cargo 1.97
//! otherwise treats every compile-fail case as a false success).

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const FIXTURES: &[(&str, &str)] = &[
    ("tests/ui/cannot_name_db_key.rs", "trybuild000"),
    ("tests/ui/cannot_call_raw_key_shim.rs", "trybuild001"),
    ("tests/ui/cannot_reach_raw_handle.rs", "trybuild002"),
    ("tests/ui/cannot_construct_create_mode.rs", "trybuild003"),
    ("tests/ui/cannot_select_cipher_profile.rs", "trybuild004"),
];

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn harness_dir() -> PathBuf {
    manifest_dir().join("tests/ui/harness")
}

fn rustc_diagnostic(stderr: &str) -> &str {
    let start = stderr
        .find("error[E")
        .or_else(|| stderr.find("error: "))
        .unwrap_or(0);
    let rest = &stderr[start..];
    let end = rest
        .find("For more information")
        .unwrap_or_else(|| rest.find("error: could not compile").unwrap_or(rest.len()));
    &rest[..end]
}

fn normalize_stderr(stderr: &str, manifest: &Path) -> String {
    let manifest = manifest.to_string_lossy().into_owned();
    let mut normalized = rustc_diagnostic(stderr).replace(&manifest, "$DIR");
    for (fixture, _) in FIXTURES {
        let file = fixture.rsplit('/').next().expect("fixture file name");
        normalized = normalized.replace(&format!("../{file}"), &format!("$DIR/{fixture}"));
    }
    normalized.replace("\r\n", "\n").trim().to_string()
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
            "--color=never",
            "--config=build.rustflags=[\"--cfg\",\"trybuild\",\"--verbose\",\"-A\",\"dead_code\"]",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command.output().expect("run cargo check for UI fixture")
}

#[test]
fn the_public_boundary_rejects_forbidden_names_and_handles() {
    let manifest = manifest_dir();
    for (fixture, bin) in FIXTURES {
        let output = cargo_check(bin);
        assert!(
            !output.status.success(),
            "fixture {fixture} was expected not to compile; stdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        let golden_path = manifest.join(fixture).with_extension("stderr");
        let expected = std::fs::read_to_string(&golden_path)
            .unwrap_or_else(|error| panic!("read {golden_path:?}: {error}"));
        let expected = expected.replace("\r\n", "\n");

        let actual = normalize_stderr(&String::from_utf8_lossy(&output.stderr), &manifest);
        assert_eq!(
            actual.trim(),
            expected.trim(),
            "stderr mismatch for {fixture}"
        );
    }
}

//! Task 2 recovery compile-fail harness (independent from Task 1's five UI cases).

use std::path::PathBuf;
use std::process::{Command, Stdio};

const FIXTURE: &str = "tests/recovery_ui/recovery_read_only.rs";

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

fn normalize_stderr(stderr: &str, manifest: &std::path::Path) -> String {
    let manifest = manifest.to_string_lossy().into_owned();
    let mut normalized = rustc_diagnostic(stderr).replace(&manifest, "$DIR");
    normalized = normalized.replace("../../recovery_ui/recovery_read_only.rs", &format!("$DIR/{FIXTURE}"));
    normalized = normalized.replace(
        &format!("../{}", FIXTURE.rsplit('/').next().expect("name")),
        &format!("$DIR/{FIXTURE}"),
    );
    normalized.replace("\r\n", "\n").trim().to_string()
}

fn cargo_check(bin: &str) -> std::process::Output {
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
    command.output().expect("run cargo check for recovery UI fixture")
}

#[test]
fn recovery_read_only() {
    let manifest = manifest_dir();
    let output = cargo_check("recovery_read_only");
    assert!(
        !output.status.success(),
        "fixture was expected not to compile; stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let golden_path = manifest.join(FIXTURE).with_extension("stderr");
    let expected = std::fs::read_to_string(&golden_path)
        .unwrap_or_else(|error| panic!("read {golden_path:?}: {error}"));
    let expected = expected.replace("\r\n", "\n");

    let actual = normalize_stderr(&String::from_utf8_lossy(&output.stderr), &manifest);
    assert_eq!(
        actual.trim(),
        expected.trim(),
        "stderr mismatch for {FIXTURE}"
    );
}

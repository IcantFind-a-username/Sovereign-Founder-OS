//! `--root` keeps workspace state off the default per-user data directory.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn default_product_root(home: &std::path::Path) -> PathBuf {
    home.join(".local/share/sovereign-founder-os")
}

#[test]
fn init_with_root_writes_only_under_explicit_path() {
    let sandbox = tempfile::tempdir().unwrap();
    let home = sandbox.path().join("home");
    let explicit = sandbox.path().join("explicit-root");
    fs::create_dir_all(&home).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_sovereign"))
        .args(["--root", explicit.to_str().unwrap(), "init"])
        .env("HOME", &home)
        .env("XDG_DATA_HOME", home.join(".local/share"))
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(explicit.join("device.json").is_file());
    assert!(explicit.join("ledger.json").is_file());
    assert!(
        explicit.join("vault").is_dir(),
        "vault directory should exist under --root"
    );
    assert!(
        !default_product_root(&home).exists(),
        "default data dir must stay untouched when --root is set"
    );
}

#[test]
fn init_with_file_root_fails_closed() {
    let sandbox = tempfile::tempdir().unwrap();
    let file = sandbox.path().join("not-a-dir");
    fs::write(&file, b"x").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_sovereign"))
        .args(["--root", file.to_str().unwrap(), "init"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(combined.contains("--root must be a directory"));
}

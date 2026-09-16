//! `demo` must not seed the owner's Workspace root.
//!
//! Default `demo` / `demo --fast` used to hand `data_dir()` to the story
//! runner, which wrote `ledger.json`, `vault/`, and `artifacts/admissions/`
//! into the same tree the Workspace and Security Center read.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const LEDGER_CANARY: &[u8] = b"{\"canary\":\"owner-ledger-p2-demo-isolate\"}\n";
const VAULT_KEY_CANARY: &[u8] = b"owner-vault-key-p2-demo-isolate";
const VAULT_ENTRY_CANARY: &[u8] = b"owner-vault-entry-p2-demo-isolate";
const ADMISSION_CANARY: &[u8] = b"owner-admission-p2-demo-isolate";
const ADMISSION_OBJECT_CANARY: &[u8] = b"owner-admission-object-p2-demo-isolate";

fn product_root(home: &Path, xdg: &Path) -> PathBuf {
    // `dirs::data_local_dir()` on macOS is `~/Library/Application Support`
    // and ignores `XDG_DATA_HOME`; on Linux it follows `XDG_DATA_HOME`.
    #[cfg(target_os = "macos")]
    {
        let _ = xdg;
        home.join("Library/Application Support/sovereign-founder-os")
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = home;
        xdg.join("sovereign-founder-os")
    }
}

fn snapshot_files(path: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut out = BTreeMap::new();
    if path.is_file() {
        out.insert(
            PathBuf::from(path.file_name().unwrap()),
            fs::read(path).unwrap(),
        );
        return out;
    }
    if !path.exists() {
        return out;
    }
    fn walk(base: &Path, dir: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(base, &path, out);
            } else if path.is_file() {
                out.insert(
                    path.strip_prefix(base).unwrap().to_path_buf(),
                    fs::read(&path).unwrap(),
                );
            }
        }
    }
    walk(path, path, &mut out);
    out
}

fn seed_product_root(root: &Path) {
    fs::create_dir_all(root.join("vault")).unwrap();
    fs::create_dir_all(root.join("artifacts/admissions/objects")).unwrap();
    fs::write(root.join("ledger.json"), LEDGER_CANARY).unwrap();
    fs::write(root.join("vault/vault.key"), VAULT_KEY_CANARY).unwrap();
    fs::write(root.join("vault/venture_profile.enc"), VAULT_ENTRY_CANARY).unwrap();
    fs::write(
        root.join("artifacts/admissions/record.json"),
        ADMISSION_CANARY,
    )
    .unwrap();
    fs::write(
        root.join("artifacts/admissions/objects/blob"),
        ADMISSION_OBJECT_CANARY,
    )
    .unwrap();
}

#[test]
fn demo_fast_leaves_seeded_product_root_byte_unchanged() {
    let sandbox = tempfile::tempdir().unwrap();
    let home = sandbox.path().join("home");
    let xdg = home.join(".local/share");
    fs::create_dir_all(&xdg).unwrap();

    let root = product_root(&home, &xdg);
    seed_product_root(&root);

    let before_ledger = snapshot_files(&root.join("ledger.json"));
    let before_vault = snapshot_files(&root.join("vault"));
    let before_admissions = snapshot_files(&root.join("artifacts/admissions"));

    let output = Command::new(env!("CARGO_BIN_EXE_sovereign"))
        .args(["demo", "--fast"])
        .env("HOME", &home)
        .env("XDG_DATA_HOME", &xdg)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "demo --fast failed: status={} stdout={} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(
        snapshot_files(&root.join("ledger.json")),
        before_ledger,
        "demo must not rewrite the product ledger.json"
    );
    assert_eq!(
        snapshot_files(&root.join("vault")),
        before_vault,
        "demo must not rewrite the product vault/"
    );
    assert_eq!(
        snapshot_files(&root.join("artifacts/admissions")),
        before_admissions,
        "demo must not rewrite the product artifacts/admissions/"
    );

    // The story still has to persist somewhere — a no-op demo would also
    // leave the seeded root unchanged. Writes belong under the marked
    // isolated subdirectory, not the Workspace root.
    let demo_root = root.join("demo");
    assert!(
        demo_root.join("ledger.json").is_file(),
        "demo should persist its own ledger under the isolated demo root"
    );
    assert!(
        demo_root.join("vault").is_dir(),
        "demo should persist its own vault under the isolated demo root"
    );
    assert!(
        demo_root.join("artifacts/admissions").is_dir(),
        "demo should persist admissions under the isolated demo root"
    );
    assert_ne!(
        fs::read(demo_root.join("ledger.json")).unwrap(),
        LEDGER_CANARY
    );
}

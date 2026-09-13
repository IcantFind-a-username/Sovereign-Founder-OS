//! Program 1A Task 6 — prove the engine stays off the product graph.

use std::path::PathBuf;

fn read_crate_relative(relative: &str) -> String {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(manifest.join(relative))
        .unwrap_or_else(|error| panic!("read {relative}: {error}"))
}

fn read_repo_relative(relative: &str) -> String {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::read_to_string(root.join(relative))
        .unwrap_or_else(|error| panic!("read {relative}: {error}"))
}

#[test]
fn engine_crate_is_not_published() {
    let manifest = read_crate_relative("Cargo.toml");
    assert!(
        manifest.contains("publish = false"),
        "vault-v2-engine must remain publish = false"
    );
}

#[test]
fn product_cli_does_not_depend_on_engine() {
    let cli = read_repo_relative("apps/cli/Cargo.toml");
    assert!(
        !cli.contains("sovereign-vault-v2-engine"),
        "sovereign-cli must not link the private engine during Program 1A"
    );
}

#[test]
fn main_process_has_no_product_dispatcher() {
    let main_rs = read_crate_relative("src/main.rs");
    assert!(
        !main_rs.contains("ActiveV2"),
        "engine main must not reference product activation"
    );
    assert!(
        !main_rs.contains("vault.format"),
        "engine main must not write vault.format"
    );
}

#[test]
fn migration_does_not_activate_product_vault() {
    let migration = read_crate_relative("src/engine/migration.rs");
    assert!(
        !migration.contains("ActiveV2"),
        "migration must not reference product ActiveV2"
    );
    assert!(
        migration.contains("role_keys_block_activation"),
        "activation blocker test must remain in migration"
    );
}

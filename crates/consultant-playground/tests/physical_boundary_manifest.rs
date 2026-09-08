//! Task 1 physical boundary: the crate's `Cargo.toml` must stay unpublished
//! and dependency-free, as reported by `cargo metadata` itself (not just by
//! reading the TOML text, which a dotted-key or table-array bypass could
//! dodge).

#[path = "support/json.rs"]
mod json;
#[path = "support/manifest.rs"]
mod manifest;
#[path = "support/source_root.rs"]
mod source_root;

use manifest::{crate_root, manifest_boundary, ManifestFixture};

#[test]
fn task_one_manifest_is_publish_false_and_dependency_free() {
    manifest_boundary(
        &crate_root().join("Cargo.toml"),
        "sovereign-consultant-playground",
    )
    .expect("Task 1 manifest must be unpublished and dependency-free");
}

#[test]
fn cargo_metadata_rejects_normal_dev_build_target_and_dotted_dependencies() {
    let declarations = [
        "[dependencies]\nforbidden = { path = \"dep\" }",
        "[dev-dependencies]\nforbidden = { path = \"dep\" }",
        "[build-dependencies]\nforbidden = { path = \"dep\" }",
        "[target.'cfg(unix)'.dependencies]\nforbidden = { path = \"dep\" }",
        "[target.'cfg(unix)'.dev-dependencies]\nforbidden = { path = \"dep\" }",
        "[target.'cfg(unix)'.build-dependencies]\nforbidden = { path = \"dep\" }",
        "[dependencies.forbidden]\npath = \"dep\"",
        "[target.'cfg(unix)'.dependencies.forbidden]\npath = \"dep\"",
    ];

    for declaration in declarations {
        let fixture = ManifestFixture::new(declaration);
        let error = manifest_boundary(&fixture.manifest, "boundary-fixture")
            .expect_err("Cargo metadata must expose the dependency declaration");
        assert!(
            error.contains("dependency declarations"),
            "declaration was not parsed as a dependency: {declaration}: {error}"
        );
    }
}

#[test]
fn cargo_metadata_rejects_commented_publish_false_bypass() {
    let fixture = ManifestFixture::with_publish_line("# publish = false", "");
    let error = manifest_boundary(&fixture.manifest, "boundary-fixture")
        .expect_err("Cargo metadata must report the package as publishable");
    assert!(
        error.contains("can be published"),
        "publish rejection must come from parsed metadata: {error}"
    );
}

//! Task 1 physical boundary: `src/lib.rs` and `src/domain.rs` must match a
//! pinned token-exact grammar with no persistence, IO, or product surface,
//! and the source root itself must not be a symlink an attacker could swap
//! out from under the check.

use std::fs;
use std::path::Path;

#[path = "support/boundary.rs"]
mod boundary;
#[path = "support/json.rs"]
mod json;
#[path = "support/manifest.rs"]
mod manifest;
#[path = "support/production_sources.rs"]
mod production_sources;
#[path = "support/rust_lexer.rs"]
mod rust_lexer;
#[path = "support/source_root.rs"]
mod source_root;
#[path = "support/symlink_fixture.rs"]
mod symlink_fixture;

use boundary::{source_boundary, SourceBoundaryKind};
use manifest::{crate_root, manifest_boundary};
use production_sources::production_sources;
use source_root::SourceRootError;
use std::collections::BTreeSet;

#[test]
fn task_one_production_source_closure_has_no_persistence_or_product_surface() {
    let source_root = crate_root().join("src");
    let expected = BTreeSet::from([source_root.join("domain.rs"), source_root.join("lib.rs")]);
    let actual = production_sources(&source_root).expect("source root must be a real directory");
    assert_eq!(actual, expected, "Task 1 source closure changed");

    for path in actual {
        let source = fs::read_to_string(&path).expect("production source must be readable");
        source_boundary(&path, &source)
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    }
}

#[cfg(any(unix, windows))]
#[test]
fn symlinked_source_root_is_rejected_before_traversal_or_manifest_canonicalization() {
    let fixture = match symlink_fixture::SymlinkedSourceFixture::new() {
        Ok(fixture) => fixture,
        #[cfg(windows)]
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            eprintln!("directory symlinks are unavailable without Windows developer privilege");
            return;
        }
        Err(error) => panic!("source-root fixture must be created: {error}"),
    };

    assert_eq!(
        production_sources(&fixture.source_link),
        Err(SourceRootError::RootSymlink),
        "source inventory must reject the symlink root itself"
    );
    let manifest_error = manifest_boundary(&fixture.base.manifest, "boundary-fixture")
        .expect_err("manifest boundary must reject its symlinked source root");
    assert!(manifest_error.contains("RootSymlink"), "{manifest_error}");
}

#[test]
fn syntax_checker_rejects_grouped_std_alias_and_path_attribute_escapes() {
    let mutations = [
        (
            "grouped std import",
            Path::new("domain.rs"),
            domain_with_extra("use std::{fs};"),
            SourceBoundaryKind::DomainProductionShape,
        ),
        (
            "grouped std alias",
            Path::new("domain.rs"),
            domain_with_extra("use std::{fs as disk};"),
            SourceBoundaryKind::DomainProductionShape,
        ),
        (
            "path attribute",
            Path::new("lib.rs"),
            "#[path=\"../outside.rs\"] mod domain;".into(),
            SourceBoundaryKind::PathAttribute,
        ),
    ];
    for (name, path, source, expected) in mutations {
        assert_source_rejection(name, path, &source, expected);
    }
}

#[test]
fn exact_shape_mutations_are_rejected() {
    let mutations = [
        (
            "cfg-gated domain",
            Path::new("lib.rs"),
            "#[cfg(test)] mod domain;".to_string(),
            SourceBoundaryKind::LibCfgAttrShape,
        ),
        (
            "removed cfg_attr",
            Path::new("lib.rs"),
            "mod domain;".into(),
            SourceBoundaryKind::LibCfgAttrShape,
        ),
        (
            "changed cfg_attr",
            Path::new("lib.rs"),
            "#[cfg_attr(test, allow(dead_code))] mod domain;".into(),
            SourceBoundaryKind::LibCfgAttrShape,
        ),
        (
            "path attribute",
            Path::new("lib.rs"),
            "#[path=\"../outside.rs\"] mod domain;".into(),
            SourceBoundaryKind::PathAttribute,
        ),
        (
            "unknown production attribute",
            Path::new("domain.rs"),
            domain_with_extra("#[allow(unused)] const EXTRA: u8 = 1;"),
            SourceBoundaryKind::DomainProductionShape,
        ),
        (
            "doc-comment attribute",
            Path::new("domain.rs"),
            domain_with_extra("/// hidden attribute"),
            SourceBoundaryKind::Lexical,
        ),
        (
            "env macro",
            Path::new("domain.rs"),
            domain_with_extra("const LEAK: &str = env!(\"SECRET\");"),
            SourceBoundaryKind::DomainProductionShape,
        ),
        (
            "format macro",
            Path::new("domain.rs"),
            domain_with_extra("fn leak() { let _ = format!(\"secret\"); }"),
            SourceBoundaryKind::DomainProductionShape,
        ),
        (
            "owned allocation",
            Path::new("domain.rs"),
            domain_with_extra("fn leak() { let _ = \"secret\".to_owned(); }"),
            SourceBoundaryKind::DomainProductionShape,
        ),
        (
            "unsafe extern",
            Path::new("domain.rs"),
            domain_with_extra(
                "unsafe extern \"C\" { fn open(path: *const u8, flags: i32) -> i32; }",
            ),
            SourceBoundaryKind::DomainProductionShape,
        ),
        (
            "inline assembly",
            Path::new("domain.rs"),
            domain_with_extra("fn leak() { unsafe { core::arch::asm!(\"nop\"); } }"),
            SourceBoundaryKind::DomainProductionShape,
        ),
    ];

    for (name, path, source, expected) in mutations {
        assert_source_rejection(name, path, &source, expected);
    }
}

#[test]
fn domain_test_module_wrapper_must_be_exact_and_terminal() {
    let source = fs::read_to_string(crate_root().join("src/domain.rs"))
        .expect("domain source must be readable");
    let changed_cfg = source.replacen("#[cfg(test)]\nmod tests", "#[cfg(any())]\nmod tests", 1);
    let removed = source
        .split_once("#[cfg(test)]\nmod tests")
        .expect("domain must contain test module")
        .0
        .to_string();

    assert_source_rejection(
        "changed test cfg",
        Path::new("domain.rs"),
        &changed_cfg,
        SourceBoundaryKind::DomainTestModuleShape,
    );
    assert_source_rejection(
        "removed test module",
        Path::new("domain.rs"),
        &removed,
        SourceBoundaryKind::DomainTestModuleShape,
    );
}

fn domain_with_extra(extra: &str) -> String {
    let source = fs::read_to_string(crate_root().join("src/domain.rs"))
        .expect("domain source must be readable");
    let marker = "#[cfg(test)]\nmod tests";
    let offset = source
        .find(marker)
        .expect("domain source must contain its unit-test module");
    format!("{}\n{extra}\n{}", &source[..offset], &source[offset..])
}

fn assert_source_rejection(name: &str, path: &Path, source: &str, expected: SourceBoundaryKind) {
    let error = match source_boundary(path, source) {
        Ok(()) => panic!("source mutation `{name}` was accepted"),
        Err(error) => error,
    };
    assert_eq!(
        error.kind, expected,
        "source mutation `{name}` hit the wrong guard: {error}"
    );
}

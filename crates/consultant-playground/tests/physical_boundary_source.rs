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

use boundary::{
    source_boundary, SourceBoundaryKind, ACTION_DOMAIN_ADDITIONS, EXPECTED_DOMAIN_PRODUCTION,
};
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

#[test]
fn action_gate_accepts_pinned_legacy_and_action_grammars() {
    let legacy = format!(
        "{}\n#[cfg(test)]\nmod tests {{}}",
        EXPECTED_DOMAIN_PRODUCTION
    );
    let action = format!(
        "{}\n{}\n#[cfg(test)]\nmod tests {{}}",
        EXPECTED_DOMAIN_PRODUCTION, ACTION_DOMAIN_ADDITIONS
    );

    assert!(source_boundary(Path::new("domain.rs"), &legacy).is_ok());
    assert!(source_boundary(Path::new("domain.rs"), &action).is_ok());
}

#[test]
fn action_gate_rejects_unapproved_action_mutations() {
    let mutations = [
        (
            "fifth action",
            "    CorrectOfferPrice,\n",
            "    CorrectOfferPrice,\n    Archive,\n",
        ),
        (
            "unit becomes tuple",
            "    CorrectOfferPrice,\n",
            "    CorrectOfferPrice(u32),\n",
        ),
        (
            "unit becomes string",
            "    CorrectOfferPrice,\n",
            "    CorrectOfferPrice(String),\n",
        ),
        (
            "unit becomes price",
            "    CorrectOfferPrice,\n",
            "    CorrectOfferPrice { cents: u32 },\n",
        ),
        (
            "wrong amount",
            "self.graph.offer.price_usd_cents = 350_000;",
            "self.graph.offer.price_usd_cents = 350_001;",
        ),
        (
            "wrong stage",
            "self.graph.relationship.stage = RelationshipStage::Customer;",
            "self.graph.relationship.stage = RelationshipStage::Lead;",
        ),
        (
            "extra field write",
            "self.graph.offer.price_usd_cents = 350_000;",
            "self.graph.offer.name_key = SemanticKey::ReportingClaritySprint;",
        ),
        (
            "search modification",
            "PlaygroundAction::ShowReportingSearch => {}",
            "PlaygroundAction::ShowReportingSearch => { self.graph.offer.price_usd_cents = 350_000; }",
        ),
        (
            "reset keeps old value",
            "*self = Self::new();",
            "*self = Self::new(); self.graph.offer.price_usd_cents = 350_000;",
        ),
        (
            "reset IO",
            "PlaygroundAction::Reset => {\n                *self = Self::new();\n            }",
            "PlaygroundAction::Reset => { let _ = std::fs::read(\"secret\"); *self = Self::new(); }",
        ),
        ("extra method", "#[cfg(test)]\nmod tests", "fn unauthorized(&mut self) {}\n#[cfg(test)]\nmod tests"),
        ("extra module", "#[cfg(test)]\nmod tests", "mod unauthorized {}\n#[cfg(test)]\nmod tests"),
        ("extra import", "#[cfg(test)]\nmod tests", "use std::fmt;\n#[cfg(test)]\nmod tests"),
        ("extra attribute", "#[cfg(test)]\nmod tests", "#[allow(dead_code)]\n#[cfg(test)]\nmod tests"),
        ("cfg hidden", "#[cfg(test)]\nmod tests", "#[cfg(any())] fn hidden() {}\n#[cfg(test)]\nmod tests"),
    ];
    for (name, target, replacement) in mutations {
        let before = action_fixture();
        assert!(
            before.contains(target),
            "mutation target missing for `{name}`"
        );
        let source = before.replacen(target, replacement, 1);
        assert_ne!(source, before, "mutation had no effect for `{name}`");
        assert_source_rejection(
            name,
            Path::new("domain.rs"),
            &source,
            SourceBoundaryKind::DomainProductionShape,
        );
    }
}

#[test]
fn action_gate_preserves_escape_and_test_wrapper_rejections() {
    let grouped = action_fixture_with_extra("use std::{fs};");
    let escapes = [
        ("grouped std", grouped),
        (
            "grouped alias",
            action_fixture_with_extra("use std::{fs as disk};"),
        ),
        (
            "env",
            action_fixture_with_extra("const LEAK: &str = env!(\"SECRET\");"),
        ),
        (
            "format",
            action_fixture_with_extra("fn leak() { let _ = format!(\"secret\"); }"),
        ),
        (
            "allocation",
            action_fixture_with_extra("fn leak() { let _ = \"secret\".to_owned(); }"),
        ),
        (
            "unsafe",
            action_fixture_with_extra("unsafe { core::arch::asm!(\"nop\"); }"),
        ),
        (
            "extern",
            action_fixture_with_extra("unsafe extern \"C\" { fn open(); }"),
        ),
        (
            "path",
            action_fixture_with_extra("#[path=\"../outside.rs\"] mod escaped;"),
        ),
    ];
    for (name, source) in escapes {
        let path = Path::new("domain.rs");
        let kind = if name == "path" {
            SourceBoundaryKind::PathAttribute
        } else {
            SourceBoundaryKind::DomainProductionShape
        };
        assert_source_rejection(name, path, &source, kind);
    }

    let fixture = action_fixture();
    let changed = fixture.replacen("#[cfg(test)]\nmod tests", "#[cfg(any())]\nmod tests", 1);
    let removed = fixture
        .split_once("#[cfg(test)]\nmod tests")
        .expect("fixture wrapper")
        .0
        .to_string();
    let nonterminal = format!("{}\nmod trailing {{}}", fixture);
    for (name, source) in [
        ("changed wrapper", changed),
        ("removed wrapper", removed),
        ("nonterminal wrapper", nonterminal),
    ] {
        assert_source_rejection(
            name,
            Path::new("domain.rs"),
            &source,
            SourceBoundaryKind::DomainTestModuleShape,
        );
    }
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

fn action_fixture() -> String {
    format!(
        "{}\n{}\n#[cfg(test)]\nmod tests {{}}",
        EXPECTED_DOMAIN_PRODUCTION, ACTION_DOMAIN_ADDITIONS
    )
}

fn action_fixture_with_extra(extra: &str) -> String {
    let source = action_fixture();
    let marker = "#[cfg(test)]\nmod tests";
    let offset = source.find(marker).expect("fixture wrapper");
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

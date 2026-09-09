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
#[path = "support/source_closure.rs"]
mod source_closure;
#[path = "support/source_root.rs"]
mod source_root;
#[path = "support/symlink_fixture.rs"]
mod symlink_fixture;

use boundary::{
    source_boundary, SourceBoundaryKind, ACTION_DOMAIN_ADDITIONS, CATALOG_PRODUCTION,
    EXPECTED_DOMAIN_PRODUCTION, READ_MODEL_DOMAIN_PRODUCTION,
};
use manifest::{crate_root, manifest_boundary, ManifestFixture};
use production_sources::production_sources;
use rust_lexer::{RustLexer, RustToken};
use source_closure::{
    assert_source_rejection, asset_source_fixture, catalog_source_fixture, source_closure_boundary,
    SourceClosureError,
};
use source_root::SourceRootError;

#[test]
fn task_one_production_source_closure_has_no_persistence_or_product_surface() {
    source_closure_boundary(&crate_root().join("src")).expect("production source closure");
    let assets = asset_source_fixture();
    source_closure_boundary(&assets.root.join("src")).expect("next asset source closure");
}

#[test]
fn http_gate_accepts_old_and_pinned_next_closure() {
    for (catalog_file, catalog_lib, http_file, http_lib, expected) in [
        (false, false, false, false, Ok(())),
        (true, true, false, false, Ok(())),
        (true, true, true, true, Ok(())),
        (
            true,
            true,
            true,
            false,
            Err(SourceClosureError::LibInventoryPair),
        ),
        (
            true,
            true,
            false,
            true,
            Err(SourceClosureError::LibInventoryPair),
        ),
        (
            true,
            false,
            true,
            false,
            Err(SourceClosureError::LibInventoryPair),
        ),
        (
            false,
            false,
            false,
            true,
            Err(SourceClosureError::LibInventoryPair),
        ),
        (
            false,
            false,
            true,
            false,
            Err(SourceClosureError::Inventory),
        ),
        (false, true, true, true, Err(SourceClosureError::Inventory)),
    ] {
        let fixture = catalog_source_fixture(catalog_file, catalog_lib, http_file, http_lib);
        assert_eq!(
            source_closure_boundary(&fixture.root.join("src")),
            expected,
            "catalog file={catalog_file}, lib={catalog_lib}; HTTP file={http_file}, lib={http_lib}"
        );
    }
    for (missing, expected) in [
        ("domain.rs", SourceClosureError::Inventory),
        ("lib.rs", SourceClosureError::Inventory),
        ("catalog.rs", SourceClosureError::Inventory),
        ("http.rs", SourceClosureError::LibInventoryPair),
    ] {
        let fixture = catalog_source_fixture(true, true, true, true);
        let root = fixture.root.join("src");
        assert_eq!(source_closure_boundary(&root), Ok(()));
        fs::remove_file(root.join(missing)).expect("remove required source");
        assert_eq!(
            source_closure_boundary(&root),
            Err(expected),
            "missing {missing}"
        );
    }
    for unknown in [
        "unknown.rs",
        "nested/unknown.rs",
        "http/mod.rs",
        "nested/http.rs",
    ] {
        let fixture = catalog_source_fixture(true, true, true, true);
        let root = fixture.root.join("src");
        assert_eq!(source_closure_boundary(&root), Ok(()));
        let extra = root.join(unknown);
        fs::create_dir_all(extra.parent().expect("extra parent")).expect("extra directory");
        fs::write(extra, "").expect("extra source");
        assert_eq!(
            source_closure_boundary(&root),
            Err(SourceClosureError::Inventory),
            "{unknown}"
        );
    }
    for (filename, source, expected) in [
        ("http.rs", "", SourceBoundaryKind::HttpTestModuleShape),
        (
            "http.rs",
            "#[path=\"../outside.rs\"] mod escaped; #[cfg(test)] mod tests {}",
            SourceBoundaryKind::PathAttribute,
        ),
        (
            "lib.rs",
            "#[path=\"../outside.rs\"] mod http;",
            SourceBoundaryKind::PathAttribute,
        ),
    ] {
        let fixture = catalog_source_fixture(true, true, true, true);
        let root = fixture.root.join("src");
        assert_eq!(source_closure_boundary(&root), Ok(()));
        fs::write(root.join(filename), source).expect("mutated source");
        assert_eq!(
            source_closure_boundary(&root),
            Err(SourceClosureError::Source(expected))
        );
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
fn catalog_gate_accepts_static_text_variants_only() {
    let source = format!("{CATALOG_PRODUCTION}\n#[cfg(test)] mod tests {{}}");
    source_boundary(Path::new("catalog.rs"), &source).expect("complete static catalog");
    let tokens = RustLexer::lex(CATALOG_PRODUCTION).expect("frozen fixture must lex");
    let mut changed_all = source.clone();
    let mut text_count = 0;
    for window in tokens.windows(3) {
        if let [RustToken::Ident(field), RustToken::Punct(':'), RustToken::Literal(value)] = window
        {
            if field == "en" || field == "zh" {
                let target = format!("{field}: {value}");
                let replacement =
                    format!("{field}: \"文字 \\\"quoted\\\" {{ std::fs::write() }}\"");
                assert!(source.contains(&target));
                let changed = source.replacen(&target, &replacement, 1);
                assert_ne!(changed, source);
                source_boundary(Path::new("catalog.rs"), &changed).expect("ordinary text is data");
                changed_all = changed_all.replacen(&target, &replacement, 1);
                text_count += 1;
            }
        }
    }
    assert_eq!(text_count, 54);
    assert_ne!(changed_all, source);
    source_boundary(Path::new("catalog.rs"), &changed_all).expect("all 54 text values may change");
}

#[test]
fn catalog_gate_rejects_structure_key_and_code_mutations() {
    let source = format!("{CATALOG_PRODUCTION}\n#[cfg(test)] mod tests {{}}");
    for (name, target, replacement) in [
        ("key", "key: \"page_title\"", "key: \"other\""),
        ("length", "[CatalogEntry; 27]", "[CatalogEntry; 28]"),
        ("type", "en: &'static str", "en: String"),
        ("field", "en: &'static str", "english: &'static str"),
        ("missing field", "    zh: &'static str,", ""),
        (
            "extra field",
            "    zh: &'static str,",
            "    zh: &'static str, extra: bool,",
        ),
        (
            "field order",
            "    key: &'static str,\n    en: &'static str,",
            "    en: &'static str,\n    key: &'static str,",
        ),
        ("public field", "en: &'static str", "pub en: &'static str"),
        (
            "public type",
            "pub(crate) struct CatalogEntry",
            "pub struct CatalogEntry",
        ),
        (
            "public const",
            "pub(crate) const CATALOG",
            "pub const CATALOG",
        ),
        ("private const", "pub(crate) const CATALOG", "const CATALOG"),
        ("derive", "serde::Serialize", "serde::Deserialize"),
        (
            "Deserialize",
            "serde::Serialize",
            "serde::Serialize, serde::Deserialize",
        ),
        ("missing Copy", "Clone, Copy, Debug", "Clone, Debug"),
        (
            "serde attribute",
            "pub(crate) struct CatalogEntry",
            "#[serde(rename_all = \"UPPERCASE\")] pub(crate) struct CatalogEntry",
        ),
        (
            "function",
            "pub(crate) const CATALOG",
            "fn leak() {}\npub(crate) const CATALOG",
        ),
        (
            "custom serializer",
            "];",
            "]; impl serde::Serialize for CatalogEntry {}",
        ),
        (
            "constructor",
            "];",
            "]; impl CatalogEntry { fn new() -> Self { loop {} } }",
        ),
        (
            "lookup",
            "];",
            "]; fn lookup(key: &str) -> Option<CatalogEntry> { None }",
        ),
        (
            "IO",
            "];",
            "]; fn leak() { std::fs::write(\"leak\", \"data\").unwrap(); }",
        ),
        (
            "environment",
            "];",
            "]; const LEAK: &str = env!(\"SECRET\");",
        ),
        (
            "process",
            "];",
            "]; fn leak() { std::process::Command::new(\"sh\").spawn().unwrap(); }",
        ),
        ("unsafe", "];", "]; unsafe extern \"C\" { fn leak(); }"),
        ("include", "];", "]; include!(\"outside.rs\");"),
        ("trailing", "];", "]; fn leak() {}"),
        (
            "extra token",
            "en: \"Consultant Playground\"",
            "en: \"Consultant Playground\" true",
        ),
        (
            "raw literal",
            "en: \"Consultant Playground\"",
            "en: r#\"changed\"#",
        ),
        (
            "raw literal without hash",
            "en: \"Consultant Playground\"",
            "en: r\"changed\"",
        ),
        (
            "byte literal",
            "en: \"Consultant Playground\"",
            "en: b\"changed\"",
        ),
        (
            "raw byte literal",
            "en: \"Consultant Playground\"",
            "en: br#\"changed\"#",
        ),
        (
            "c literal",
            "en: \"Consultant Playground\"",
            "en: c\"changed\"",
        ),
        (
            "raw c literal",
            "en: \"Consultant Playground\"",
            "en: cr#\"changed\"#",
        ),
        ("identifier", "en: \"Consultant Playground\"", "en: OTHER"),
        ("number", "en: \"Consultant Playground\"", "en: 42"),
        ("character", "en: \"Consultant Playground\"", "en: 'x'"),
        ("macro", "en: \"Consultant Playground\"", "en: env!(\"X\")"),
        (
            "include macro",
            "en: \"Consultant Playground\"",
            "en: include_str!(\"outside\")",
        ),
        (
            "concat macro",
            "en: \"Consultant Playground\"",
            "en: concat!(\"a\", \"b\")",
        ),
        (
            "expression",
            "en: \"Consultant Playground\"",
            "en: \"a\" + \"b\"",
        ),
        ("block", "en: \"Consultant Playground\"", "en: { \"text\" }"),
        (
            "adjacent strings",
            "en: \"Consultant Playground\"",
            "en: \"a\" \"b\"",
        ),
        ("zh expression", "zh: \"顾问练习场\"", "zh: TEXT"),
    ] {
        assert!(source.contains(target), "missing {name} target");
        let changed = source.replacen(target, replacement, 1);
        assert_ne!(source, changed, "mutation {name} must take effect");
        assert_source_rejection(
            name,
            Path::new("catalog.rs"),
            &changed,
            SourceBoundaryKind::CatalogProductionShape,
        );
    }
    let entries: Vec<_> = CATALOG_PRODUCTION
        .lines()
        .filter(|line| line.contains("CatalogEntry { key:"))
        .collect();
    assert_eq!(entries.len(), 27);
    for entry in &entries {
        let key = entry
            .split_once("key: ")
            .expect("fixed key field")
            .1
            .split_once(',')
            .expect("key terminator")
            .0;
        let changed = source.replacen(&format!("key: {key}"), "key: \"changed\"", 1);
        assert_ne!(changed, source);
        assert_source_rejection(
            key,
            Path::new("catalog.rs"),
            &changed,
            SourceBoundaryKind::CatalogProductionShape,
        );
    }
    for (name, changed) in [
        ("missing entry", source.replacen(entries[0], "", 1)),
        (
            "duplicate entry",
            source.replacen(entries[1], entries[0], 1),
        ),
        (
            "extra entry",
            source.replacen("];", &format!("{}\n];", entries[0]), 1),
        ),
        (
            "entry order",
            source.replacen(
                &format!("{}\n{}", entries[0], entries[1]),
                &format!("{}\n{}", entries[1], entries[0]),
                1,
            ),
        ),
    ] {
        assert_ne!(changed, source, "{name}");
        assert_source_rejection(
            name,
            Path::new("catalog.rs"),
            &changed,
            SourceBoundaryKind::CatalogProductionShape,
        );
    }
    let path = format!(
        "{CATALOG_PRODUCTION}\n#[path=\"../outside.rs\"] mod escaped;\n#[cfg(test)] mod tests {{}}"
    );
    assert_ne!(path, source);
    assert_source_rejection(
        "catalog path",
        Path::new("catalog.rs"),
        &path,
        SourceBoundaryKind::PathAttribute,
    );
}

#[test]
fn catalog_gate_preserves_wrapper_and_source_closure() {
    for (catalog_file, catalog_lib, expected) in [
        (false, false, Ok(())),
        (true, true, Ok(())),
        (false, true, Err(SourceClosureError::LibInventoryPair)),
        (true, false, Err(SourceClosureError::LibInventoryPair)),
    ] {
        let fixture = catalog_source_fixture(catalog_file, catalog_lib, false, false);
        assert_eq!(
            source_closure_boundary(&fixture.root.join("src")),
            expected,
            "catalog file={catalog_file}, lib declaration={catalog_lib}"
        );
    }
    for catalog in [false, true] {
        for unknown in ["unknown.rs", "nested/unknown.rs"] {
            let fixture = catalog_source_fixture(catalog, catalog, false, false);
            let source_root = fixture.root.join("src");
            assert_eq!(source_closure_boundary(&source_root), Ok(()));
            let path = source_root.join(unknown);
            fs::create_dir_all(path.parent().expect("parent")).expect("fixture subdir");
            fs::write(path, "").expect("unknown fixture source");
            assert_eq!(
                source_closure_boundary(&source_root),
                Err(SourceClosureError::Inventory),
                "{unknown}"
            );
        }
        for missing in ["lib.rs", "domain.rs"] {
            let fixture = catalog_source_fixture(catalog, catalog, false, false);
            let source_root = fixture.root.join("src");
            assert_eq!(source_closure_boundary(&source_root), Ok(()));
            fs::remove_file(source_root.join(missing)).expect("remove required source");
            assert_eq!(
                source_closure_boundary(&source_root),
                Err(SourceClosureError::Inventory),
                "missing {missing}"
            );
        }
    }
    for (filename, source, expected) in [
        ("catalog.rs", "", SourceBoundaryKind::CatalogTestModuleShape),
        (
            "lib.rs",
            "#[path=\"../outside.rs\"] mod domain;",
            SourceBoundaryKind::PathAttribute,
        ),
    ] {
        let fixture = catalog_source_fixture(true, true, false, false);
        let source_root = fixture.root.join("src");
        assert_eq!(source_closure_boundary(&source_root), Ok(()));
        fs::write(source_root.join(filename), source).expect("mutated fixture source");
        assert_eq!(
            source_closure_boundary(&source_root),
            Err(SourceClosureError::Source(expected))
        );
    }
    let fixture = ManifestFixture::new("");
    assert_eq!(
        source_closure_boundary(&fixture.root.join("missing")),
        Err(SourceClosureError::Root(
            SourceRootError::MetadataUnreadable
        ))
    );
    assert_eq!(
        source_closure_boundary(&fixture.manifest),
        Err(SourceClosureError::Root(SourceRootError::RootNotDirectory))
    );
    let source = format!("{CATALOG_PRODUCTION}\n#[cfg(test)] mod tests {{}}");
    for (name, changed) in [
        (
            "changed wrapper",
            source.replacen("#[cfg(test)] mod tests", "#[cfg(any())] mod tests", 1),
        ),
        ("missing wrapper", CATALOG_PRODUCTION.to_string()),
        (
            "external wrapper",
            source.replace("mod tests {}", "mod tests;"),
        ),
        ("wrapper not terminal", format!("{source} fn leak() {{}}")),
        (
            "duplicate wrapper",
            format!("{source} #[cfg(test)] mod tests {{}}"),
        ),
        (
            "unbalanced wrapper",
            format!("{CATALOG_PRODUCTION}\n#[cfg(test)] mod tests {{"),
        ),
    ] {
        assert_ne!(changed, source, "{name}");
        assert_source_rejection(
            name,
            Path::new("catalog.rs"),
            &changed,
            SourceBoundaryKind::CatalogTestModuleShape,
        );
    }
    assert_source_rejection(
        "unknown source",
        Path::new("other.rs"),
        &source,
        SourceBoundaryKind::UnexpectedSourceFile,
    );
}

#[cfg(any(unix, windows))]
#[test]
fn catalog_gate_rejects_source_symlink_escapes() {
    let fixture = match symlink_fixture::SymlinkedSourceFixture::new() {
        Ok(fixture) => fixture,
        #[cfg(windows)]
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            eprintln!("directory symlinks are unavailable without Windows developer privilege");
            return;
        }
        Err(error) => panic!("symlink fixture: {error}"),
    };
    assert_eq!(
        source_closure_boundary(&fixture.source_link),
        Err(SourceClosureError::Root(SourceRootError::RootSymlink))
    );
    let valid = catalog_source_fixture(true, true, false, false);
    let source_root = valid.root.join("src");
    assert_eq!(source_closure_boundary(&source_root), Ok(()));
    // Reuse the existing directory symlink fixture inside an otherwise valid
    // source inventory; the existing scanner must reject it before traversal.
    let directory_link = source_root.join("linked");
    fs::rename(&fixture.source_link, &directory_link).expect("move fixture symlink");
    let error = std::panic::catch_unwind(|| source_closure_boundary(&source_root))
        .expect_err("source symlink must panic in the existing scanner");
    let message = error
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| error.downcast_ref::<&str>().copied())
        .expect("scanner panic message");
    assert!(
        message.contains("production sources must not be symlinks"),
        "{message}"
    );
    for (http, filename) in [
        (false, "catalog.rs"),
        (false, "domain.rs"),
        (false, "lib.rs"),
        (true, "catalog.rs"),
        (true, "domain.rs"),
        (true, "lib.rs"),
        (true, "http.rs"),
    ] {
        let valid = catalog_source_fixture(true, true, http, http);
        let source_root = valid.root.join("src");
        assert_eq!(source_closure_boundary(&source_root), Ok(()));
        let file = source_root.join(filename);
        let outside = valid.root.join("outside.rs");
        fs::rename(&file, &outside).expect("move source outside root");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&outside, &file).expect("source symlink");
        #[cfg(windows)]
        std::os::windows::fs::symlink_file(&outside, &file).expect("source symlink");
        let error = std::panic::catch_unwind(|| source_closure_boundary(&source_root))
            .expect_err("file symlink must panic in existing scanner");
        let message = error
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| error.downcast_ref::<&str>().copied())
            .expect("scanner panic message");
        assert!(
            message.contains("production sources must not be symlinks"),
            "{filename}: {message}"
        );
    }
}

#[test]
fn read_model_gate_accepts_only_pinned_stage_grammars() {
    for (name, production) in [
        ("legacy", EXPECTED_DOMAIN_PRODUCTION.to_string()),
        ("actions", action_production_fixture()),
        ("DTO", READ_MODEL_DOMAIN_PRODUCTION.to_string()),
    ] {
        let source = format!("{production}\n#[cfg(test)]\nmod tests {{}}");
        source_boundary(Path::new("domain.rs"), &source)
            .unwrap_or_else(|error| panic!("{name}: {error}"));
    }
    let (visible_actions, dto) = READ_MODEL_DOMAIN_PRODUCTION
        .split_once("#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]")
        .expect("static fixture contains DTO declaration");
    let dto = format!("#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]{dto}");
    let incomplete = [
        ("visibility only", visible_actions.to_string()),
        ("DTO only", dto.clone()),
        (
            "private actions plus DTO",
            format!("{}{dto}", action_production_fixture()),
        ),
        (
            "missing actions",
            format!("{}{dto}", EXPECTED_DOMAIN_PRODUCTION),
        ),
    ];
    for (name, production) in incomplete {
        assert_source_rejection(
            name,
            Path::new("domain.rs"),
            &format!("{production}\n#[cfg(test)]\nmod tests {{}}"),
            SourceBoundaryKind::DomainProductionShape,
        );
    }
}

#[test]
fn read_model_gate_rejects_projection_mutations() {
    let fixture = format!("{READ_MODEL_DOMAIN_PRODUCTION}\n#[cfg(test)]\nmod tests {{}}");
    let mutations = [
        ("profile metadata", "profile: \"synthetic_playground\"", "profile: \"changed\""),
        ("real data metadata", "real_data_enabled: false", "real_data_enabled: true"),
        ("persistence metadata", "persistence: \"none\"", "persistence: \"disk\""),
        ("catalog key", "\"reporting_clarity_sprint\"", "\"changed\""),
        ("field type", "offer_price_usd_cents: u32", "offer_price_usd_cents: u64"),
        ("field name", "persistence: &'static str", "other: &'static str"),
        ("field missing", "    persistence: &'static str,", ""),
        ("field order", "    profile: &'static str,\n    real_data_enabled: bool,", "    real_data_enabled: bool,\n    profile: &'static str,"),
        ("field public", "    profile: &'static str,", "    pub profile: &'static str,"),
        ("value source", "company_name: self.graph.company.name", "company_name: self.graph.relationship.organization"),
        ("stage projection", "RelationshipStage::Lead => \"lead\"", "RelationshipStage::Lead => \"customer\""),
        ("graph public", "struct ConsultantPlaygroundGraph", "pub(crate) struct ConsultantPlaygroundGraph"),
        ("graph field public", "    graph: ConsultantPlaygroundGraph,", "    pub(crate) graph: ConsultantPlaygroundGraph,"),
        ("serialize graph", "struct ConsultantPlaygroundGraph", "#[derive(serde::Serialize)] struct ConsultantPlaygroundGraph"),
        ("serialize session", "pub(crate) struct PlaygroundSession", "#[derive(serde::Serialize)] pub(crate) struct PlaygroundSession"),
        ("serialize action", "pub(crate) enum PlaygroundAction", "#[derive(serde::Serialize)] pub(crate) enum PlaygroundAction"),
        ("serialize domain", "enum SemanticKey", "#[derive(serde::Serialize)] enum SemanticKey"),
        ("DTO deserialize", "PartialEq, serde::Serialize", "PartialEq, serde::Serialize, serde::Deserialize"),
        ("DTO loses Copy", "#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]", "#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]"),
        ("mutable receiver", "fn read_model(&self)", "fn read_model(&mut self)"),
        ("graph reference", "fn read_model(&self) -> PlaygroundReadModel", "fn read_model(&self) -> &ConsultantPlaygroundGraph"),
        ("extra parameter", "fn read_model(&self)", "fn read_model(&self, input: u32)"),
        ("projection write", "profile: \"synthetic_playground\"", "profile: { self.graph.offer.price_usd_cents = 1; \"synthetic_playground\" }"),
        ("projection IO", "company_name: self.graph.company.name", "company_name: { std::fs::write(\"leak\", \"data\").unwrap(); self.graph.company.name }"),
        ("projection env", "company_name: self.graph.company.name", "company_name: env!(\"SECRET\")"),
        ("projection process", "company_name: self.graph.company.name", "company_name: { std::process::Command::new(\"sh\").spawn().unwrap(); self.graph.company.name }"),
        ("projection unsafe", "company_name: self.graph.company.name", "company_name: unsafe { self.graph.company.name }"),
        ("read model visibility", "pub(crate) fn read_model", "pub fn read_model"),
        ("session visibility missing", "pub(crate) struct PlaygroundSession", "struct PlaygroundSession"),
        ("action visibility missing", "pub(crate) enum PlaygroundAction", "enum PlaygroundAction"),
        ("new visibility missing", "pub(crate) fn new", "fn new"),
        ("apply visibility missing", "pub(crate) fn apply", "fn apply"),
    ];
    for (name, target, replacement) in mutations {
        assert!(fixture.contains(target), "missing target for {name}");
        let changed = fixture.replacen(target, replacement, 1);
        assert_ne!(changed, fixture, "mutation had no effect for {name}");
        assert_source_rejection(
            name,
            Path::new("domain.rs"),
            &changed,
            SourceBoundaryKind::DomainProductionShape,
        );
    }
    for extra in [
        "impl From<PlaygroundReadModel> for PlaygroundSession { fn from(_: PlaygroundReadModel) -> Self { Self::new() } }",
        "impl From<PlaygroundReadModel> for ConsultantPlaygroundGraph { fn from(_: PlaygroundReadModel) -> Self { PlaygroundSession::new().graph } }",
        "impl PlaygroundSession { fn extra(&self) {} }",
        "impl serde::Serialize for PlaygroundReadModel {}",
        "use std::{fs as disk};",
    ] {
        let source = format!("{READ_MODEL_DOMAIN_PRODUCTION}\n{extra}\n#[cfg(test)]\nmod tests {{}}");
        assert_source_rejection(extra, Path::new("domain.rs"), &source, SourceBoundaryKind::DomainProductionShape);
    }
    for attribute in [
        "#[serde(rename = \"changed\")]",
        "#[serde(flatten)]",
        "#[serde(default)]",
        "#[serde(skip)]",
        "#[serde(serialize_with = \"custom\")]",
    ] {
        let source = fixture.replacen(
            "    profile: &'static str,",
            &format!("    {attribute} profile: &'static str,"),
            1,
        );
        assert_source_rejection(
            attribute,
            Path::new("domain.rs"),
            &source,
            SourceBoundaryKind::DomainProductionShape,
        );
    }
    let path = format!("{READ_MODEL_DOMAIN_PRODUCTION}\n#[path=\"../outside.rs\"] mod escaped;\n#[cfg(test)] mod tests {{}}");
    assert_source_rejection(
        "DTO path escape",
        Path::new("domain.rs"),
        &path,
        SourceBoundaryKind::PathAttribute,
    );
    for (name, source) in [
        ("wrapper missing", READ_MODEL_DOMAIN_PRODUCTION.to_string()),
        (
            "wrapper cfg changed",
            fixture.replace("#[cfg(test)]", "#[cfg(any())]"),
        ),
        (
            "wrapper not terminal",
            format!("{fixture} fn escaped() {{}}"),
        ),
        (
            "wrapper duplicated",
            format!("{fixture} #[cfg(test)] mod tests {{}}"),
        ),
    ] {
        assert_source_rejection(
            name,
            Path::new("domain.rs"),
            &source,
            SourceBoundaryKind::DomainTestModuleShape,
        );
    }
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
        action_production_fixture(),
        ""
    )
}

fn action_production_fixture() -> String {
    format!(
        "{}\n{}",
        EXPECTED_DOMAIN_PRODUCTION, ACTION_DOMAIN_ADDITIONS
    )
}

fn action_fixture_with_extra(extra: &str) -> String {
    let source = action_fixture();
    let marker = "#[cfg(test)]\nmod tests";
    let offset = source.find(marker).expect("fixture wrapper");
    format!("{}\n{extra}\n{}", &source[..offset], &source[offset..])
}

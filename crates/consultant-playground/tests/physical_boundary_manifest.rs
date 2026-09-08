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

use std::fs;

use json::{JsonParser, JsonValue};
use manifest::{crate_root, manifest_boundary, validate_dependencies, ManifestFixture};

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

#[test]
fn cargo_metadata_rejects_features_build_and_library_escapes() {
    for (name, declaration, filename, content, expected_error) in [
        (
            "features",
            "[features]\nextra = []",
            "unused.rs",
            "",
            "feature declarations",
        ),
        (
            "auto build script",
            "",
            "build.rs",
            "fn main() {}",
            "build script",
        ),
        (
            "custom library path",
            "[lib]\npath = \"outside.rs\"",
            "outside.rs",
            "",
            "outside `src/lib.rs`",
        ),
        (
            "binary",
            "[[bin]]\nname = \"escape\"\npath = \"escape.rs\"",
            "escape.rs",
            "fn main() {}",
            "unexpected target kinds",
        ),
    ] {
        let fixture = ManifestFixture::new(declaration);
        fs::write(fixture.root.join(filename), content).expect("fixture file");
        let error = manifest_boundary(&fixture.manifest, "boundary-fixture")
            .expect_err("package boundary must reject mutation");
        assert!(error.contains(expected_error), "{name}: {error}");
    }
}

#[test]
fn manifest_accepts_only_empty_or_exact_dto_dependencies() {
    let declarations = [
        "",
        "[dependencies]\nserde = { version = \"1\", features = [\"derive\"] }\nserde_json = \"1\"",
        "[workspace.dependencies]\nserde = { version = \"1\", features = [\"derive\"] }\nserde_json = \"1\"\n[dependencies]\nserde = { workspace = true }\nserde_json = { workspace = true }",
    ];
    for declaration in declarations {
        let fixture = ManifestFixture::new(declaration);
        manifest_boundary(&fixture.manifest, "boundary-fixture")
            .unwrap_or_else(|error| panic!("exact dependency set: {declaration}: {error}"));
    }
    for raw in [
        "[]".to_string(),
        DTO_DEPENDENCIES.to_string(),
        DTO_DEPENDENCIES.replace(",\"path\":null", ""),
    ] {
        validate_dependencies(&JsonParser::parse(raw.as_bytes()).expect("static JSON"))
            .expect("frozen metadata, including omitted path, must pass");
    }
}

#[test]
fn manifest_rejects_dto_dependency_mutations() {
    let declarations = [
        ("serde alone", "[dependencies]\nserde = { version = \"1\", features = [\"derive\"] }"),
        ("serde_json alone", "[dependencies]\nserde_json = \"1\""),
        ("third item", "[dependencies]\nserde = { version = \"1\", features = [\"derive\"] }\nserde_json = \"1\"\nforbidden = { path = \"dep\" }"),
        ("dev", "[dev-dependencies]\nserde = { version = \"1\", features = [\"derive\"] }\nserde_json = \"1\""),
        ("build", "[build-dependencies]\nserde = { version = \"1\", features = [\"derive\"] }\nserde_json = \"1\""),
        ("target", "[target.'cfg(unix)'.dependencies]\nserde = { version = \"1\", features = [\"derive\"] }\nserde_json = \"1\""),
        ("target dev", "[target.'cfg(unix)'.dev-dependencies]\nserde = { version = \"1\", features = [\"derive\"] }\nserde_json = \"1\""),
        ("target build", "[target.'cfg(unix)'.build-dependencies]\nserde = { version = \"1\", features = [\"derive\"] }\nserde_json = \"1\""),
        ("dotted optional", "[dependencies]\nserde.version = \"1\"\nserde.features = [\"derive\"]\nserde.optional = true\nserde_json = \"1\""),
        ("table optional", "[dependencies.serde]\nversion = \"1\"\nfeatures = [\"derive\"]\noptional = true\n[dependencies.serde_json]\nversion = \"1\""),
        ("alias", "[dependencies]\nrenamed = { package = \"serde\", version = \"1\", features = [\"derive\"] }\nserde_json = \"1\""),
        ("optional", "[dependencies]\nserde = { version = \"1\", features = [\"derive\"], optional = true }\nserde_json = \"1\""),
        ("missing derive", "[dependencies]\nserde = \"1\"\nserde_json = \"1\""),
        ("extra serde feature", "[dependencies]\nserde = { version = \"1\", features = [\"derive\", \"alloc\"] }\nserde_json = \"1\""),
        ("extra json feature", "[dependencies]\nserde = { version = \"1\", features = [\"derive\"] }\nserde_json = { version = \"1\", features = [\"preserve_order\"] }"),
        ("defaults disabled", "[dependencies]\nserde = { version = \"1\", features = [\"derive\"], default-features = false }\nserde_json = \"1\""),
        ("version", "[dependencies]\nserde = { version = \"=1.0.228\", features = [\"derive\"] }\nserde_json = \"1\""),
        ("git source", "[dependencies]\nserde = { git = \"https://example.invalid/serde\", features = [\"derive\"] }\nserde_json = \"1\""),
        ("path source", "[dependencies]\nserde = { path = \"dep\", package = \"forbidden\" }\nserde_json = \"1\""),
    ];
    for (name, declaration) in declarations {
        let fixture = ManifestFixture::new(declaration);
        assert!(
            manifest_boundary(&fixture.manifest, "boundary-fixture").is_err(),
            "mutation must be rejected: {name}: {declaration}"
        );
    }
    // Exercise the same validator directly so Cargo rejecting a malformed TOML
    // fixture cannot mask missing field/type checks in our gate.
    let mutations = [
        ("name", "\"other\""),
        ("req", "\"=1.0.228\""),
        ("kind", "\"dev\""),
        ("kind", "\"build\""),
        ("target", "\"cfg(unix)\""),
        ("rename", "\"alias\""),
        ("registry", "\"https://other.invalid\""),
        ("source", "null"),
        ("source", "\"git+https://other.invalid\""),
        ("path", "\"../outside\""),
        ("optional", "true"),
        ("uses_default_features", "false"),
        ("features", "[\"extra\"]"),
        ("features", "[\"derive\",\"derive\"]"),
        ("features", "[false]"),
    ];
    for index in 0..2 {
        for (field, raw) in mutations {
            let mut dependencies =
                JsonParser::parse(DTO_DEPENDENCIES.as_bytes()).expect("static JSON");
            let JsonValue::Array(items) = &mut dependencies else {
                panic!("array")
            };
            let JsonValue::Object(fields) = &mut items[index] else {
                panic!("object")
            };
            assert!(fields
                .insert(
                    field.into(),
                    JsonParser::parse(raw.as_bytes()).expect("mutation JSON")
                )
                .is_some());
            assert!(
                validate_dependencies(&dependencies).is_err(),
                "dependency {index}, {field}={raw}"
            );
        }
        let mut dependencies = JsonParser::parse(DTO_DEPENDENCIES.as_bytes()).expect("static JSON");
        let JsonValue::Array(items) = &mut dependencies else {
            panic!("array")
        };
        let JsonValue::Object(fields) = &mut items[index] else {
            panic!("object")
        };
        fields.insert(
            "features".into(),
            JsonParser::parse(if index == 0 { b"[]" } else { br#"["derive"]"# })
                .expect("static JSON"),
        );
        assert!(
            validate_dependencies(&dependencies).is_err(),
            "wrong features for dependency {index}"
        );
    }
    for raw in [
        DTO_DEPENDENCIES.replacen("\"serde_json\"", "\"serde\"", 1),
        DTO_DEPENDENCIES.replacen("\"serde\"", "\"serde_json\"", 1),
    ] {
        assert!(
            validate_dependencies(&JsonParser::parse(raw.as_bytes()).expect("static JSON"))
                .is_err()
        );
    }
}

#[test]
fn metadata_json_distinguishes_boolean_null_and_number() {
    let parsed = JsonParser::parse(br#"[true,false,null,42]"#).expect("valid JSON");
    let JsonValue::Array(values) = parsed else {
        panic!("expected array")
    };
    assert!(matches!(values[0], JsonValue::Bool(true)));
    assert!(matches!(values[1], JsonValue::Bool(false)));
    assert!(matches!(values[2], JsonValue::Null));
    assert!(matches!(values[3], JsonValue::Scalar));
    for raw in ["wat", "undefined", "nul", "True", "falsehood", "nullx"] {
        assert!(
            JsonParser::parse(raw.as_bytes()).is_err(),
            "unknown input {raw}"
        );
    }
    let fields = [
        "name",
        "req",
        "kind",
        "target",
        "rename",
        "registry",
        "source",
        "optional",
        "uses_default_features",
        "features",
        "path",
    ];
    for index in 0..2 {
        for field in fields {
            let wrong_types: &[&str] = match field {
                "kind" | "target" | "rename" | "registry" | "path" => {
                    &["true", "false", "0", "\"null\"", "[]", "{}"]
                }
                "optional" | "uses_default_features" => {
                    &["null", "0", "\"false\"", "\"true\"", "[]", "{}"]
                }
                "features" => &["null", "true", "false", "0", "\"derive\"", "{}"],
                _ => &["null", "true", "false", "0", "[]", "{}"],
            };
            for raw in wrong_types
                .iter()
                .copied()
                .chain(std::iter::once("missing"))
            {
                let mut dependencies =
                    JsonParser::parse(DTO_DEPENDENCIES.as_bytes()).expect("static JSON");
                let JsonValue::Array(items) = &mut dependencies else {
                    panic!("array")
                };
                let JsonValue::Object(fields) = &mut items[index] else {
                    panic!("object")
                };
                if raw == "missing" {
                    assert!(fields.remove(field).is_some());
                } else {
                    fields.insert(
                        field.into(),
                        JsonParser::parse(raw.as_bytes()).expect("mutation JSON"),
                    );
                }
                assert_eq!(
                    validate_dependencies(&dependencies).is_ok(),
                    field == "path" && raw == "missing",
                    "dependency {index}, {field}={raw}"
                );
            }
        }
    }
    for raw in [
        "null",
        "true",
        "false",
        "42",
        "{}",
        "\"dependencies\"",
        "[null,null]",
    ] {
        assert!(
            validate_dependencies(&JsonParser::parse(raw.as_bytes()).expect("valid JSON")).is_err(),
            "wrong dependencies type {raw}"
        );
    }
}

const DTO_DEPENDENCIES: &str = r#"[
    {"name":"serde","req":"^1","kind":null,"target":null,"rename":null,"registry":null,"source":"registry+https://github.com/rust-lang/crates.io-index","optional":false,"uses_default_features":true,"features":["derive"],"path":null},
    {"name":"serde_json","req":"^1","kind":null,"target":null,"rename":null,"registry":null,"source":"registry+https://github.com/rust-lang/crates.io-index","optional":false,"uses_default_features":true,"features":[],"path":null}
]"#;

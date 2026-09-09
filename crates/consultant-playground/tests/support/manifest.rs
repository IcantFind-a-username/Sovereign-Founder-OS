//! `cargo metadata`-backed manifest boundary check, plus the disposable
//! fixture packages the manifest tests mutate.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::json::{
    expect_json_string, json_array, json_field, json_object, json_string, JsonParser, JsonValue,
};
use crate::source_root::source_root_boundary;

static FIXTURE_SEQUENCE: AtomicUsize = AtomicUsize::new(0);

pub(crate) fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub(crate) fn cargo_metadata(manifest_path: &Path) -> Result<JsonValue, String> {
    let mut command = Command::new(
        std::env::var_os("CARGO").unwrap_or_else(|| std::ffi::OsString::from("cargo")),
    );
    command.args([
        "metadata",
        "--format-version",
        "1",
        "--no-deps",
        "--offline",
        "--manifest-path",
    ]);
    command.arg(manifest_path);
    if manifest_path == crate_root().join("Cargo.toml") {
        command.arg("--locked");
    }
    let output = command
        .output()
        .map_err(|error| format!("could not run Cargo metadata: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "Cargo metadata rejected the manifest: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    JsonParser::parse(&output.stdout)
}

pub(crate) fn manifest_boundary(manifest_path: &Path, expected_name: &str) -> Result<(), String> {
    let metadata = cargo_metadata(manifest_path)?;
    let packages = json_array(json_field(&metadata, "packages")?, "packages")?;
    let canonical_manifest = fs::canonicalize(manifest_path)
        .map_err(|error| format!("could not canonicalize manifest: {error}"))?;
    let mut matching = packages.iter().filter(|package| {
        let Ok(path_value) = json_field(package, "manifest_path") else {
            return false;
        };
        let Ok(path) = json_string(path_value, "manifest_path") else {
            return false;
        };
        fs::canonicalize(path).is_ok_and(|path| path == canonical_manifest)
    });
    let package = matching
        .next()
        .ok_or_else(|| "Cargo metadata omitted the selected package".to_string())?;
    if matching.next().is_some() {
        return Err("Cargo metadata returned the selected package twice".into());
    }

    expect_json_string(package, "name", expected_name)?;
    expect_json_string(package, "version", "0.1.0")?;
    expect_json_string(package, "edition", "2021")?;
    expect_json_string(package, "rust_version", "1.97")?;
    expect_json_string(package, "license", "Apache-2.0")?;
    expect_json_string(
        package,
        "repository",
        "https://github.com/IcantFind-a-username/Sovereign-Founder-OS",
    )?;

    match json_field(package, "publish")? {
        JsonValue::Array(registries) if registries.is_empty() => {}
        _ => return Err("Cargo metadata says the package can be published".into()),
    }
    validate_dependencies(json_field(package, "dependencies")?)?;
    let features = json_object(json_field(package, "features")?, "features")?;
    if !features.is_empty() {
        return Err("Cargo metadata reports feature declarations".into());
    }
    let package_root = canonical_manifest
        .parent()
        .ok_or_else(|| "selected manifest has no package directory".to_string())?;
    let source_root = package_root.join("src");
    source_root_boundary(&source_root)
        .map_err(|error| format!("Cargo package source root rejected: {error:?}"))?;
    let parser_dependency = json_array(json_field(package, "dependencies")?, "dependencies")?
        .iter()
        .any(|dependency| {
            json_field(dependency, "name").and_then(|value| json_string(value, "dependency.name"))
                == Ok("httparse")
        });
    let server_source = fs::symlink_metadata(source_root.join("server.rs"))
        .map(|metadata| metadata.file_type().is_file())
        .or_else(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                Ok(false)
            } else {
                Err(format!("could not inspect server source: {error}"))
            }
        })?;
    if server_source != parser_dependency {
        return Err("Cargo parser dependency and server source stage must be paired".into());
    }
    let expected_lib = fs::canonicalize(source_root.join("lib.rs"))
        .map_err(|error| format!("could not canonicalize library source: {error}"))?;
    let mut library_targets = 0;
    for target in json_array(json_field(package, "targets")?, "targets")? {
        let kinds = json_array(json_field(target, "kind")?, "target.kind")?;
        let kinds = kinds
            .iter()
            .map(|kind| json_string(kind, "target kind"))
            .collect::<Result<Vec<_>, _>>()?;
        if kinds.contains(&"custom-build") {
            return Err("Cargo metadata reports a build script".into());
        }
        if kinds.contains(&"lib") {
            library_targets += 1;
            let source = json_string(json_field(target, "src_path")?, "target.src_path")?;
            let source = fs::canonicalize(source)
                .map_err(|error| format!("could not canonicalize target source: {error}"))?;
            if source != expected_lib {
                return Err("Cargo metadata points the library outside `src/lib.rs`".into());
            }
        } else if kinds != ["test"] {
            return Err(format!(
                "Cargo metadata reports unexpected target kinds {kinds:?}"
            ));
        }
    }
    if library_targets != 1 {
        return Err(format!(
            "Cargo metadata reports {library_targets} library targets"
        ));
    }
    Ok(())
}

// Shared by the Cargo-backed gate and direct hostile-metadata regression tests.
pub(crate) fn validate_dependencies(value: &JsonValue) -> Result<(), String> {
    let dependencies = json_array(value, "dependencies")?;
    if !dependencies.is_empty() {
        let mut names = dependencies
            .iter()
            .map(|dependency| json_string(json_field(dependency, "name")?, "dependency.name"))
            .collect::<Result<Vec<_>, _>>()?;
        names.sort_unstable();
        if names != ["serde", "serde_json"] && names != ["httparse", "serde", "serde_json"] {
            return Err("Cargo metadata reports unexpected dependency declarations".into());
        }
        for dependency in dependencies {
            let name = json_string(json_field(dependency, "name")?, "dependency.name")?;
            let expected_features: &[&str] = if name == "serde" { &["derive"] } else { &[] };
            expect_json_string(
                dependency,
                "req",
                if name == "httparse" { "=1.10.1" } else { "^1" },
            )?;
            expect_json_string(
                dependency,
                "source",
                "registry+https://github.com/rust-lang/crates.io-index",
            )?;
            for field in ["kind", "target", "rename", "registry"] {
                if !matches!(json_field(dependency, field)?, JsonValue::Null) {
                    return Err(format!("dependency `{name}` has unexpected `{field}`"));
                }
            }
            if let Ok(value) = json_field(dependency, "path") {
                if !matches!(value, JsonValue::Null) {
                    return Err(format!("dependency `{name}` has unexpected `path`"));
                }
            }
            if !matches!(json_field(dependency, "optional")?, JsonValue::Bool(false))
                || !matches!(
                    json_field(dependency, "uses_default_features")?,
                    JsonValue::Bool(defaults) if *defaults == (name != "httparse")
                )
            {
                return Err(format!("dependency `{name}` has unexpected flags"));
            }
            let features = json_array(json_field(dependency, "features")?, "dependency.features")?;
            if features.len() != expected_features.len()
                || features
                    .iter()
                    .zip(expected_features)
                    .any(|(actual, expected)| {
                        json_string(actual, "dependency feature") != Ok(*expected)
                    })
            {
                return Err(format!("dependency `{name}` has unexpected features"));
            }
        }
    }
    Ok(())
}

pub(crate) struct ManifestFixture {
    pub(crate) root: PathBuf,
    pub(crate) manifest: PathBuf,
}

impl ManifestFixture {
    pub(crate) fn new(declaration: &str) -> Self {
        Self::with_publish_line("publish = false", declaration)
    }

    pub(crate) fn with_publish_line(publish_line: &str, declaration: &str) -> Self {
        let sequence = FIXTURE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "sovereign-playground-boundary-{}-{sequence}",
            std::process::id()
        ));
        let dependency = root.join("dep");
        fs::create_dir_all(root.join("src")).expect("fixture source directory must be created");
        fs::create_dir_all(dependency.join("src"))
            .expect("fixture dependency source directory must be created");
        fs::write(root.join("src/lib.rs"), "").expect("fixture package source must be written");
        fs::write(dependency.join("src/lib.rs"), "")
            .expect("fixture dependency source must be written");
        fs::write(
            dependency.join("Cargo.toml"),
            "[package]\nname = \"forbidden\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .expect("fixture dependency manifest must be written");
        let manifest = root.join("Cargo.toml");
        fs::write(
            &manifest,
            format!(
                "[package]\nname = \"boundary-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\nrust-version = \"1.97\"\nlicense = \"Apache-2.0\"\nrepository = \"https://github.com/IcantFind-a-username/Sovereign-Founder-OS\"\n{publish_line}\n\n{declaration}\n"
            ),
        )
        .expect("fixture package manifest must be written");
        Self { root, manifest }
    }
}

impl Drop for ManifestFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

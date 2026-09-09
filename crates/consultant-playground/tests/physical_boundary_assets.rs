//! Closed compile-time asset mapping and its physical six-file inventory.
//! This gate does not validate website behavior or JavaScript capabilities.

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
    source_boundary, SourceBoundaryKind, ASSETS_PRODUCTION, EXPECTED_LIB_ASSETS_SHAPE,
    EXPECTED_LIB_HTTP_SHAPE,
};
use manifest::{crate_root, manifest_boundary};
use source_closure::{
    assert_source_rejection, asset_source_fixture, catalog_source_fixture, source_closure_boundary,
    SourceClosureError, ASSET_FILES,
};
use source_root::SourceRootError;

#[test]
fn asset_gate_accepts_only_closed_compile_time_mapping() {
    let fixture = asset_source_fixture();
    assert!(fixture.manifest.is_file(), "disposable fixture manifest");
    source_closure_boundary(&fixture.root.join("src")).expect("complete valid new asset closure");
    source_boundary(Path::new("assets.rs"), &asset_source())
        .expect("complete valid new asset mapping");
    source_closure_boundary(&crate_root().join("src")).expect("real production inventory");
    manifest_boundary(
        &crate_root().join("Cargo.toml"),
        "sovereign-consultant-playground",
    )
    .expect("existing manifest boundary");
    for stage in [(false, false), (true, false), (true, true)] {
        let previous = catalog_source_fixture(stage.0, stage.0, stage.1, stage.1);
        assert!(!previous.root.join("assets").exists());
        assert_eq!(source_closure_boundary(&previous.root.join("src")), Ok(()));
    }
    // Asset bytes and comments may change without a second expected website.
    for name in ASSET_FILES {
        fs::write(
            fixture.root.join("assets").join(name),
            b"different placeholder",
        )
        .expect("replace placeholder");
    }
    let commented = format!("// include_bytes!(\"/not-code\")\n{}", asset_source());
    fs::write(fixture.root.join("src/assets.rs"), &commented).expect("commented mapping");
    assert_eq!(source_closure_boundary(&fixture.root.join("src")), Ok(()));

    for (name, from, to) in [
        (
            "wrong import",
            "crate::http::AssetRoute",
            "crate::domain::AssetRoute",
        ),
        ("public struct", "pub(crate) struct", "pub struct"),
        (
            "extra field",
            "pub(crate) bytes:",
            "pub(crate) path: &'static str, pub(crate) bytes:",
        ),
        ("public field", "pub(crate) bytes:", "pub bytes:"),
        (
            "owned content type",
            "content_type: &'static str",
            "content_type: String",
        ),
        ("owned bytes", "bytes: &'static [u8]", "bytes: Vec<u8>"),
        ("public function", "pub(crate) fn asset", "pub fn asset"),
        (
            "free path parameter",
            "route: AssetRoute",
            "route: AssetRoute, path: &str",
        ),
        (
            "free URL parameter",
            "route: AssetRoute",
            "route: AssetRoute, url: &str",
        ),
        ("wrong route input", "route: AssetRoute", "route: String"),
        ("non exhaustive wildcard", "AssetRoute::Favicon =>", "_ =>"),
        ("wrong route", "AssetRoute::Index =>", "AssetRoute::App =>"),
        (
            "combined arm",
            "AssetRoute::Index =>",
            "AssetRoute::Index | AssetRoute::App =>",
        ),
        (
            "guarded arm",
            "AssetRoute::Index =>",
            "AssetRoute::Index if false =>",
        ),
        (
            "runtime IO",
            "match route {",
            "std::fs::read(\"example\").unwrap(); match route {",
        ),
        (
            "network side effect",
            "match route {",
            "std::net::TcpStream::connect(\"localhost:80\").unwrap(); match route {",
        ),
        (
            "process side effect",
            "match route {",
            "std::process::Command::new(\"sh\").spawn().unwrap(); match route {",
        ),
        (
            "cfg hidden effect",
            "match route {",
            "#[cfg(any())] { std::fs::read(\"hidden\").unwrap(); } match route {",
        ),
        (
            "helper function",
            "use crate::http",
            "fn extra() {} use crate::http",
        ),
        (
            "unapproved derive",
            "pub(crate) struct",
            "#[derive(Clone)] pub(crate) struct",
        ),
    ] {
        assert_asset_mutation(name, from, to, SourceBoundaryKind::AssetsProductionShape);
    }
    for (route, file, content_type) in [
        ("Index", "index.html", "text/html; charset=utf-8"),
        ("Styles", "styles.css", "text/css; charset=utf-8"),
        ("I18n", "i18n.js", "text/javascript; charset=utf-8"),
        ("App", "app.js", "text/javascript; charset=utf-8"),
        (
            "ConsultantUi",
            "consultant-ui.js",
            "text/javascript; charset=utf-8",
        ),
        ("Favicon", "favicon.svg", "image/svg+xml"),
    ] {
        let arm = format!(
            "        AssetRoute::{route} => EmbeddedAsset {{\n            content_type: \"{content_type}\",\n            bytes: include_bytes!(\"../assets/{file}\"),\n        }},\n"
        );
        for replacement in [
            String::new(),
            format!("{arm}{arm}"),
            arm.replace(content_type, "text/plain"),
        ] {
            assert_asset_mutation(
                route,
                &arm,
                &replacement,
                SourceBoundaryKind::AssetsProductionShape,
            );
        }
        let literal = format!("\"../assets/{file}\"");
        for replacement in ["\"../assets/unknown\"", "\"../assets/app.js\""] {
            if replacement != literal {
                assert_asset_mutation(
                    file,
                    &literal,
                    replacement,
                    SourceBoundaryKind::AssetsProductionShape,
                );
            }
        }
    }
}

#[test]
fn asset_gate_rejects_include_path_file_and_symlink_escapes() {
    let fixture = asset_source_fixture();
    assert_eq!(source_closure_boundary(&fixture.root.join("src")), Ok(()));
    for (name, from, to) in [
        ("include Rust", "include_bytes!", "include!"),
        ("include string", "include_bytes!", "include_str!"),
        (
            "concat path",
            "\"../assets/index.html\"",
            "concat!(\"../assets/\", \"index.html\")",
        ),
        (
            "env path",
            "\"../assets/index.html\"",
            "env!(\"ASSET_PATH\")",
        ),
        ("free path", "\"../assets/index.html\"", "path"),
        (
            "absolute path",
            "\"../assets/index.html\"",
            "\"/tmp/index.html\"",
        ),
        (
            "Windows absolute path",
            "\"../assets/index.html\"",
            "r\"C:\\assets\\index.html\"",
        ),
        ("parent escape", "../assets/index.html", "../../index.html"),
        (
            "parent normalization",
            "../assets/index.html",
            "../assets/../index.html",
        ),
        (
            "nested path",
            "../assets/index.html",
            "../assets/nested/index.html",
        ),
        (
            "dot normalization",
            "../assets/index.html",
            "../assets/./index.html",
        ),
        (
            "URL",
            "../assets/index.html",
            "https://example.test/index.html",
        ),
        (
            "raw string alias",
            "\"../assets/index.html\"",
            "r\"../assets/index.html\"",
        ),
    ] {
        assert_asset_mutation(name, from, to, SourceBoundaryKind::AssetsProductionShape);
    }
    for missing in ASSET_FILES {
        let fixture = asset_source_fixture();
        fs::remove_file(fixture.root.join("assets").join(missing)).expect("remove asset");
        assert_eq!(
            source_closure_boundary(&fixture.root.join("src")),
            Err(SourceClosureError::AssetInventory),
            "missing {missing}"
        );
    }
    for extra in [
        "extra.js",
        "tsconfig.json",
        "app.test.js",
        "nested/index.html",
    ] {
        let fixture = asset_source_fixture();
        let path = fixture.root.join("assets").join(extra);
        fs::create_dir_all(path.parent().expect("extra parent")).expect("extra parent directory");
        fs::write(path, b"placeholder").expect("extra asset");
        assert_eq!(
            source_closure_boundary(&fixture.root.join("src")),
            Err(SourceClosureError::AssetInventory),
            "extra {extra}"
        );
    }
    for name in ASSET_FILES {
        let fixture = asset_source_fixture();
        let path = fixture.root.join("assets").join(name);
        fs::remove_file(&path).expect("remove ordinary asset");
        fs::create_dir(&path).expect("directory masquerading as asset");
        assert_eq!(
            source_closure_boundary(&fixture.root.join("src")),
            Err(SourceClosureError::AssetInventory),
            "directory {name}"
        );
    }
    for root_as_file in [false, true] {
        let fixture = asset_source_fixture();
        let root = fixture.root.join("assets");
        fs::remove_dir_all(&root).expect("remove asset root");
        if root_as_file {
            fs::write(root, b"placeholder").expect("root as file");
        }
        let expected = if root_as_file {
            SourceRootError::RootNotDirectory
        } else {
            SourceRootError::MetadataUnreadable
        };
        assert_eq!(
            source_closure_boundary(&fixture.root.join("src")),
            Err(SourceClosureError::AssetRoot(expected))
        );
    }
    #[cfg(any(unix, windows))]
    assert_symlink_rejections();
}

#[test]
fn asset_gate_preserves_source_module_and_wrapper_boundaries() {
    let fixture = asset_source_fixture();
    assert_eq!(source_closure_boundary(&fixture.root.join("src")), Ok(()));
    for (from, to, expected) in [
        (
            "#[cfg(test)]",
            "#[cfg(any(test, unix))]",
            SourceBoundaryKind::AssetsTestModuleShape,
        ),
        (
            "#[cfg(test)]",
            "#[cfg(all(test, unix))]",
            SourceBoundaryKind::AssetsTestModuleShape,
        ),
        (
            "mod tests {}",
            "pub mod tests {}",
            SourceBoundaryKind::AssetsTestModuleShape,
        ),
        (
            "mod tests {}",
            "mod tests;",
            SourceBoundaryKind::AssetsTestModuleShape,
        ),
        (
            "mod tests {}",
            "mod other {}",
            SourceBoundaryKind::AssetsTestModuleShape,
        ),
        (
            "mod tests {}",
            "mod tests {} fn escaped() {}",
            SourceBoundaryKind::AssetsTestModuleShape,
        ),
        (
            "#[cfg(test)] mod tests {}",
            "",
            SourceBoundaryKind::AssetsTestModuleShape,
        ),
        (
            "mod tests {}",
            "mod tests {} #[cfg(test)] mod tests {}",
            SourceBoundaryKind::AssetsTestModuleShape,
        ),
        (
            "mod tests {}",
            "mod tests {",
            SourceBoundaryKind::AssetsTestModuleShape,
        ),
        (
            "use crate::http",
            "#[path=\"../outside.rs\"] mod escaped; use crate::http",
            SourceBoundaryKind::PathAttribute,
        ),
        (
            "use crate::http",
            "#[cfg_attr(any(), path=\"../outside.rs\")] mod escaped; use crate::http",
            SourceBoundaryKind::PathAttribute,
        ),
        (
            "use crate::http",
            "mod escaped; use crate::http",
            SourceBoundaryKind::AssetsProductionShape,
        ),
    ] {
        assert_asset_mutation(to, from, to, expected);
    }
    let test_only = asset_source().replace(
        "mod tests {}",
        "mod tests { #[test] fn harmless() { assert_eq!(2, 2); } }",
    );
    source_boundary(Path::new("assets.rs"), &test_only)
        .expect("ordinary inline unit tests permitted");
    for (file, expected) in [
        ("assets.rs", SourceClosureError::LibInventoryPair),
        ("http.rs", SourceClosureError::Inventory),
        ("catalog.rs", SourceClosureError::Inventory),
        ("domain.rs", SourceClosureError::Inventory),
        ("lib.rs", SourceClosureError::Inventory),
    ] {
        let fixture = asset_source_fixture();
        fs::remove_file(fixture.root.join("src").join(file)).expect("remove required source");
        assert_eq!(
            source_closure_boundary(&fixture.root.join("src")),
            Err(expected),
            "missing {file}"
        );
    }
    for file in ["unknown.rs", "nested/assets.rs", "assets/mod.rs"] {
        let fixture = asset_source_fixture();
        let extra = fixture.root.join("src").join(file);
        fs::create_dir_all(extra.parent().expect("extra parent")).expect("extra directory");
        fs::write(extra, "").expect("extra source");
        assert_eq!(
            source_closure_boundary(&fixture.root.join("src")),
            Err(SourceClosureError::Inventory),
            "extra {file}"
        );
    }
    for (files, lib, expected) in [
        (
            true,
            EXPECTED_LIB_HTTP_SHAPE.to_owned(),
            SourceClosureError::LibInventoryPair,
        ),
        (
            false,
            EXPECTED_LIB_ASSETS_SHAPE.to_owned(),
            SourceClosureError::LibInventoryPair,
        ),
        (
            true,
            EXPECTED_LIB_ASSETS_SHAPE.replace("mod assets;", "pub mod assets;"),
            SourceClosureError::Source(SourceBoundaryKind::LibItemShape),
        ),
        (
            true,
            EXPECTED_LIB_ASSETS_SHAPE.replace("mod assets;", "mod assets {}"),
            SourceClosureError::Source(SourceBoundaryKind::LibItemShape),
        ),
        (
            true,
            EXPECTED_LIB_ASSETS_SHAPE
                .replace("mod assets;", "#[path=\"../assets.rs\"] mod assets;"),
            SourceClosureError::Source(SourceBoundaryKind::PathAttribute),
        ),
        (
            true,
            EXPECTED_LIB_ASSETS_SHAPE.replace("mod assets;", "mod assets; mod other;"),
            SourceClosureError::Source(SourceBoundaryKind::LibItemShape),
        ),
    ] {
        let fixture = if files {
            asset_source_fixture()
        } else {
            catalog_source_fixture(true, true, true, true)
        };
        fs::write(fixture.root.join("src/lib.rs"), lib).expect("mutated lib");
        assert_eq!(
            source_closure_boundary(&fixture.root.join("src")),
            Err(expected)
        );
    }
    // The include exception belongs exclusively to assets.rs.
    for (file, expected) in [
        ("http.rs", SourceBoundaryKind::HttpProductionShape),
        ("domain.rs", SourceBoundaryKind::DomainProductionShape),
        ("catalog.rs", SourceBoundaryKind::CatalogProductionShape),
        ("lib.rs", SourceBoundaryKind::LibItemShape),
        ("other.rs", SourceBoundaryKind::UnexpectedSourceFile),
    ] {
        assert_source_rejection(file, Path::new(file), &asset_source(), expected);
    }
}

fn asset_source() -> String {
    format!("{ASSETS_PRODUCTION}\n#[cfg(test)] mod tests {{}}")
}

fn assert_asset_mutation(name: &str, from: &str, to: &str, expected: SourceBoundaryKind) {
    let original = asset_source();
    assert!(original.contains(from), "mutation marker: {name}");
    let source = original.replacen(from, to, 1);
    assert_ne!(source, original, "mutation must change fixture: {name}");
    assert_source_rejection(name, Path::new("assets.rs"), &source, expected);
    let fixture = asset_source_fixture();
    fs::write(fixture.root.join("src/assets.rs"), source).expect("mutated asset source");
    assert_eq!(
        source_closure_boundary(&fixture.root.join("src")),
        Err(SourceClosureError::Source(expected)),
        "real inventory path: {name}"
    );
}

#[cfg(any(unix, windows))]
fn assert_symlink_rejections() {
    for root in ["src", "assets"] {
        let fixture = asset_source_fixture();
        let linked = match symlink_fixture::SymlinkedSourceFixture::new() {
            Ok(linked) => linked,
            #[cfg(windows)]
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
                eprintln!("directory symlinks unavailable without Windows developer privilege");
                return;
            }
            Err(error) => panic!("symlink fixture: {error}"),
        };
        assert!(linked.base.root.exists());
        fs::remove_dir_all(fixture.root.join(root)).expect("remove root for symlink");
        fs::rename(linked.source_link, fixture.root.join(root)).expect("reuse directory symlink");
        let expected = if root == "src" {
            SourceClosureError::Root(SourceRootError::RootSymlink)
        } else {
            SourceClosureError::AssetRoot(SourceRootError::RootSymlink)
        };
        assert_eq!(
            source_closure_boundary(&fixture.root.join("src")),
            Err(expected)
        );
    }
    for file in ASSET_FILES {
        for outside in [false, true] {
            let fixture = asset_source_fixture();
            let path = fixture.root.join("assets").join(file);
            let target = if outside {
                fixture.root.join("outside-placeholder")
            } else {
                fixture.root.join("assets/app.js")
            };
            fs::remove_file(&path).expect("replace asset with symlink");
            if outside {
                fs::write(&target, b"placeholder").expect("outside placeholder");
            }
            #[cfg(unix)]
            std::os::unix::fs::symlink(&target, &path).expect("asset file symlink");
            #[cfg(windows)]
            std::os::windows::fs::symlink_file(&target, &path).expect("asset file symlink");
            assert_eq!(
                source_closure_boundary(&fixture.root.join("src")),
                Err(SourceClosureError::AssetInventory),
                "symlink {file} outside={outside}"
            );
        }
    }
    for file in ["assets.rs", "catalog.rs", "domain.rs", "http.rs", "lib.rs"] {
        let fixture = asset_source_fixture();
        let path = fixture.root.join("src").join(file);
        let outside = fixture.root.join("outside.rs");
        fs::rename(&path, &outside).expect("move source outside");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&outside, &path).expect("source file symlink");
        #[cfg(windows)]
        std::os::windows::fs::symlink_file(&outside, &path).expect("source file symlink");
        let panic = std::panic::catch_unwind(|| source_closure_boundary(&fixture.root.join("src")))
            .expect_err("existing scanner must reject source symlinks");
        let message = panic
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| panic.downcast_ref::<&str>().copied())
            .expect("scanner panic message");
        assert!(
            message.contains("production sources must not be symlinks"),
            "{file}: {message}"
        );
    }
}

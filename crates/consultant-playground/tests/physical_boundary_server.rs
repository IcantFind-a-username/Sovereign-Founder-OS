//! Loopback transport source closure and exact stage-paired parser dependency.

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
    source_boundary, SourceBoundaryKind, ACTION_DOMAIN_ADDITIONS, ASSETS_PRODUCTION,
    EXPECTED_DOMAIN_PRODUCTION, EXPECTED_LIB_ASSETS_SHAPE, EXPECTED_LIB_SERVER_SHAPE,
    SERVER_PRODUCTION,
};
use json::{JsonParser, JsonValue};
use manifest::{
    cargo_metadata, crate_root, manifest_boundary, validate_dependencies, ManifestFixture,
};
use source_closure::{
    assert_source_rejection, asset_source_fixture, catalog_source_fixture, source_closure_boundary,
    SourceClosureError, ASSET_FILES,
};
use source_root::SourceRootError;

const SERVER_DEPENDENCIES: &str = "\n[dependencies]\nserde = { version = \"1\", features = [\"derive\"] }\nserde_json = \"1\"\nhttparse = { version = \"=1.10.1\", default-features = false }\n";

#[test]
fn server_gate_accepts_old_and_pinned_next_closure() {
    source_closure_boundary(&crate_root().join("src")).expect("accepted actual source closure");
    manifest_boundary(
        &crate_root().join("Cargo.toml"),
        "sovereign-consultant-playground",
    )
    .expect("accepted actual manifest");
    for (catalog, http) in [(false, false), (true, false), (true, true)] {
        let fixture = catalog_source_fixture(catalog, catalog, http, http);
        assert_eq!(source_closure_boundary(&fixture.root.join("src")), Ok(()));
    }
    for additions in ["", ACTION_DOMAIN_ADDITIONS] {
        let fixture = catalog_source_fixture(false, false, false, false);
        fs::write(
            fixture.root.join("src/domain.rs"),
            format!("{EXPECTED_DOMAIN_PRODUCTION}\n{additions}\n#[cfg(test)] mod tests {{}}"),
        )
        .expect("historical domain stage");
        assert_eq!(source_closure_boundary(&fixture.root.join("src")), Ok(()));
    }
    let old = asset_source_fixture();
    assert_eq!(source_closure_boundary(&old.root.join("src")), Ok(()));
    let fixture = server_source_fixture();
    assert_eq!(
        source_closure_boundary(&fixture.root.join("src")),
        Ok(()),
        "full valid next stage must be accepted"
    );
    source_boundary(Path::new("server.rs"), &server_source()).expect("full server fixture");
    manifest_boundary(&fixture.manifest, "boundary-fixture").expect("full paired next manifest");
    for (file, expected) in [
        ("src/server.rs", SourceClosureError::LibInventoryPair),
        ("src/lib.rs", SourceClosureError::Inventory),
        ("src/assets.rs", SourceClosureError::Inventory),
        ("src/http.rs", SourceClosureError::Inventory),
        ("src/catalog.rs", SourceClosureError::Inventory),
        ("src/domain.rs", SourceClosureError::Inventory),
        ("assets/app.js", SourceClosureError::AssetInventory),
    ] {
        let fixture = server_source_fixture();
        fs::remove_file(fixture.root.join(file)).expect("remove required stage file");
        assert_eq!(
            source_closure_boundary(&fixture.root.join("src")),
            Err(expected),
            "{file}"
        );
    }
    for (server, lib) in [
        (true, EXPECTED_LIB_ASSETS_SHAPE),
        (false, EXPECTED_LIB_SERVER_SHAPE),
    ] {
        let fixture = if server {
            server_source_fixture()
        } else {
            asset_source_fixture()
        };
        fs::write(fixture.root.join("src/lib.rs"), lib).expect("mismatched lib");
        assert_eq!(
            source_closure_boundary(&fixture.root.join("src")),
            Err(SourceClosureError::LibInventoryPair)
        );
    }

    assert_source_rejection(
        "unexpected source remains rejected",
        Path::new("other.rs"),
        "",
        boundary::SourceBoundaryKind::UnexpectedSourceFile,
    );
}

fn server_source() -> String {
    format!("{SERVER_PRODUCTION}\n#[cfg(test)]\nmod tests {{}}\n")
}

fn server_source_fixture() -> ManifestFixture {
    let fixture = asset_source_fixture();
    fs::write(fixture.root.join("src/lib.rs"), EXPECTED_LIB_SERVER_SHAPE)
        .expect("fixture server lib");
    fs::write(fixture.root.join("src/server.rs"), server_source()).expect("fixture server");
    let manifest = fs::read_to_string(&fixture.manifest).expect("fixture manifest");
    fs::write(
        &fixture.manifest,
        format!("{manifest}{SERVER_DEPENDENCIES}"),
    )
    .expect("fixture pinned parser dependency");
    fixture
}

#[test]
fn server_gate_rejects_transport_authority_and_resource_widening() {
    for (name, from, to) in [
        ("nonliteral bind", "TcpListener::bind((\"127.0.0.1\", port))", "TcpListener::bind((host, port))"),
        ("wildcard bind", "\"127.0.0.1\", port", "\"0.0.0.0\", port"),
        ("IPv6 bind", "\"127.0.0.1\", port", "\"::1\", port"),
        ("second bind", "let address =", "let _other = TcpListener::bind((\"127.0.0.1\", port))?; let address ="),
        ("outbound connect", "let address =", "let _outbound = TcpStream::connect((\"example.test\", 80))?; let address ="),
        ("Host derived port", "let length = content_length(parsed.headers)?;", "let bound_port = parsed.headers[0].value.len() as u16; let length = content_length(parsed.headers)?;"),
        ("requested instead of actual port", "let bound_port = address.port();", "let bound_port = port;"),
        ("zero port accepted", "|| bound_port == 0", "|| bound_port == 1"),
        ("public server API", "pub(crate) fn run", "pub fn run"),
        ("backend", "use crate::assets;", "use sovereign_vault::Vault; use crate::assets;"),
        ("body expanded", "MAX_BODY_READ_BYTES: usize = 257", "MAX_BODY_READ_BYTES: usize = 258"),
        ("head expanded", "MAX_HEAD_BYTES: usize = 8192", "MAX_HEAD_BYTES: usize = 8193"),
        ("headers expanded", "MAX_HEADERS: usize = 32", "MAX_HEADERS: usize = 33"),
        ("read deadline expanded", "READ_BUDGET: Duration = Duration::from_secs(5)", "READ_BUDGET: Duration = Duration::from_secs(6)"),
        ("write deadline expanded", "WRITE_BUDGET: Duration = Duration::from_secs(5)", "WRITE_BUDGET: Duration = Duration::from_secs(6)"),
        ("unbounded read", "stream.read(&mut head[length..length + 1])", "stream.read_to_end(&mut Vec::new())"),
        ("head prefetch", "head[length..length + 1]", "head[length..]"),
        ("drain", "let _ = stream.shutdown", "let _ = stream.read_to_end(&mut Vec::new()); let _ = stream.shutdown"),
        ("per-read budget reset", "set_read_timeout(Some(remaining))", "set_read_timeout(Some(READ_BUDGET))"),
        ("per-write budget reset", "set_write_timeout(Some(remaining))", "set_write_timeout(Some(WRITE_BUDGET))"),
        ("per-body budget reset", "read_body(stream, deadline, length", "read_body(stream, Instant::now() + READ_BUDGET, length"),
        ("permissive parser", "parsed.parse(head)", "httparse::ParserConfig::default().allow_spaces_after_header_name_in_responses(true).parse_request(&mut parsed, head)"),
        ("TE accepted", "[\"Transfer-Encoding\", \"Expect\", \"Upgrade\"]", "[\"Ignored\", \"Expect\", \"Upgrade\"]"),
        ("Expect accepted", "[\"Transfer-Encoding\", \"Expect\", \"Upgrade\"]", "[\"Transfer-Encoding\", \"Ignored\", \"Upgrade\"]"),
        ("Upgrade accepted", "[\"Transfer-Encoding\", \"Expect\", \"Upgrade\"]", "[\"Transfer-Encoding\", \"Expect\", \"Ignored\"]"),
        ("Connection upgrade accepted", "eq_ignore_ascii_case(\"upgrade\")", "eq_ignore_ascii_case(\"ignored\")"),
        ("duplicate CL accepted", "if length.is_some()", "if false"),
        ("leading zero accepted", "value.len() > 1 && value[0] == b'0'", "value.len() > 1 && value[0] == b'X'"),
        ("overflow accepted", ".checked_mul(10)", ".wrapping_mul(10)"),
        ("lost duplicate Host", ".map(|header| {", ".filter(|header| !header.name.eq_ignore_ascii_case(\"Host\")).map(|header| {"),
        ("lost duplicate Origin", ".map(|header| {", ".filter(|header| !header.name.eq_ignore_ascii_case(\"Origin\")).map(|header| {"),
        ("lost duplicate Content-Type", ".map(|header| {", ".filter(|header| !header.name.eq_ignore_ascii_case(\"Content-Type\")).map(|header| {"),
        ("normalized target", "let target = parsed.path.ok_or(ReadFailure::BadRequest)?;", "let target = parsed.path.ok_or(ReadFailure::BadRequest)?.trim();"),
        ("normalized method", "let method = parsed.method.ok_or(ReadFailure::BadRequest)?;", "let method = parsed.method.ok_or(ReadFailure::BadRequest)?.trim();"),
        ("arbitrary route", "assets::asset(route)", "assets::asset(crate::http::AssetRoute::Index)"),
        ("file lookup", "let asset = assets::asset(route);", "let _bytes = std::fs::read(target); let asset = assets::asset(route);"),
        ("retry request", "let _ = serve_connection(&mut stream, &handler, bound_port);", "let _ = serve_connection(&mut stream, &handler, bound_port); let _ = serve_connection(&mut stream, &handler, bound_port);"),
        ("session reset", "let _ = serve_connection(&mut stream, &handler, bound_port);", "let handler = PlaygroundHttpHandler::new(); let _ = serve_connection(&mut stream, &handler, bound_port);"),
        ("CORS", "head.push_str(name);", "head.push_str(\"Access-Control-Allow-Origin: *\\r\\n\"); head.push_str(name);"),
        ("Date", "head.push_str(name);", "head.push_str(\"Date: fixed\\r\\n\"); head.push_str(name);"),
        ("Server", "head.push_str(name);", "head.push_str(\"Server: demo\\r\\n\"); head.push_str(name);"),
        ("clock derived state", "let request = HttpRequest {", "let bound_port = std::time::SystemTime::now().elapsed().unwrap().as_secs() as u16; let request = HttpRequest {"),
        ("production test switch", "let listener =", "if std::env::var_os(\"TEST_PORT\").is_some() { return Ok(()); } let listener ="),
        ("thread", "let handler = PlaygroundHttpHandler::new();", "std::thread::spawn(|| {}); let handler = PlaygroundHttpHandler::new();"),
        ("fabricated oversized body", "body: &body[..length]", "body: &[0; MAX_BODY_READ_BYTES]"),
        ("HEAD body written", "if !head_only", "if true"),
        ("HEAD status changed", "response.status,", "if method == \"HEAD\" { 200 } else { response.status },"),
        ("application headers lost", "response.headers,", "TRANSPORT_HEADERS,"),
        ("body length lost", "body.len()", "0"),
        ("keep alive", "Connection: close", "Connection: keep-alive"),
        ("unbounded write", "write_bytes(stream, deadline, body)?", "stream.write_all(body)?"),
        ("EOF accepted", "Ok(0) => return Err(ReadFailure::BadRequest)", "Ok(0) => return Ok(0)"),
        ("bare LF tolerated", "byte == b'\\n' &&", "byte == b'X' &&"),
        ("ASCII relaxed", "!byte.is_ascii()", "false"),
        ("failure rollback", "let status = match result {", "let _handler = PlaygroundHttpHandler::new(); let status = match result {"),
    ] {
        assert_server_mutation(name, from, to, SourceBoundaryKind::ServerProductionShape);
    }
}

#[test]
fn server_gate_preserves_path_wrapper_and_asset_boundaries() {
    for (name, from, to, kind) in [
        (
            "path attribute",
            "use crate::assets;",
            "#[path = \"../outside.rs\"] mod other; use crate::assets;",
            SourceBoundaryKind::PathAttribute,
        ),
        (
            "conditional path",
            "use crate::assets;",
            "#[cfg_attr(test, path = \"../outside.rs\")] mod other; use crate::assets;",
            SourceBoundaryKind::PathAttribute,
        ),
        (
            "wrong cfg",
            "#[cfg(test)]",
            "#[cfg(any(test, unix))]",
            SourceBoundaryKind::ServerTestModuleShape,
        ),
        (
            "cfg attribute wrapper",
            "#[cfg(test)]",
            "#[cfg_attr(test, allow(dead_code))]",
            SourceBoundaryKind::ServerTestModuleShape,
        ),
        (
            "public test module",
            "mod tests {}",
            "pub mod tests {}",
            SourceBoundaryKind::ServerTestModuleShape,
        ),
        (
            "external test module",
            "mod tests {}",
            "mod tests;",
            SourceBoundaryKind::ServerTestModuleShape,
        ),
        (
            "trailing code",
            "mod tests {}",
            "mod tests {} fn escape() {}",
            SourceBoundaryKind::ServerTestModuleShape,
        ),
        (
            "second test module",
            "mod tests {}",
            "mod tests {} #[cfg(test)] mod tests {}",
            SourceBoundaryKind::ServerTestModuleShape,
        ),
        (
            "top level lint",
            "use crate::assets;",
            "#[allow(dead_code)] use crate::assets;",
            SourceBoundaryKind::ServerProductionShape,
        ),
        (
            "include",
            "use crate::assets;",
            "include!(\"../outside.rs\"); use crate::assets;",
            SourceBoundaryKind::ServerProductionShape,
        ),
    ] {
        assert_server_mutation(name, from, to, kind);
    }
    for source in [
        format!("// harmless comment\n{}", server_source()),
        server_source().replace("mod tests {}", "mod tests { #[test] fn local_test() {} }"),
    ] {
        source_boundary(Path::new("server.rs"), &source)
            .expect("comments and exact test-only body");
    }
    for (from, to, expected) in [
        (
            "mod server;",
            "pub mod server;",
            SourceBoundaryKind::LibItemShape,
        ),
        (
            "mod server;",
            "#[path=\"../server.rs\"] mod server;",
            SourceBoundaryKind::PathAttribute,
        ),
        (
            "server::run(port)",
            "server::run(0)",
            SourceBoundaryKind::LibItemShape,
        ),
        (
            "mod server;",
            "mod server; pub use server::*;",
            SourceBoundaryKind::LibItemShape,
        ),
    ] {
        let fixture = server_source_fixture();
        fs::write(
            fixture.root.join("src/lib.rs"),
            EXPECTED_LIB_SERVER_SHAPE.replace(from, to),
        )
        .expect("mutated wrapper");
        assert_eq!(
            source_closure_boundary(&fixture.root.join("src")),
            Err(SourceClosureError::Source(expected))
        );
    }
    for path in ["other.rs", "other/nested.rs", "server/extra.rs"] {
        let fixture = server_source_fixture();
        let path = fixture.root.join("src").join(path);
        fs::create_dir_all(path.parent().expect("source parent")).expect("source directory");
        fs::write(&path, "").expect("extra source");
        assert_eq!(
            source_closure_boundary(&fixture.root.join("src")),
            Err(SourceClosureError::Inventory)
        );
    }
    for file in ASSET_FILES {
        for directory in [false, true] {
            let fixture = server_source_fixture();
            let path = fixture.root.join("assets").join(file);
            fs::remove_file(&path).expect("remove asset");
            if directory {
                fs::create_dir(&path).expect("directory masquerading as asset");
            }
            assert_eq!(
                source_closure_boundary(&fixture.root.join("src")),
                Err(SourceClosureError::AssetInventory),
                "{file} directory={directory}"
            );
        }
        let fixture = server_source_fixture();
        let path = fixture.root.join("src/assets.rs");
        let escaped = ASSETS_PRODUCTION.replace(
            &format!("../assets/{file}"),
            &format!("../../outside/{file}"),
        );
        assert_ne!(escaped, ASSETS_PRODUCTION);
        fs::write(path, format!("{escaped}\n#[cfg(test)] mod tests {{}}"))
            .expect("escaped asset path");
        assert_eq!(
            source_closure_boundary(&fixture.root.join("src")),
            Err(SourceClosureError::Source(
                SourceBoundaryKind::AssetsProductionShape
            ))
        );
    }
    for (path, directory) in [("extra.js", false), ("nested", true)] {
        let fixture = server_source_fixture();
        let path = fixture.root.join("assets").join(path);
        if directory {
            fs::create_dir(path).expect("extra directory");
        } else {
            fs::write(path, "").expect("extra asset");
        }
        assert_eq!(
            source_closure_boundary(&fixture.root.join("src")),
            Err(SourceClosureError::AssetInventory)
        );
    }
    for root in ["src", "assets"] {
        for missing in [false, true] {
            let fixture = server_source_fixture();
            fs::remove_dir_all(fixture.root.join(root)).expect("remove root");
            if !missing {
                fs::write(fixture.root.join(root), "").expect("root is file");
            }
            let error = if missing {
                SourceRootError::MetadataUnreadable
            } else {
                SourceRootError::RootNotDirectory
            };
            let expected = if root == "src" {
                SourceClosureError::Root(error)
            } else {
                SourceClosureError::AssetRoot(error)
            };
            assert_eq!(
                source_closure_boundary(&fixture.root.join("src")),
                Err(expected)
            );
        }
    }
    #[cfg(any(unix, windows))]
    assert_server_symlinks();
}

fn assert_server_mutation(name: &str, from: &str, to: &str, expected: SourceBoundaryKind) {
    let original = server_source();
    assert!(original.contains(from), "missing mutation marker: {name}");
    let source = original.replacen(from, to, 1);
    assert_ne!(source, original, "mutation must change source: {name}");
    assert_source_rejection(name, Path::new("server.rs"), &source, expected);
    let fixture = server_source_fixture();
    fs::write(fixture.root.join("src/server.rs"), source).expect("mutated server");
    assert_eq!(
        source_closure_boundary(&fixture.root.join("src")),
        Err(SourceClosureError::Source(expected)),
        "real inventory: {name}"
    );
}

#[cfg(any(unix, windows))]
fn assert_server_symlinks() {
    for root in ["src", "assets"] {
        let fixture = server_source_fixture();
        let linked = symlink_fixture::SymlinkedSourceFixture::new()
            .expect("symlink fixture capability required");
        assert!(linked.base.root.exists());
        fs::remove_dir_all(fixture.root.join(root)).expect("remove root for symlink");
        fs::rename(linked.source_link, fixture.root.join(root))
            .expect("reuse symlink root fixture");
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
    for root in ["src", "assets"] {
        let files = if root == "src" {
            [
                "assets.rs",
                "catalog.rs",
                "domain.rs",
                "http.rs",
                "lib.rs",
                "server.rs",
            ]
        } else {
            ASSET_FILES
        };
        for file in files {
            for outside in [false, true] {
                let fixture = server_source_fixture();
                let path = fixture.root.join(root).join(file);
                let target = if outside {
                    fixture.root.join("outside-placeholder")
                } else {
                    fixture
                        .root
                        .join(root)
                        .join(if file == files[0] { files[1] } else { files[0] })
                };
                if outside {
                    fs::rename(&path, &target).expect("move original for symlink");
                } else {
                    fs::remove_file(&path).expect("replace original with in-tree symlink");
                }
                #[cfg(unix)]
                std::os::unix::fs::symlink(&target, &path).expect("file symlink");
                #[cfg(windows)]
                std::os::windows::fs::symlink_file(&target, &path).expect("file symlink");
                if root == "assets" {
                    assert_eq!(
                        source_closure_boundary(&fixture.root.join("src")),
                        Err(SourceClosureError::AssetInventory)
                    );
                } else {
                    let panic = std::panic::catch_unwind(|| {
                        source_closure_boundary(&fixture.root.join("src"))
                    })
                    .expect_err("existing production scanner must reject source symlinks");
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
        }
    }
}

#[test]
fn server_manifest_accepts_only_stage_paired_parser_dependency() {
    const DTO: &str = "[dependencies]\nserde = { version = \"1\", features = [\"derive\"] }\nserde_json = \"1\"\n";
    for declaration in ["", DTO] {
        let fixture = ManifestFixture::new(declaration);
        manifest_boundary(&fixture.manifest, "boundary-fixture").expect("historical declaration");
    }
    for declaration in [
        SERVER_DEPENDENCIES.to_string(),
        SERVER_DEPENDENCIES.replace(
            "httparse = { version = \"=1.10.1\", default-features = false }",
            "[dependencies.httparse]\nversion = \"=1.10.1\"\ndefault-features = false",
        ),
        SERVER_DEPENDENCIES.replace(
            "default-features = false",
            "default-features = false, features = []",
        ),
    ] {
        let fixture = ManifestFixture::new(&declaration);
        fs::write(fixture.root.join("src/server.rs"), server_source()).expect("paired source");
        manifest_boundary(&fixture.manifest, "boundary-fixture").expect("exact pinned metadata");
    }
    for (server, declaration) in [(false, SERVER_DEPENDENCIES), (true, ""), (true, DTO)] {
        let fixture = ManifestFixture::new(declaration);
        if server {
            fs::write(fixture.root.join("src/server.rs"), server_source()).expect("server stage");
        }
        let error =
            manifest_boundary(&fixture.manifest, "boundary-fixture").expect_err("stage mismatch");
        assert!(
            error.contains("must be paired"),
            "stage pairing guard: {error}"
        );
    }
    for (name, declaration, expected) in [
        (
            "parser alone",
            "[dependencies]\nhttparse = { version = \"=1.10.1\", default-features = false }".into(),
            "dependency declarations",
        ),
        (
            "missing serde",
            SERVER_DEPENDENCIES
                .replace("serde = { version = \"1\", features = [\"derive\"] }\n", ""),
            "dependency declarations",
        ),
        (
            "missing serde_json",
            SERVER_DEPENDENCIES.replace("serde_json = \"1\"\n", ""),
            "dependency declarations",
        ),
        (
            "extra dependency",
            format!("{SERVER_DEPENDENCIES}forbidden = {{ path = \"dep\" }}\n"),
            "dependency declarations",
        ),
        (
            "unpinned version",
            SERVER_DEPENDENCIES.replace("=1.10.1", "1.10.1"),
            "req",
        ),
        (
            "wrong version",
            SERVER_DEPENDENCIES.replace("=1.10.1", "=1.10.0"),
            "req",
        ),
        (
            "default true",
            SERVER_DEPENDENCIES.replace("default-features = false", "default-features = true"),
            "flags",
        ),
        (
            "default omitted",
            SERVER_DEPENDENCIES.replace(", default-features = false", ""),
            "flags",
        ),
        (
            "features",
            SERVER_DEPENDENCIES.replace(
                "default-features = false",
                "default-features = false, features = [\"std\"]",
            ),
            "features",
        ),
        (
            "rename",
            SERVER_DEPENDENCIES.replace("httparse = {", "alias = { package = \"httparse\","),
            "rename",
        ),
        (
            "optional",
            SERVER_DEPENDENCIES.replace(
                "default-features = false",
                "default-features = false, optional = true",
            ),
            "flags",
        ),
        (
            "dev kind",
            SERVER_DEPENDENCIES.replace("httparse =", "[dev-dependencies]\nhttparse ="),
            "kind",
        ),
        (
            "build kind",
            SERVER_DEPENDENCIES.replace("httparse =", "[build-dependencies]\nhttparse ="),
            "kind",
        ),
        (
            "target",
            SERVER_DEPENDENCIES.replace(
                "httparse =",
                "[target.'cfg(unix)'.dependencies]\nhttparse =",
            ),
            "target",
        ),
        (
            "target dev",
            SERVER_DEPENDENCIES.replace(
                "httparse =",
                "[target.'cfg(unix)'.dev-dependencies]\nhttparse =",
            ),
            "kind",
        ),
        (
            "target build",
            SERVER_DEPENDENCIES.replace(
                "httparse =",
                "[target.'cfg(unix)'.build-dependencies]\nhttparse =",
            ),
            "kind",
        ),
        (
            "path source",
            SERVER_DEPENDENCIES.replace(
                "version = \"=1.10.1\"",
                "path = \"dep\", package = \"forbidden\"",
            ),
            "dependency declarations",
        ),
        (
            "git source",
            SERVER_DEPENDENCIES.replace(
                "version = \"=1.10.1\"",
                "version = \"=1.10.1\", git = \"https://example.invalid/httparse\"",
            ),
            "source",
        ),
        (
            "feature declaration",
            format!("{SERVER_DEPENDENCIES}\n[features]\nextra = []\n"),
            "feature declarations",
        ),
        (
            "alternate lib",
            format!("{SERVER_DEPENDENCIES}\n[lib]\npath = \"outside.rs\"\n"),
            "outside `src/lib.rs`",
        ),
        (
            "build script",
            format!("build = \"outside.rs\"\n{SERVER_DEPENDENCIES}"),
            "build script",
        ),
        (
            "binary",
            format!("{SERVER_DEPENDENCIES}\n[[bin]]\nname = \"extra\"\npath = \"outside.rs\"\n"),
            "unexpected target kinds",
        ),
    ] {
        let fixture = ManifestFixture::new(&declaration);
        fs::write(fixture.root.join("src/server.rs"), server_source()).expect("server stage");
        fs::write(fixture.root.join("outside.rs"), "fn main() {}").expect("alternate target");
        let error = manifest_boundary(&fixture.manifest, "boundary-fixture").expect_err(name);
        // Offline Cargo can reject an uncached version/source before our gate;
        // the direct metadata matrix below separately proves those field guards.
        assert!(
            error.contains(expected)
                || ((name == "wrong version" || name == "git source")
                    && error.starts_with("Cargo metadata rejected the manifest:")),
            "{name}: {error}"
        );
    }
    let fixture = server_source_fixture();
    let mut dependencies = dependency_metadata(&fixture);
    validate_dependencies(&dependencies).expect("actual Cargo three-dependency tuple");
    let index = match &dependencies {
        JsonValue::Array(items) => items.iter().position(|item| {
            matches!(item, JsonValue::Object(fields) if matches!(fields.get("name"), Some(JsonValue::String(name)) if name == "httparse"))
        }).expect("parser dependency index"),
        _ => panic!("Cargo dependencies array"),
    };
    for (field, raw, expected) in [
        ("name", "\"forbidden\"", "dependency declarations"),
        ("req", "\"^1.10.1\"", "req"),
        ("req", "\"=1.10.0\"", "req"),
        ("source", "null", "source"),
        ("source", "\"git+https://example.invalid\"", "source"),
        ("source", "\"registry+https://example.invalid\"", "source"),
        ("kind", "\"dev\"", "kind"),
        ("kind", "\"build\"", "kind"),
        ("target", "\"cfg(unix)\"", "target"),
        ("rename", "\"alias\"", "rename"),
        ("registry", "\"https://example.invalid\"", "registry"),
        ("path", "\"../outside\"", "path"),
        ("optional", "true", "flags"),
        ("uses_default_features", "true", "flags"),
        ("features", "[\"std\"]", "features"),
        ("features", "[\"std\",\"std\"]", "features"),
        ("features", "[false]", "features"),
    ] {
        let JsonValue::Array(items) = &mut dependencies else {
            panic!("dependencies array")
        };
        let JsonValue::Object(fields) = &mut items[index] else {
            panic!("dependency object")
        };
        let previous = fields.insert(
            field.into(),
            JsonParser::parse(raw.as_bytes()).expect("mutation JSON"),
        );
        let error = validate_dependencies(&dependencies).expect_err("direct mutation rejected");
        assert!(error.contains(expected), "{field}={raw}: {error}");
        let JsonValue::Array(items) = &mut dependencies else {
            panic!("dependencies array")
        };
        let JsonValue::Object(fields) = &mut items[index] else {
            panic!("dependency object")
        };
        if let Some(previous) = previous {
            fields.insert(field.into(), previous);
        } else {
            fields.remove(field);
        }
    }
    for field in [
        "name",
        "req",
        "source",
        "kind",
        "target",
        "rename",
        "registry",
        "path",
        "optional",
        "uses_default_features",
        "features",
    ] {
        for raw in ["missing", "42", "{}", "\"wrong-type-or-value\""] {
            let JsonValue::Array(items) = &mut dependencies else {
                panic!("dependencies array")
            };
            let JsonValue::Object(fields) = &mut items[index] else {
                panic!("dependency object")
            };
            let previous = fields.remove(field);
            if raw != "missing" {
                fields.insert(
                    field.into(),
                    JsonParser::parse(raw.as_bytes()).expect("mutation JSON"),
                );
            }
            let result = validate_dependencies(&dependencies);
            if field == "path" && raw == "missing" {
                result.expect("Cargo may omit registry dependency path");
            } else {
                let error = result.expect_err("field type or required field must reject");
                assert!(
                    error.contains(field)
                        || error.contains("flags")
                        || error.contains("dependency declarations"),
                    "{field}={raw}: {error}"
                );
            }
            let JsonValue::Array(items) = &mut dependencies else {
                panic!("dependencies array")
            };
            let JsonValue::Object(fields) = &mut items[index] else {
                panic!("dependency object")
            };
            fields.remove(field);
            if let Some(previous) = previous {
                fields.insert(field.into(), previous);
            }
        }
    }
    validate_dependencies(&dependencies).expect("restored original metadata remains valid");
}

fn dependency_metadata(fixture: &ManifestFixture) -> JsonValue {
    let JsonValue::Object(mut metadata) =
        cargo_metadata(&fixture.manifest).expect("actual Cargo metadata")
    else {
        panic!("metadata object")
    };
    let Some(JsonValue::Array(mut packages)) = metadata.remove("packages") else {
        panic!("packages array")
    };
    assert_eq!(packages.len(), 1, "single disposable package");
    let JsonValue::Object(mut package) = packages.remove(0) else {
        panic!("package object")
    };
    package
        .remove("dependencies")
        .expect("Cargo dependency metadata")
}

//! Shared source closure boundary and disposable source fixtures.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use crate::boundary::{
    source_boundary, SourceBoundaryKind, ASSETS_PRODUCTION, CATALOG_GUIDANCE_PRODUCTION,
    CATALOG_PRODUCTION, EXPECTED_LIB_ASSETS_SHAPE, EXPECTED_LIB_CATALOG_SHAPE,
    EXPECTED_LIB_HTTP_SHAPE, EXPECTED_LIB_SERVER_SHAPE, EXPECTED_LIB_SHAPE, HTTP_PRODUCTION,
    READ_MODEL_DOMAIN_PRODUCTION, TEACHING_DOMAIN_PRODUCTION,
};
use crate::manifest::ManifestFixture;
use crate::production_sources::production_sources;
use crate::rust_lexer::RustLexer;
use crate::source_root::{source_root_boundary, SourceRootError};

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum SourceClosureError {
    Root(SourceRootError),
    Inventory,
    AssetRoot(SourceRootError),
    AssetInventory,
    LibInventoryPair,
    Source(SourceBoundaryKind),
    Unreadable,
}

// The real inventory and adversarial fixtures use this same boundary. Traversal
// and symlink handling remain owned by production_sources/source_root.
pub(crate) fn source_closure_boundary(source_root: &Path) -> Result<(), SourceClosureError> {
    let actual = production_sources(source_root).map_err(SourceClosureError::Root)?;
    let catalog = actual.contains(&source_root.join("catalog.rs"));
    let http = actual.contains(&source_root.join("http.rs"));
    let assets = actual.contains(&source_root.join("assets.rs"));
    let server = actual.contains(&source_root.join("server.rs"));
    if (http && !catalog) || (assets && !http) || (server && !assets) {
        return Err(SourceClosureError::Inventory);
    }
    let mut expected = BTreeSet::from([source_root.join("domain.rs"), source_root.join("lib.rs")]);
    if catalog {
        expected.insert(source_root.join("catalog.rs"));
    }
    if http {
        expected.insert(source_root.join("http.rs"));
    }
    if assets {
        expected.insert(source_root.join("assets.rs"));
    }
    if server {
        expected.insert(source_root.join("server.rs"));
    }
    if actual != expected {
        return Err(SourceClosureError::Inventory);
    }
    for path in actual {
        let source = fs::read_to_string(&path).map_err(|_| SourceClosureError::Unreadable)?;
        source_boundary(&path, &source).map_err(|error| SourceClosureError::Source(error.kind))?;
        if path == source_root.join("lib.rs") {
            let expected_lib = if server {
                EXPECTED_LIB_SERVER_SHAPE
            } else if assets {
                EXPECTED_LIB_ASSETS_SHAPE
            } else if http {
                EXPECTED_LIB_HTTP_SHAPE
            } else if catalog {
                EXPECTED_LIB_CATALOG_SHAPE
            } else {
                EXPECTED_LIB_SHAPE
            };
            if RustLexer::lex(&source).expect("validated lib must lex")
                != RustLexer::lex(expected_lib).expect("frozen lib must lex")
            {
                return Err(SourceClosureError::LibInventoryPair);
            }
        }
    }
    let root = source_root.parent().ok_or(SourceClosureError::Inventory)?;
    asset_inventory_boundary(&root.join("assets"), assets)
}

// Six ordinary files only: no recursion, content parsing, or runtime product IO.
fn asset_inventory_boundary(root: &Path, required: bool) -> Result<(), SourceClosureError> {
    if !required
        && fs::symlink_metadata(root)
            .is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound)
    {
        return Ok(());
    }
    source_root_boundary(root).map_err(SourceClosureError::AssetRoot)?;
    let mut remaining: BTreeSet<_> = ASSET_FILES
        .into_iter()
        .map(std::ffi::OsString::from)
        .collect();
    for entry in fs::read_dir(root).map_err(|_| SourceClosureError::Unreadable)? {
        let entry = entry.map_err(|_| SourceClosureError::Unreadable)?;
        let file_type = entry
            .file_type()
            .map_err(|_| SourceClosureError::Unreadable)?;
        if !file_type.is_file() || file_type.is_symlink() || !remaining.remove(&entry.file_name()) {
            return Err(SourceClosureError::AssetInventory);
        }
    }
    if !remaining.is_empty() {
        return Err(SourceClosureError::AssetInventory);
    }
    Ok(())
}

pub(crate) fn catalog_source_fixture(
    catalog_file: bool,
    catalog_lib: bool,
    http_file: bool,
    http_lib: bool,
) -> ManifestFixture {
    let fixture = ManifestFixture::new("");
    let source_root = fixture.root.join("src");
    fs::write(
        source_root.join("lib.rs"),
        if http_lib {
            EXPECTED_LIB_HTTP_SHAPE
        } else if catalog_lib {
            EXPECTED_LIB_CATALOG_SHAPE
        } else {
            EXPECTED_LIB_SHAPE
        },
    )
    .expect("fixture lib");
    fs::write(
        source_root.join("domain.rs"),
        format!(
            "{READ_MODEL_DOMAIN_PRODUCTION}\n{}\n#[cfg(test)] mod tests {{}}",
            if http_file {
                TEACHING_DOMAIN_PRODUCTION
            } else {
                ""
            }
        ),
    )
    .expect("fixture domain");
    if catalog_file {
        fs::write(
            source_root.join("catalog.rs"),
            format!(
                "{}\n#[cfg(test)] mod tests {{}}",
                if http_file {
                    CATALOG_GUIDANCE_PRODUCTION
                } else {
                    CATALOG_PRODUCTION
                }
            ),
        )
        .expect("fixture catalog");
    }
    if http_file {
        fs::write(
            source_root.join("http.rs"),
            format!("{HTTP_PRODUCTION}\n#[cfg(test)] mod tests {{}}"),
        )
        .expect("fixture HTTP");
    }
    fixture
}

pub(crate) fn assert_source_rejection(
    name: &str,
    path: &Path,
    source: &str,
    expected: SourceBoundaryKind,
) {
    let error = match source_boundary(path, source) {
        Ok(()) => panic!("source mutation `{name}` was accepted"),
        Err(error) => error,
    };
    assert_eq!(
        error.kind, expected,
        "source mutation `{name}` hit the wrong guard: {error}"
    );
}

pub(crate) const ASSET_FILES: [&str; 6] = [
    "index.html",
    "styles.css",
    "i18n.js",
    "app.js",
    "consultant-ui.js",
    "favicon.svg",
];

pub(crate) fn asset_source_fixture() -> ManifestFixture {
    let fixture = catalog_source_fixture(true, true, true, true);
    fs::write(fixture.root.join("src/lib.rs"), EXPECTED_LIB_ASSETS_SHAPE)
        .expect("fixture assets lib");
    fs::write(
        fixture.root.join("src/assets.rs"),
        format!("{ASSETS_PRODUCTION}\n#[cfg(test)] mod tests {{}}"),
    )
    .expect("fixture assets source");
    fs::create_dir(fixture.root.join("assets")).expect("fixture assets directory");
    for name in ASSET_FILES {
        fs::write(fixture.root.join("assets").join(name), name).expect("placeholder asset");
    }
    fixture
}

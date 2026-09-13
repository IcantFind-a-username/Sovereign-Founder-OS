//! Rules for the exactly-two production FFI entry points (RFC 0005 Program 1A,
//! plan lines 540-546). Used by the source-closure gate in `gate.rs`.

use syn::Attribute;

/// Production entry points: the only functions that may host project-authored
/// `unsafe` blocks in the FFI boundary files.
pub fn admitted_production_entry_function(file: &str, name: &str) -> bool {
    matches!(
        (file, name),
        ("src/engine/process.rs", "bootstrap_crypto_process")
            | ("src/engine/ffi.rs", "open_keyed_hardened")
    )
}

/// The single test-only OpenSSL LOAD_CONFIG negative control.
pub fn admitted_test_only_entry_function(file: &str, name: &str) -> bool {
    matches!(
        (file, name),
        (
            "src/engine/process.rs",
            "load_configuration_from_the_environment"
        )
    )
}

/// `extern "C"` symbols the boundary files may declare.
pub fn admitted_extern_symbol(file: &str, name: &str) -> bool {
    matches!(
        (file, name),
        ("src/engine/process.rs", "OPENSSL_init_crypto") | ("src/engine/ffi.rs", "sqlite3_key_v2")
    )
}

pub fn attr_is_cfg_test(attributes: &[Attribute]) -> bool {
    attributes.iter().any(|attribute| {
        attribute.path().is_ident("cfg")
            && attribute
                .parse_args::<syn::Path>()
                .is_ok_and(|path| path.is_ident("test"))
    })
}

/// Crate-root paths that may not be imported (directly or as a leading segment).
pub const FORBIDDEN_USE_ROOTS: &[&str] = &["openssl_sys"];

/// Identifiers that may not appear as the final segment of a path or as a bare
/// call target outside an admitted entry function body.
pub const FORBIDDEN_FFI_SYMBOLS: &[&str] = &[
    "OPENSSL_init_crypto",
    "sqlite3_key_v2",
    "sqlite3_key",
    "sqlite3_open_v2",
    "sqlite3_close",
    "sqlite3_db_config",
    "sqlite3_enable_load_extension",
    "sqlite3_threadsafe",
    "sqlite3_load_extension",
    "sqlcipher_export",
];

pub fn path_tail_is_forbidden(path: &syn::Path) -> Option<String> {
    path.segments
        .last()
        .map(|segment| segment.ident.to_string())
        .filter(|tail| FORBIDDEN_FFI_SYMBOLS.contains(&tail.as_str()))
}

/// Rejects `use foo::*` smuggling while allowing `use super::*` in test modules.
pub fn use_tree_has_forbidden_glob(tree: &syn::UseTree) -> bool {
    match tree {
        syn::UseTree::Glob(_) => true,
        syn::UseTree::Group(group) => group.items.iter().any(use_tree_has_forbidden_glob),
        syn::UseTree::Path(path) => match path.tree.as_ref() {
            syn::UseTree::Glob(_) => {
                let root = path.ident.to_string();
                root != "super" && root != "crate" && root != "self"
            }
            other => use_tree_has_forbidden_glob(other),
        },
        syn::UseTree::Name(_) | syn::UseTree::Rename(_) => false,
    }
}

pub fn use_tree_forbidden_root(tree: &syn::UseTree) -> Option<String> {
    match tree {
        syn::UseTree::Path(path) => {
            let root = path.ident.to_string();
            if FORBIDDEN_USE_ROOTS.contains(&root.as_str()) {
                return Some(root);
            }
            use_tree_forbidden_root(&path.tree)
        }
        syn::UseTree::Group(group) => group.items.iter().find_map(use_tree_forbidden_root),
        syn::UseTree::Name(_) | syn::UseTree::Rename(_) => None,
        syn::UseTree::Glob(_) => None,
    }
}

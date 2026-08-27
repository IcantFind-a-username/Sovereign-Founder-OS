//! The source-closure gate (v1), applied to this crate for real, plus teeth
//! tests proving each rejection actually fires.
//!
//! The configuration below is the closed surface of this crate today. Growing
//! any list is a reviewed decision:
//!
//! - a new explicit Cargo target must join `ROOTS` in the same change that
//!   declares it in `Cargo.toml`;
//! - the queued FFI item adds exactly `src/engine/ffi.rs` and
//!   `src/engine/process.rs` to `FFI_BOUNDARY_FILES` when it creates them —
//!   nothing else is expected to ever join that list;
//! - the exactly-two-entry-points proof and the five `tests/ui/` compile-fail
//!   fixtures are a separate queued item that tightens this gate once the
//!   engine API exists.

mod gate;

use gate::{run_gate, GateConfig};
use std::path::Path;

const ROOTS: &[&str] = &[
    "build.rs",
    "src/lib.rs",
    "tests/build_gate.rs",
    "tests/ast_gate.rs",
];

/// The two `include!` edges that let the build script's gate logic be the
/// literal code under test (see `build_gate.rs`). Any other include is a
/// violation.
const ADMITTED_INCLUDES: &[(&str, &str)] = &[
    ("build.rs", "build_gate.rs"),
    ("tests/build_gate.rs", "../build_gate.rs"),
];

/// Empty until the queued FFI item lands `src/engine/ffi.rs` and
/// `src/engine/process.rs`; the whole crate is unsafe-free today and
/// `src/lib.rs` additionally carries `#![forbid(unsafe_code)]`.
const FFI_BOUNDARY_FILES: &[&str] = &[];

const ALLOWED_MACROS: &[&str] = &[
    "assert",
    "assert_eq",
    "assert_ne",
    "panic",
    "println",
    "eprintln",
    "format",
    "vec",
    "matches",
    "write",
    "writeln",
    "concat",
    "stringify",
    "env",
    "line",
    "file",
    "column",
    "todo",
    "unreachable",
    "unimplemented",
    "compile_error",
];

const ALLOWED_ATTRIBUTES: &[&str] = &[
    "cfg",
    "cfg_attr",
    "allow",
    "warn",
    "deny",
    "forbid",
    "doc",
    "test",
    "ignore",
    "should_panic",
    "derive",
    "must_use",
    "inline",
    "cold",
    "non_exhaustive",
    "track_caller",
];

const ALLOWED_DERIVES: &[&str] = &[
    "Debug",
    "Clone",
    "Copy",
    "PartialEq",
    "Eq",
    "PartialOrd",
    "Ord",
    "Hash",
    "Default",
];

fn real_config() -> GateConfig<'static> {
    GateConfig {
        roots: ROOTS,
        admitted_includes: ADMITTED_INCLUDES,
        ffi_boundary_files: FFI_BOUNDARY_FILES,
        allowed_macros: ALLOWED_MACROS,
        allowed_attributes: ALLOWED_ATTRIBUTES,
        allowed_derives: ALLOWED_DERIVES,
    }
}

#[test]
fn recursive_syn_source_closure_is_complete_and_ffi_boundary_is_exact() {
    let outcome = run_gate(Path::new(env!("CARGO_MANIFEST_DIR")), &real_config());
    assert_eq!(
        outcome.violations,
        Vec::<String>::new(),
        "the crate no longer passes its own source-closure gate"
    );
    let closure: Vec<&str> = outcome.closure.iter().map(String::as_str).collect();
    assert_eq!(
        closure,
        vec![
            "build.rs",
            "build_gate.rs",
            "src/lib.rs",
            "tests/ast_gate.rs",
            "tests/build_gate.rs",
            "tests/gate.rs",
        ],
        "the source closure changed — a new file must be a declared target \
         root, a resolved module, or an admitted include, and this pin must \
         move in the same reviewed change"
    );
}

// ---------------------------------------------------------------------------
// Teeth: every rejection below is proven to fire against a fixture crate in a
// temporary directory, so a regression in the machinery cannot pass silently.
// ---------------------------------------------------------------------------

fn fixture(files: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("create fixture dir");
    for (path, contents) in files {
        let full = dir.path().join(path);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).expect("create fixture parents");
        }
        std::fs::write(full, contents).expect("write fixture file");
    }
    dir
}

fn lib_only_config() -> GateConfig<'static> {
    GateConfig {
        roots: &["src/lib.rs"],
        admitted_includes: &[],
        ffi_boundary_files: &[],
        allowed_macros: &["assert", "assert_eq", "matches", "stringify"],
        allowed_attributes: &["cfg", "test", "allow", "doc", "derive"],
        allowed_derives: &["Debug"],
    }
}

fn violations_of(files: &[(&str, &str)], config: &GateConfig) -> Vec<String> {
    let dir = fixture(files);
    run_gate(dir.path(), config).violations
}

fn assert_rejects(files: &[(&str, &str)], config: &GateConfig, needle: &str) {
    let violations = violations_of(files, config);
    assert!(
        violations
            .iter()
            .any(|violation| violation.contains(needle)),
        "expected a violation containing {needle:?}, got: {violations:?}"
    );
}

#[test]
fn a_clean_fixture_crate_passes() {
    let violations = violations_of(
        &[
            (
                "src/lib.rs",
                "mod child;\npub fn top() -> u32 { child::f() }\n",
            ),
            ("src/child.rs", "pub fn f() -> u32 { 7 }\n"),
        ],
        &lib_only_config(),
    );
    assert_eq!(violations, Vec::<String>::new());
}

#[test]
fn an_unlisted_rs_file_anywhere_in_the_crate_is_an_orphan() {
    assert_rejects(
        &[
            ("src/lib.rs", "pub fn f() {}\n"),
            ("src/stray.rs", "pub fn hidden() {}\n"),
        ],
        &lib_only_config(),
        "orphan",
    );
}

#[test]
fn a_path_attribute_on_a_module_is_rejected() {
    assert_rejects(
        &[
            ("src/lib.rs", "#[path = \"elsewhere.rs\"]\nmod child;\n"),
            ("src/elsewhere.rs", "pub fn f() {}\n"),
        ],
        &lib_only_config(),
        "attribute `path`",
    );
}

#[test]
fn an_unadmitted_include_is_rejected() {
    assert_rejects(
        &[
            ("src/lib.rs", "include!(\"shared.rs\");\n"),
            ("src/shared.rs", "pub fn f() {}\n"),
        ],
        &lib_only_config(),
        "not an admitted include edge",
    );
}

#[test]
fn an_admitted_include_joins_the_closure() {
    let config = GateConfig {
        admitted_includes: &[("src/lib.rs", "shared.rs")],
        ..lib_only_config()
    };
    let dir = fixture(&[
        ("src/lib.rs", "include!(\"shared.rs\");\n"),
        ("src/shared.rs", "pub fn f() {}\n"),
    ]);
    let outcome = run_gate(dir.path(), &config);
    assert_eq!(outcome.violations, Vec::<String>::new());
    assert!(
        outcome.closure.contains("src/shared.rs"),
        "closure: {:?}",
        outcome.closure
    );
}

#[test]
fn unsafe_hidden_inside_allowed_macro_tokens_is_rejected() {
    assert_rejects(
        &[("src/lib.rs", "pub fn f() { assert!(unsafe { true }); }\n")],
        &lib_only_config(),
        "forbidden token `unsafe`",
    );
}

#[test]
fn a_macro_definition_is_rejected() {
    assert_rejects(
        &[(
            "src/lib.rs",
            "macro_rules! sneaky { () => {}; }\npub fn f() {}\n",
        )],
        &lib_only_config(),
        "macro definitions are forbidden",
    );
}

#[test]
fn an_unsafe_block_outside_the_ffi_boundary_is_rejected() {
    assert_rejects(
        &[("src/lib.rs", "pub fn f() { unsafe {} }\n")],
        &lib_only_config(),
        "unsafe block outside the declared FFI boundary",
    );
}

#[test]
fn an_unsafe_block_inside_an_admitted_boundary_file_is_accepted() {
    // This is the admission mechanism the queued FFI item will use for
    // `src/engine/ffi.rs` and `src/engine/process.rs`.
    let config = GateConfig {
        ffi_boundary_files: &["src/boundary.rs"],
        ..lib_only_config()
    };
    let violations = violations_of(
        &[
            ("src/lib.rs", "mod boundary;\n"),
            ("src/boundary.rs", "pub fn f() { unsafe {} }\n"),
        ],
        &config,
    );
    assert_eq!(violations, Vec::<String>::new());
}

#[test]
fn an_extern_block_outside_the_boundary_is_rejected() {
    assert_rejects(
        &[(
            "src/lib.rs",
            "extern \"C\" { fn abs(input: i32) -> i32; }\n",
        )],
        &lib_only_config(),
        "extern block outside the declared FFI boundary",
    );
}

#[test]
fn an_extern_crate_declaration_is_rejected() {
    assert_rejects(
        &[("src/lib.rs", "extern crate core;\n")],
        &lib_only_config(),
        "extern crate declarations are forbidden",
    );
}

#[test]
fn ambiguous_module_candidates_are_rejected() {
    assert_rejects(
        &[
            ("src/lib.rs", "mod child;\n"),
            ("src/child.rs", "pub fn f() {}\n"),
            ("src/child/mod.rs", "pub fn f() {}\n"),
        ],
        &lib_only_config(),
        "ambiguous",
    );
}

#[test]
fn a_missing_module_file_is_rejected() {
    assert_rejects(
        &[("src/lib.rs", "mod child;\n")],
        &lib_only_config(),
        "has no file",
    );
}

#[test]
fn a_cfg_disabled_module_is_still_resolved() {
    // The gate checks cfg-disabled code as syntax: a module that the current
    // platform would skip still needs its file accounted for.
    assert_rejects(
        &[("src/lib.rs", "#[cfg(windows)]\nmod windows_only;\n")],
        &lib_only_config(),
        "has no file",
    );
}

#[test]
fn an_attribute_outside_the_allowlist_is_rejected() {
    assert_rejects(
        &[("src/lib.rs", "#[no_mangle]\npub fn f() {}\n")],
        &lib_only_config(),
        "attribute `no_mangle`",
    );
}

#[test]
fn a_derive_outside_the_allowlist_is_rejected() {
    assert_rejects(
        &[("src/lib.rs", "#[derive(Debug, Clone)]\npub struct S;\n")],
        &lib_only_config(),
        "derive `Clone`",
    );
}

#[test]
fn an_unknown_macro_invocation_is_rejected() {
    assert_rejects(
        &[("src/lib.rs", "pub fn f() { println!(\"x\"); }\n")],
        &lib_only_config(),
        "macro `println!` outside the closed allowlist",
    );
}

#[test]
fn a_smuggled_attribute_inside_macro_tokens_is_rejected() {
    assert_rejects(
        &[(
            "src/lib.rs",
            "pub fn f() -> &'static str { stringify!(#[path = \"x.rs\"] mod m;) }\n",
        )],
        &lib_only_config(),
        "smuggled inside macro tokens",
    );
}

#[test]
fn a_raw_identifier_inside_macro_tokens_is_rejected() {
    assert_rejects(
        &[(
            "src/lib.rs",
            "pub fn f(r#type: bool) { assert!(r#type); }\n",
        )],
        &lib_only_config(),
        "raw identifier",
    );
}

#[cfg(unix)]
#[test]
fn a_symlink_in_the_crate_tree_is_rejected() {
    let dir = fixture(&[
        ("src/lib.rs", "pub fn f() {}\n"),
        ("src/real.rs", "pub fn g() {}\n"),
    ]);
    std::os::unix::fs::symlink(
        dir.path().join("src/real.rs"),
        dir.path().join("src/link.rs"),
    )
    .expect("create symlink");
    let outcome = run_gate(dir.path(), &lib_only_config());
    assert!(
        outcome
            .violations
            .iter()
            .any(|violation| violation.contains("symlink")),
        "violations: {:?}",
        outcome.violations
    );
}

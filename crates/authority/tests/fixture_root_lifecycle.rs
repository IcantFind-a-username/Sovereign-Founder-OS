//! The closed lifecycle of the one root kind the fixture may use.
//!
//! `classify` answers "may this directory be used at all". This answers a
//! narrower question that only has meaning once that is settled: which
//! fixture root is this, and is it the one the caller thinks it is?
//!
//! Fixture roots are disposable — created, filled, discarded, recreated at
//! the same path. Without a generation, a stale handle points at the new root
//! and looks valid, because every structural check still passes.

#![cfg(feature = "owner-effect-fixture")]

use sovereign_authority::broker::bootstrap::FIXTURE_MARKER;
use sovereign_authority::broker::fixture_root::SyntheticFixtureRootV1;
use sovereign_authority::broker::protocol::Diagnostic;

#[test]
fn a_created_root_opens_at_its_own_generation() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("root");

    let created = SyntheticFixtureRootV1::create(&path, 7).unwrap();
    assert_eq!(created.generation(), 7);

    let opened = SyntheticFixtureRootV1::open(&path, 7).unwrap();
    assert_eq!(opened.generation(), 7);
    assert_eq!(opened.path(), created.path());
}

/// The whole reason the generation exists. A root discarded and recreated at
/// the same path passes every structural check, so only the generation tells
/// a stale handle from a live one.
#[test]
fn a_root_recreated_at_the_same_path_does_not_open_at_the_old_generation() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("root");

    SyntheticFixtureRootV1::create(&path, 1).unwrap();
    std::fs::remove_dir_all(&path).unwrap();
    SyntheticFixtureRootV1::create(&path, 2).unwrap();

    assert_eq!(
        SyntheticFixtureRootV1::open(&path, 1).err(),
        Some(Diagnostic::RootRejected),
        "a handle from the previous root opened the new one"
    );
    assert!(SyntheticFixtureRootV1::open(&path, 2).is_ok());
}

/// Creating over an existing marker would silently take over a root another
/// process might still be using. A caller who wants a fresh root can pick a
/// fresh path.
#[test]
fn creating_over_an_existing_root_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("root");
    SyntheticFixtureRootV1::create(&path, 1).unwrap();

    assert_eq!(
        SyntheticFixtureRootV1::create(&path, 2).err(),
        Some(Diagnostic::RootRejected),
        "a second create took over an existing root"
    );
    // And the original is untouched.
    assert!(SyntheticFixtureRootV1::open(&path, 1).is_ok());
}

/// A marker this fixture did not write does not open, whatever it says.
#[test]
fn a_marker_from_another_format_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("root");
    std::fs::create_dir(&path).unwrap();

    for marker in [
        // The bare tag an earlier slice's tests write: structurally a fixture
        // root, but carries no generation, so it is not one of these.
        "synthetic",
        "synthetic-owner-effect-fixture-v1",
        "synthetic-owner-effect-fixture-v1\n",
        "synthetic-owner-effect-fixture-v2\ngeneration=1\n",
        "synthetic-owner-effect-fixture-v1\ngeneration=not-a-number\n",
        "synthetic-owner-effect-fixture-v1\ngeneration=1\nextra=smuggled\n",
        "generation=1\nsynthetic-owner-effect-fixture-v1\n",
        "",
    ] {
        std::fs::write(path.join(FIXTURE_MARKER), marker).unwrap();
        assert_eq!(
            SyntheticFixtureRootV1::open(&path, 1).err(),
            Some(Diagnostic::RootRejected),
            "marker {marker:?} was accepted"
        );
    }
}

/// The structural checks still apply: a root that also looks like product
/// state is refused before its marker is even read.
#[test]
fn a_root_holding_product_state_is_still_refused() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("root");
    SyntheticFixtureRootV1::create(&path, 1).unwrap();
    std::fs::write(path.join("device.json"), b"{}").unwrap();

    assert_eq!(
        SyntheticFixtureRootV1::open(&path, 1).err(),
        Some(Diagnostic::RootRejected),
        "a root holding product state opened"
    );
}

/// There is no product root, so there is nothing for a fixture root to be
/// promoted into. The absence is the property.
///
/// Comments are stripped before the scan. A check that forbade a name in
/// prose as well as in code would push the next author to write a worse
/// comment rather than better code — and the doc comment above this module
/// says "no ProductRoot sibling exists", which is exactly the sentence that
/// should be encouraged.
#[test]
fn no_product_root_type_or_conversion_exists() {
    let source = include_str!("../src/broker/fixture_root.rs");
    let code: String = source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");

    for forbidden in [
        "ProductRoot",
        "ProductFixtureRoot",
        "impl From<",
        "fn into_product",
        "fn promote",
    ] {
        assert!(
            !code.contains(forbidden),
            "fixture_root.rs declares {forbidden:?}, which opens a path out of the fixture"
        );
    }
    // The scan is only worth something if it is looking at something: a
    // filter that removed every line would pass vacuously.
    assert!(
        code.contains("pub struct SyntheticFixtureRootV1"),
        "stripping comments removed the code as well"
    );
}

/// A caller cannot construct the type — holding one is evidence that `create`
/// or `open` ran. Both fields are private, so the struct literal is
/// unavailable outside the crate, and this asserts they stay that way.
#[test]
fn the_root_cannot_be_constructed_by_a_caller() {
    let source = include_str!("../src/broker/fixture_root.rs");
    assert!(
        source.contains(
            "pub struct SyntheticFixtureRootV1 {\n    path: PathBuf,\n    generation: u64,\n}"
        ),
        "the fields must stay private, or a caller can build a root that never passed a check"
    );
}

/// The root carries a path, and a path is a detail of the owner's machine.
#[test]
fn the_root_never_prints_its_path() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("root");
    let root = SyntheticFixtureRootV1::create(&path, 3).unwrap();

    let rendered = format!("{root:?}");
    assert!(rendered.contains("redacted"), "{rendered}");
    assert!(
        !rendered.contains(&path.display().to_string()),
        "the path appears: {rendered}"
    );
    // The generation is not secret and is useful in a failure message.
    assert!(rendered.contains('3'), "{rendered}");
}

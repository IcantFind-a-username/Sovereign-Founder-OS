//! Migrating a fixture store between generations, and refusing to on a
//! platform where the mechanism was never measured.
//!
//! These tests run on every platform and assert different things depending on
//! which one. That is deliberate. The publish's crash behaviour was
//! characterised on Linux x86_64; rename atomicity and what a crash between
//! the fsync and the rename leaves are filesystem properties, and APFS, NTFS
//! and a network mount each answer differently.
//!
//! So on a qualified platform the tests check that a migration produces
//! exactly one complete generation, and on every other they check that it
//! refuses and changes nothing. Both are real assertions; neither is skipped.
//! A test that quietly did nothing off Linux would leave the refusal path —
//! the one most developers actually run — unchecked.

#![cfg(feature = "owner-effect-fixture")]

use sovereign_authority::broker::platform_publish::{
    generations, has_staging_debris, migrate, platform_name, MigrationError, PLATFORM_IS_QUALIFIED,
};
use std::path::Path;

fn seeded() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("authority.1.redb"), b"generation one").unwrap();
    dir
}

fn attempt(root: &Path) -> Result<(), MigrationError> {
    migrate(root, 1, 2, |bytes| {
        let mut converted = bytes.to_vec();
        converted.extend_from_slice(b" converted");
        converted
    })
}

/// The qualified platform is exactly one, and it is a compile-time fact.
#[test]
fn the_qualified_platform_is_linux_x86_64_and_is_not_a_runtime_setting() {
    assert_eq!(
        PLATFORM_IS_QUALIFIED,
        cfg!(all(target_os = "linux", target_arch = "x86_64")),
        "{} disagrees with the compiled gate",
        platform_name()
    );

    let source = include_str!("../src/broker/platform_publish.rs");
    let code: String = source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    for override_shape in ["static mut", "fn set_qualified", "env::var", "AtomicBool"] {
        assert!(
            !code.contains(override_shape),
            "the platform gate can be widened at run time via {override_shape:?}"
        );
    }
}

/// On an unqualified platform the migration refuses and leaves the root
/// exactly as it found it — no target, no staging debris.
///
/// This is the path most developers run, and it is the one that would rot if
/// the test were skipped off Linux.
#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
#[test]
fn an_unqualified_platform_refuses_and_changes_nothing() {
    let dir = seeded();

    assert_eq!(
        attempt(dir.path()),
        Err(MigrationError::PlatformUnqualified)
    );

    assert_eq!(generations(dir.path()), vec![1], "a generation appeared");
    assert!(!has_staging_debris(dir.path()), "staging debris was left");
    assert_eq!(
        std::fs::read(dir.path().join("authority.1.redb")).unwrap(),
        b"generation one",
        "the source generation was touched"
    );
}

/// On the qualified platform a migration produces exactly one complete new
/// generation and no debris.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[test]
fn a_qualified_platform_publishes_exactly_one_complete_generation() {
    let dir = seeded();

    assert_eq!(attempt(dir.path()), Ok(()));

    assert_eq!(generations(dir.path()), vec![1, 2]);
    assert!(!has_staging_debris(dir.path()), "staging debris survived");
    assert_eq!(
        std::fs::read(dir.path().join("authority.2.redb")).unwrap(),
        b"generation one converted",
        "the published generation is not the converted bytes"
    );
    assert_eq!(
        std::fs::read(dir.path().join("authority.1.redb")).unwrap(),
        b"generation one",
        "the source was modified rather than read"
    );
}

/// A migration onto a generation that already exists refuses rather than
/// overwriting. Overwriting would destroy a generation something may still be
/// reading.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[test]
fn migrating_onto_an_existing_generation_is_refused() {
    let dir = seeded();
    std::fs::write(dir.path().join("authority.2.redb"), b"already here").unwrap();

    assert_eq!(attempt(dir.path()), Err(MigrationError::TargetExists));
    assert_eq!(
        std::fs::read(dir.path().join("authority.2.redb")).unwrap(),
        b"already here"
    );
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[test]
fn migrating_from_a_generation_that_is_not_there_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(attempt(dir.path()), Err(MigrationError::SourceMissing));
    assert!(generations(dir.path()).is_empty());
}

/// Staging debris from an interrupted migration is visible to a reader, so
/// "one complete generation" and "a migration that did not finish" are
/// distinguishable rather than both looking like a directory with one file.
#[test]
fn staging_debris_is_visible_to_a_reader() {
    let dir = seeded();
    assert!(!has_staging_debris(dir.path()));

    std::fs::write(dir.path().join("authority.2.redb.staging"), b"interrupted").unwrap();

    assert!(has_staging_debris(dir.path()));
    assert_eq!(
        generations(dir.path()),
        vec![1],
        "a staging file was counted as a generation"
    );
}

/// The generation listing reads names and nothing else, so a file that is not
/// a generation cannot be mistaken for one.
#[test]
fn only_generation_files_are_counted() {
    let dir = seeded();
    for name in [
        "authority.redb",
        "authority.x.redb",
        "authority.2.redb.staging",
        ".owner-effect-broker.lock",
        "notes.txt",
    ] {
        std::fs::write(dir.path().join(name), b"x").unwrap();
    }
    assert_eq!(generations(dir.path()), vec![1]);
}

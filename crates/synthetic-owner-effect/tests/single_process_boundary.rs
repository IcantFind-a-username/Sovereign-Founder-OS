//! v01-D02: release-excluded single-process fixture boundary.
//!
//! These tests pin the slice that is in scope: classifier, retained OS lock,
//! sole redb open, one listener in source, no internal transport. They do not
//! exercise owner sessions, WebAuthn, or effect publication.

#![cfg(feature = "owner-effect-fixture")]

use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use sovereign_authority::broker::bootstrap::FIXTURE_MARKER;
use sovereign_authority::broker::fixture_root::SyntheticFixtureRootV1;
use sovereign_authority::broker::process_lock::LOCK_FILE;
use sovereign_authority::broker::store::STORE_FILE;
use sovereign_synthetic_owner_effect::{BoundaryError, ProcessBoundary, RELEASE_EXCLUSION_NEEDLE};

fn fixture_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sovereign-synthetic-owner-effect"))
}

fn marked_root(parent: &Path) -> PathBuf {
    let path = parent.join("fixture-root");
    SyntheticFixtureRootV1::create(&path, 1).expect("a fresh fixture root must create");
    path
}

fn assert_no_listener_or_database(root: &Path) {
    assert!(
        !root.join(LOCK_FILE).exists(),
        "a rejected root must not create the lock file"
    );
    assert!(
        !root.join(STORE_FILE).exists(),
        "a rejected root must not open redb"
    );
}

/// Lock first, and only then the one production redb open. The type system
/// already requires `&HeldLock`; this test is the observable filesystem
/// order a later refactor could still invert by opening through another API.
#[test]
fn lock_acquisition_precedes_the_only_production_redb_open() {
    let dir = tempfile::tempdir().unwrap();
    let root = marked_root(dir.path());

    let boundary = ProcessBoundary::acquire(&root).expect("a marked root must lock");
    assert!(
        root.join(LOCK_FILE).is_file(),
        "the lock must be a real file before redb exists"
    );
    assert!(
        !root.join(STORE_FILE).exists(),
        "redb opened before the lock was proven held"
    );

    let _store = boundary
        .open_store()
        .expect("the sole production open must succeed under the lock");
    assert!(
        root.join(STORE_FILE).is_file(),
        "the store must be a real file beside the lock"
    );
}

/// The only two-live-process claim in this slice: a second real fixture
/// process is refused on the OS lock and never reaches redb open.
#[test]
fn second_real_fixture_process_fails_on_os_lock_before_redb_open() {
    let dir = tempfile::tempdir().unwrap();
    let root = marked_root(dir.path());

    let mut holder = Command::new(fixture_bin())
        .args(["--root", root.to_str().unwrap(), "--hold"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the fixture process must start");

    let stdout = holder.stdout.take().expect("holder stdout");
    let mut ready = false;
    for line in BufReader::new(stdout).lines() {
        if line.unwrap().contains("boundary-ready") {
            ready = true;
            break;
        }
    }
    assert!(ready, "the holder never reported a ready boundary");
    assert!(
        root.join(STORE_FILE).is_file(),
        "the holder must have opened redb so a skipped lock would surface as a store error"
    );

    let second = Command::new(fixture_bin())
        .args(["--root", root.to_str().unwrap()])
        .output()
        .expect("the second fixture process must start");

    let stderr = String::from_utf8_lossy(&second.stderr);
    assert!(
        !second.status.success(),
        "a second process was allowed to run: stderr={stderr}"
    );
    assert!(
        stderr.contains(BoundaryError::AlreadyRunning.code()),
        "the second process must fail on the OS lock, not later: stderr={stderr}"
    );
    assert!(
        !stderr.contains(BoundaryError::StoreUnavailable.code()),
        "the second process reached redb open: stderr={stderr}"
    );

    holder.kill().ok();
    let _ = holder.wait();
}

/// Product, unmarked, and symlink roots end at classification — no lock, no
/// redb, and (because acquire never binds) no listener state.
#[test]
fn product_unmarked_or_symlink_roots_fail_before_listener_or_database_state() {
    let dir = tempfile::tempdir().unwrap();

    let unmarked = dir.path().join("unmarked");
    fs::create_dir(&unmarked).unwrap();
    assert_eq!(
        ProcessBoundary::acquire(&unmarked).err(),
        Some(BoundaryError::RootRejected)
    );
    assert_no_listener_or_database(&unmarked);

    let product = dir.path().join("product");
    fs::create_dir(&product).unwrap();
    fs::write(product.join(FIXTURE_MARKER), b"synthetic\n").unwrap();
    fs::write(product.join("device.json"), b"{}").unwrap();
    assert_eq!(
        ProcessBoundary::acquire(&product).err(),
        Some(BoundaryError::RootRejected)
    );
    assert_no_listener_or_database(&product);

    #[cfg(unix)]
    {
        let real = marked_root(dir.path());
        let link = dir.path().join("link-to-root");
        std::os::unix::fs::symlink(&real, &link).unwrap();
        assert_eq!(
            ProcessBoundary::acquire(&link).err(),
            Some(BoundaryError::RootRejected)
        );
        // The target must stay untouched: following the symlink then locking
        // would create the lock beside the real fixture root.
        assert_no_listener_or_database(&real);
    }
}

/// Source inventory: one public listener, no internal transport module.
#[test]
fn source_inventory_has_one_listener_and_no_internal_transport_module() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    collect_rs(&src, &mut files);
    files.sort();
    assert!(
        !files.is_empty(),
        "source inventory inspected nothing under {}",
        src.display()
    );

    let mut bind_hits = 0;
    let mut listener_files = 0;
    for path in &files {
        let text = fs::read_to_string(path).unwrap();
        let relative = path.strip_prefix(&src).unwrap();
        let name = relative.to_string_lossy();

        if name == "listener.rs" {
            listener_files += 1;
            assert!(
                text.contains("TcpListener::bind"),
                "the one listener module must contain the bind"
            );
            assert!(
                text.contains("Ipv4Addr::LOCALHOST"),
                "the listener must bind loopback, not an unspecified address"
            );
        } else {
            assert!(
                !text.contains("TcpListener::bind"),
                "{name} must not bind; the crate has one listener"
            );
        }

        bind_hits += text.matches("TcpListener::bind").count();

        for forbidden in [
            "hmac::",
            "use hmac",
            "__owner-effect-broker",
            "run_owner_effect_fixture_broker",
            "mod supervisor",
            "mod connections",
            "mod protocol",
            "Ipv4Addr::UNSPECIFIED",
            "Ipv6Addr::UNSPECIFIED",
            "BROKER_SUBCOMMAND",
        ] {
            assert!(
                !text.contains(forbidden),
                "{name} contains forbidden v1 transport residue {forbidden:?}"
            );
        }
    }

    assert_eq!(
        listener_files, 1,
        "there must be exactly one listener module"
    );
    assert_eq!(
        bind_hits, 1,
        "source inventory must contain exactly one TcpListener::bind, found {bind_hits}"
    );

    let boundary = fs::read_to_string(src.join("boundary.rs")).unwrap();
    assert!(
        boundary.contains("process_lock::acquire"),
        "the boundary must take the existing OS lock"
    );
    assert!(
        boundary.contains("OwnedStore::open"),
        "the boundary must open redb only through OwnedStore"
    );
    assert_eq!(
        boundary.matches("OwnedStore::open").count(),
        1,
        "there must be exactly one production redb open"
    );
    assert!(
        !boundary.contains("bind_public_origin"),
        "acquire/open must not bind the listener"
    );
    assert!(
        RELEASE_EXCLUSION_NEEDLE.contains("synthetic-owner-effect"),
        "the release needle must name this package"
    );
}

fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            collect_rs(&path, out);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

use super::*;
use std::io::{BufRead, BufReader};

/// The point of the whole guard. If this passes while running as root, the
/// injection is real; a chmod-based guard would silently inject nothing there
/// and every test built on it would false-green.
#[test]
fn blocking_a_directory_makes_writes_beneath_it_fail() {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("store");
    fs::create_dir(&store).unwrap();
    fs::write(store.join("existing"), b"kept").unwrap();

    let blocked = BlockedPath::block(&store).unwrap();
    let error = fs::write(store.join("new"), b"denied").unwrap_err();
    assert!(
        matches!(
            error.kind(),
            io::ErrorKind::NotADirectory | io::ErrorKind::AlreadyExists | io::ErrorKind::Other
        ),
        "a blocked directory must refuse writes, got {error:?}"
    );
    assert!(store.is_file(), "the blocker itself must be a regular file");

    drop(blocked);
    assert!(store.is_dir(), "the original directory must come back");
    assert_eq!(fs::read(store.join("existing")).unwrap(), b"kept");
    fs::write(store.join("new"), b"allowed").unwrap();
}

/// Blocking a location that does not exist yet is the case where the code
/// under test is the thing that would have created it.
#[test]
fn blocking_a_missing_location_denies_its_creation_and_leaves_nothing_behind() {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("not-created-yet");

    let blocked = BlockedPath::block(&store).unwrap();
    assert!(fs::create_dir_all(&store).is_err());
    assert!(fs::write(store.join("entry"), b"denied").is_err());

    drop(blocked);
    assert!(
        !store.exists(),
        "a location that did not exist must not exist afterwards"
    );
}

/// Blocking a regular file would prove nothing — writes to it still succeed —
/// so the guard refuses rather than injecting a fault that is not one.
#[test]
fn blocking_a_regular_file_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("a-file");
    fs::write(&file, b"content").unwrap();

    assert!(BlockedPath::block(&file).is_err());
    assert_eq!(fs::read(&file).unwrap(), b"content");
}

#[test]
fn corrupt_byte_changes_one_byte_and_keeps_the_length() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("entry");
    fs::write(&path, b"0123456789").unwrap();

    corrupt_byte(&path, 4).unwrap();
    let after = fs::read(&path).unwrap();
    assert_eq!(after.len(), 10);
    assert_eq!(&after[..4], b"0123");
    assert_ne!(after[4], b'4');
    assert_eq!(&after[5..], b"56789");
}

#[test]
fn corrupt_byte_past_the_end_is_an_error_not_a_silent_no_op() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("entry");
    fs::write(&path, b"short").unwrap();

    assert!(corrupt_byte(&path, 99).is_err());
    assert_eq!(fs::read(&path).unwrap(), b"short");
}

#[test]
fn truncate_by_removes_exactly_the_tail() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("entry");
    fs::write(&path, b"0123456789").unwrap();

    truncate_by(&path, 3).unwrap();
    assert_eq!(fs::read(&path).unwrap(), b"0123456");
}

#[test]
fn truncate_by_more_than_the_file_holds_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("entry");
    fs::write(&path, b"short").unwrap();

    assert!(truncate_by(&path, 99).is_err());
    assert_eq!(fs::read(&path).unwrap(), b"short");
}

const WORKER_MARKER: (&str, &str) = ("SOVEREIGN_FAULT_TESTING_WORKER", "1");

/// Proves the respawn actually re-entered this binary and ran the ignored
/// worker: the parent reads the marker the child printed, then kills it while
/// it is still alive, which is the whole point of the primitive.
#[test]
fn respawn_self_runs_the_ignored_worker_and_can_be_killed_at_its_marker() {
    let mut child = respawn_self("tests::worker_that_waits_to_be_killed", WORKER_MARKER).unwrap();
    let stdout = child.stdout.take().unwrap();

    let mut reached = false;
    for line in BufReader::new(stdout).lines() {
        if line.unwrap().contains("worker-ready") {
            reached = true;
            break;
        }
    }
    assert!(reached, "the worker never reported that it started");

    child.kill().unwrap();
    let status = child.wait().unwrap();
    assert!(!status.success(), "a killed worker must not report success");
}

/// The worker half. `#[ignore]` keeps it out of ordinary runs, and the marker
/// keeps it from doing anything even if someone runs the ignored set by hand.
#[test]
#[ignore = "spawned by respawn_self; not a standalone test"]
fn worker_that_waits_to_be_killed() {
    if std::env::var(WORKER_MARKER.0).as_deref() != Ok(WORKER_MARKER.1) {
        return;
    }
    println!("worker-ready");
    // Bounded so a parent that regresses and never kills cannot hang CI.
    std::thread::sleep(std::time::Duration::from_secs(30));
}

/// The crate's whole safety property: it may appear under `[dev-dependencies]`
/// anywhere, and under `[dependencies]` nowhere. A production edge would ship
/// fault injection inside the product.
///
/// Parsing the manifests catches the mistake at the moment it is written,
/// which `cargo build` does not: a dev-dependency and a real dependency build
/// exactly the same way.
#[test]
fn no_production_crate_depends_on_this() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf();

    let mut manifests = Vec::new();
    collect_manifests(&workspace.join("crates"), &mut manifests);
    collect_manifests(&workspace.join("apps"), &mut manifests);
    collect_manifests(&workspace.join("tests"), &mut manifests);
    assert!(
        manifests.len() > 10,
        "found only {} manifests; the scan is not reaching the workspace",
        manifests.len()
    );

    for manifest in manifests {
        let text = fs::read_to_string(&manifest).unwrap();
        let mut section = "";
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('[') {
                section = trimmed;
            }
            if trimmed.starts_with("sovereign-fault-testing") {
                assert_ne!(
                    section,
                    "[dependencies]",
                    "{} depends on the fault-injection crate in production",
                    manifest.display()
                );
            }
        }
    }
}

fn collect_manifests(dir: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let manifest = path.join("Cargo.toml");
            if manifest.is_file() {
                found.push(manifest);
            }
            collect_manifests(&path, found);
        }
    }
}

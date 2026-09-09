//! The broker's bootstrap frame and root classification.
//!
//! Two properties, and the second is why the first is written so strictly.
//!
//! Anything but one exact frame ends the broker before it looks at a root.
//! And a root is classified by the broker itself, so a same-account caller
//! that hands the hidden mode a perfectly valid frame — which RFC 0006 says
//! is possible, and calls unqualified fixture control rather than product
//! admission — still cannot aim it at the owner's Vault or workspace.

#![cfg(feature = "owner-effect-fixture")]

use sovereign_authority::broker::bootstrap::{classify, FIXTURE_MARKER};
use sovereign_authority::broker::protocol::{
    decode, encode, read_frame, Diagnostic, KEY_LEN, MAGIC, MAX_FRAME_LEN, MAX_ROOT_LEN, NONCE_LEN,
};
use std::path::Path;

const KEY: [u8; KEY_LEN] = [7; KEY_LEN];
const NONCE: [u8; NONCE_LEN] = [9; NONCE_LEN];

fn fixture_root(parent: &Path) -> std::path::PathBuf {
    let root = parent.join("fixture-root");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join(FIXTURE_MARKER), b"synthetic").unwrap();
    root
}

// ------------------------------------------------------------------ frame --

#[test]
fn a_well_formed_frame_round_trips() {
    let dir = tempfile::tempdir().unwrap();
    let root = fixture_root(dir.path());
    let frame = encode(&KEY, &NONCE, &root);
    let decoded = decode(&frame).expect("a frame this encoder produced must decode");
    assert_eq!(decoded.launch_key, KEY);
    assert_eq!(decoded.nonce, NONCE);
    assert_eq!(decoded.root, root);
}

/// No input at all is the case a direct caller produces by simply running the
/// hidden subcommand, and it must be distinguishable from a malformed one.
#[test]
fn an_empty_frame_is_missing_not_malformed() {
    assert_eq!(decode(&[]).err(), Some(Diagnostic::BootstrapMissing));
}

#[test]
fn a_truncated_frame_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let frame = encode(&KEY, &NONCE, &fixture_root(dir.path()));
    for cut in [1, MAGIC.len(), MAGIC.len() + 2, frame.len() - 1] {
        assert!(
            matches!(
                decode(&frame[..cut]).err(),
                Some(Diagnostic::BootstrapTruncated) | Some(Diagnostic::BootstrapMalformed)
            ),
            "a frame cut to {cut} bytes was accepted"
        );
    }
}

/// Trailing bytes are not ignored. A parser that accepted a valid prefix
/// would let a caller append anything it liked to a legitimate frame.
#[test]
fn a_frame_with_trailing_bytes_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let mut frame = encode(&KEY, &NONCE, &fixture_root(dir.path()));
    frame.push(0);
    assert_eq!(decode(&frame).err(), Some(Diagnostic::BootstrapMalformed));
}

#[test]
fn a_frame_with_the_wrong_magic_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let mut frame = encode(&KEY, &NONCE, &fixture_root(dir.path()));
    frame[0] = b'X';
    assert_eq!(decode(&frame).err(), Some(Diagnostic::BootstrapMalformed));
}

#[test]
fn an_unsupported_version_is_named_as_such() {
    let dir = tempfile::tempdir().unwrap();
    let mut frame = encode(&KEY, &NONCE, &fixture_root(dir.path()));
    let at = MAGIC.len();
    frame[at..at + 2].copy_from_slice(&99u16.to_le_bytes());
    assert_eq!(
        decode(&frame).err(),
        Some(Diagnostic::BootstrapVersionUnsupported)
    );
}

/// Oversized input is refused rather than buffered: the reader takes one byte
/// more than a legal frame and no more, so a caller cannot make the broker
/// hold an unbounded amount of its choosing.
#[test]
fn an_oversized_frame_is_refused_without_being_buffered() {
    let huge = vec![0u8; MAX_FRAME_LEN * 4];
    let mut cursor = std::io::Cursor::new(huge);
    assert_eq!(
        read_frame(&mut cursor).err(),
        Some(Diagnostic::BootstrapOversized)
    );
    assert!(
        cursor.position() <= (MAX_FRAME_LEN + 1) as u64,
        "the reader consumed {} bytes; it must stop at the cap",
        cursor.position()
    );
}

#[test]
fn a_root_longer_than_the_cap_cannot_be_encoded_into_a_valid_frame() {
    let long = std::path::PathBuf::from("/".to_owned() + &"a".repeat(MAX_ROOT_LEN + 10));
    let frame = encode(&KEY, &NONCE, &long);
    assert!(frame.len() > MAX_FRAME_LEN);
    assert_eq!(decode(&frame).err(), Some(Diagnostic::BootstrapOversized));
}

/// The frame carries the launch key, so anything that prints it leaks it.
#[test]
fn the_parsed_frame_never_prints_its_key() {
    let dir = tempfile::tempdir().unwrap();
    let root = fixture_root(dir.path());
    let decoded = decode(&encode(&KEY, &NONCE, &root)).unwrap();
    let rendered = format!("{decoded:?}");
    assert!(rendered.contains("redacted"), "{rendered}");
    assert!(
        !rendered.contains(&format!("{}", KEY[0])) || !rendered.contains('['),
        "the key must not appear in a debug rendering: {rendered}"
    );
}

// ------------------------------------------------------------------- root --

#[test]
fn a_marked_directory_classifies() {
    let dir = tempfile::tempdir().unwrap();
    let root = fixture_root(dir.path());
    assert!(classify(&root).is_ok());
}

/// The marker is a positive claim, and its absence is decisive. An allow-list
/// is used rather than a deny-list because the next product file to be
/// invented would not be on any deny-list.
#[test]
fn an_unmarked_directory_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let plain = dir.path().join("plain");
    std::fs::create_dir(&plain).unwrap();
    assert_eq!(classify(&plain).err(), Some(Diagnostic::RootRejected));
}

/// This is the one that matters: a caller that drops the fixture marker into
/// the owner's real data root must not thereby gain a broker over it.
#[test]
fn a_marked_directory_that_also_looks_like_product_state_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let root = fixture_root(dir.path());
    std::fs::write(root.join("device.json"), b"{}").unwrap();
    assert_eq!(
        classify(&root).err(),
        Some(Diagnostic::RootRejected),
        "a root holding product state must be refused even when marked"
    );
}

/// Nor by creating a marked directory *inside* the owner's data root.
#[test]
fn a_marked_directory_nested_inside_product_state_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let product = dir.path().join("sovereign-founder-os");
    std::fs::create_dir(&product).unwrap();
    std::fs::write(product.join("ledger.json"), b"[]").unwrap();
    let nested = fixture_root(&product);
    assert_eq!(
        classify(&nested).err(),
        Some(Diagnostic::RootRejected),
        "a fixture root nested in product state must be refused"
    );
}

/// A symlink is refused before it is followed. Canonicalizing first would
/// resolve it and then approve whatever it pointed at.
#[cfg(unix)]
#[test]
fn a_symlinked_root_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let real = fixture_root(dir.path());
    let link = dir.path().join("link-to-root");
    std::os::unix::fs::symlink(&real, &link).unwrap();
    assert_eq!(classify(&link).err(), Some(Diagnostic::RootRejected));
}

/// So is a marker that is itself a symlink, for the same reason.
#[cfg(unix)]
#[test]
fn a_symlinked_marker_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root");
    std::fs::create_dir(&root).unwrap();
    let elsewhere = dir.path().join("elsewhere");
    std::fs::write(&elsewhere, b"synthetic").unwrap();
    std::os::unix::fs::symlink(&elsewhere, root.join(FIXTURE_MARKER)).unwrap();
    assert_eq!(classify(&root).err(), Some(Diagnostic::RootRejected));
}

/// Every rejection reports the same code. Telling a caller which rule it
/// tripped would let it probe the filesystem through this interface.
#[test]
fn every_root_rejection_is_indistinguishable() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("does-not-exist");
    let unmarked = dir.path().join("unmarked");
    std::fs::create_dir(&unmarked).unwrap();
    let a_file = dir.path().join("a-file");
    std::fs::write(&a_file, b"x").unwrap();

    for candidate in [&missing, &unmarked, &a_file] {
        assert_eq!(
            classify(candidate).err(),
            Some(Diagnostic::RootRejected),
            "{} produced a distinguishable rejection",
            candidate.display()
        );
    }
}

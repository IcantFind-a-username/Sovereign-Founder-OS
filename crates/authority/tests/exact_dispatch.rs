//! Writing the one message, and what to believe when that is interrupted.
//!
//! The write is the ordinary durable pattern. The interesting part is what a
//! later process concludes from the disk, and the answer is deliberately
//! "I don't know" in the one case where nothing on disk can say.

#![cfg(feature = "owner-effect-fixture")]

use sovereign_authority::broker::dispatch::{dispatch, recover, DispatchError, DispatchOutcome};
use sovereign_authority::broker::exact_fixture::{EffectIntentId, ProtectedFixturePayload};
use std::path::Path;

fn outbox() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

fn published(root: &Path, id: EffectIntentId) -> std::path::PathBuf {
    root.join(format!("{}.eml", id.file_stem()))
}

fn debris(root: &Path, id: EffectIntentId) -> std::path::PathBuf {
    root.join(format!("{}.eml.tmp", id.file_stem()))
}

#[test]
fn a_dispatch_publishes_the_exact_message_and_no_debris() {
    let dir = outbox();
    let id = EffectIntentId::allocate();
    let payload = ProtectedFixturePayload::compose(id);

    assert_eq!(
        dispatch(dir.path(), &payload),
        Ok(DispatchOutcome::Dispatched)
    );

    let file = published(dir.path(), id);
    assert!(file.is_file());
    assert!(!debris(dir.path(), id).exists(), "a temp file survived");

    // Whole and RFC 5322 shaped, read from disk rather than from the type.
    let bytes = std::fs::read(&file).unwrap();
    let text = String::from_utf8(bytes).expect("the message is ASCII");
    assert!(
        text.contains("\r\n\r\n"),
        "no header/body separator on disk"
    );
    assert!(text.starts_with("From: "));
    assert!(text.ends_with("\r\n"));
}

/// The filename is the intent id and nothing else, so a directory listing
/// cannot be read for a hint about any message in it.
#[test]
fn the_published_filename_carries_only_the_intent_id() {
    let dir = outbox();
    let id = EffectIntentId::allocate();
    dispatch(dir.path(), &ProtectedFixturePayload::compose(id)).unwrap();

    let names: Vec<String> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, vec![format!("{}.eml", id.file_stem())]);
    for fragment in ["fixture-recipient", "example", "CANARY"] {
        assert!(
            !names[0].contains(fragment),
            "the filename carries {fragment}"
        );
    }
}

/// A second dispatch refuses rather than overwrites. Overwriting would make
/// the file a record of the last attempt instead of the one that happened.
#[test]
fn a_second_dispatch_of_the_same_intent_is_refused() {
    let dir = outbox();
    let id = EffectIntentId::allocate();
    let payload = ProtectedFixturePayload::compose(id);
    dispatch(dir.path(), &payload).unwrap();

    assert_eq!(
        dispatch(dir.path(), &payload),
        Err(DispatchError::AlreadyDispatched)
    );
}

/// The one that matters. A leftover temp file means some earlier attempt
/// reached the write and not the publish, and nothing on disk says whether
/// the bytes were complete. Retrying is the natural thing to code and the
/// wrong thing to do: for a message, an automatic retry after an ambiguous
/// outcome is how one send becomes two, and the person who notices is the
/// recipient.
#[test]
fn debris_makes_the_outcome_indeterminate_and_never_retries() {
    let dir = outbox();
    let id = EffectIntentId::allocate();
    let payload = ProtectedFixturePayload::compose(id);

    // Exactly what a kill between write and rename leaves.
    std::fs::write(debris(dir.path(), id), b"partially written").unwrap();

    assert_eq!(
        dispatch(dir.path(), &payload),
        Err(DispatchError::Indeterminate),
        "an interrupted attempt was retried"
    );
    assert!(
        !published(dir.path(), id).exists(),
        "a retry published a file after an ambiguous outcome"
    );
    assert_eq!(recover(dir.path(), id), DispatchOutcome::Indeterminate);
}

/// The debris is left where it is. Deleting it would destroy the only
/// evidence that the ambiguity existed.
#[test]
fn recovery_reads_the_disk_and_changes_nothing() {
    let dir = outbox();
    let id = EffectIntentId::allocate();
    std::fs::write(debris(dir.path(), id), b"partial").unwrap();

    let before: Vec<String> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();

    assert_eq!(recover(dir.path(), id), DispatchOutcome::Indeterminate);
    assert_eq!(recover(dir.path(), id), DispatchOutcome::Indeterminate);

    let after: Vec<String> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        before, after,
        "recovery tidied up the evidence it exists to read"
    );
    assert_eq!(std::fs::read(debris(dir.path(), id)).unwrap(), b"partial");
}

/// Debris is reported before an already-published file, because an
/// interrupted attempt is the more urgent fact and a root with both needs a
/// person to look at it.
#[test]
fn debris_is_reported_even_when_a_file_is_also_published() {
    let dir = outbox();
    let id = EffectIntentId::allocate();
    let payload = ProtectedFixturePayload::compose(id);
    dispatch(dir.path(), &payload).unwrap();
    std::fs::write(debris(dir.path(), id), b"a later interrupted attempt").unwrap();

    assert_eq!(
        dispatch(dir.path(), &payload),
        Err(DispatchError::Indeterminate),
        "debris was hidden by an already-published file"
    );
    assert_eq!(recover(dir.path(), id), DispatchOutcome::Indeterminate);
}

#[test]
fn an_intent_that_was_never_dispatched_reads_as_refused() {
    let dir = outbox();
    assert_eq!(
        recover(dir.path(), EffectIntentId::allocate()),
        DispatchOutcome::Refused
    );
}

/// A write into an unavailable outbox publishes nothing and leaves no debris
/// of its own — this process knows the temp file is its own, so the ambiguity
/// a later process must preserve is one it cannot attribute.
#[test]
fn an_unavailable_outbox_publishes_nothing() {
    let dir = outbox();
    let root = dir.path().join("outbox");
    std::fs::create_dir(&root).unwrap();
    let id = EffectIntentId::allocate();
    let payload = ProtectedFixturePayload::compose(id);

    let blocked = sovereign_fault_testing::BlockedPath::block(&root).unwrap();
    assert_eq!(dispatch(&root, &payload), Err(DispatchError::Unavailable));
    drop(blocked);

    let names: Vec<String> = std::fs::read_dir(&root)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(names.is_empty(), "a failed dispatch left {names:?}");
}

/// There are three outcomes and no fourth. An outcome set that can express
/// "probably fine" eventually records "probably fine".
#[test]
fn the_outcome_set_is_closed() {
    let source = include_str!("../src/broker/dispatch.rs");
    let code: String = source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    let declaration = code
        .split("pub enum DispatchOutcome {")
        .nth(1)
        .and_then(|rest| rest.split('}').next())
        .expect("the outcome enum");
    let variants: Vec<&str> = declaration
        .lines()
        .filter_map(|line| line.trim().strip_suffix(','))
        .collect();
    assert_eq!(variants, vec!["Dispatched", "Refused", "Indeterminate"]);

    for hedge in ["Probably", "Likely", "Assumed", "Retrying"] {
        assert!(!code.contains(hedge), "the outcome set gained {hedge}");
    }
}

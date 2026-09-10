//! Writing the one message to disk, and what to believe when that is
//! interrupted.
//!
//! The write itself is the ordinary durable pattern: a temp file, fsynced,
//! renamed over the target, and the directory fsynced. A crash at any point
//! leaves either no file or the complete one.
//!
//! The interesting part is what a *later* process concludes. A leftover temp
//! file means some earlier attempt reached the point of writing and did not
//! reach the point of publishing. That is genuinely ambiguous: the bytes may
//! have been fully written and the rename lost, or the process may have died
//! mid-write. Nothing on disk distinguishes them.
//!
//! So recovery is conservative in the specific sense that matters here: an
//! interrupted attempt becomes `Indeterminate`, and `Indeterminate` never
//! retries on its own. Retrying would be the natural thing to code and the
//! wrong thing to do — for a message, an automatic retry after an ambiguous
//! outcome is how one send becomes two, and the person who would notice is
//! the recipient. The ambiguity is surfaced rather than resolved by guessing,
//! and a person decides.
//!
//! The debris is left in place for the same reason. Deleting it would destroy
//! the only evidence that the ambiguity existed.

use super::exact_fixture::{EffectIntentId, ProtectedFixturePayload};
use std::io::Write;
use std::path::{Path, PathBuf};

/// What happened to one dispatch. The same three outcomes the evidence chain
/// records, and no fourth: an outcome set that can express "probably fine"
/// eventually records "probably fine".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchOutcome {
    /// The file is complete and published.
    Dispatched,
    /// Nothing was written and nothing was published.
    Refused,
    /// An earlier attempt was interrupted. Unknown, and stays unknown.
    Indeterminate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchError {
    /// The outbox could not be written. Nothing was published.
    Unavailable,
    /// This intent already has a published file.
    AlreadyDispatched,
    /// An earlier attempt left debris. Refused, and left for a person.
    Indeterminate,
}

fn published_path(root: &Path, intent_id: EffectIntentId) -> PathBuf {
    root.join(format!("{}.eml", intent_id.file_stem()))
}

fn temp_path(root: &Path, intent_id: EffectIntentId) -> PathBuf {
    root.join(format!("{}.eml.tmp", intent_id.file_stem()))
}

/// Write the payload, exactly once.
///
/// Refuses rather than overwrites if a file is already published, and refuses
/// rather than retries if an earlier attempt left a temp file. Both are cases
/// where the safe action is to stop and say so.
pub fn dispatch(
    root: &Path,
    payload: &ProtectedFixturePayload,
) -> Result<DispatchOutcome, DispatchError> {
    let intent_id = payload.intent_id();
    let published = published_path(root, intent_id);
    let temp = temp_path(root, intent_id);

    // Debris first. An earlier interrupted attempt is the more urgent fact,
    // and checking the published file first would report AlreadyDispatched
    // for a root that also needs a person to look at it.
    if temp.exists() {
        return Err(DispatchError::Indeterminate);
    }
    if published.exists() {
        return Err(DispatchError::AlreadyDispatched);
    }

    let result = (|| -> std::io::Result<()> {
        let mut file = std::fs::File::create(&temp)?;
        file.write_all(payload.sealed_bytes())?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&temp, &published)?;
        #[cfg(unix)]
        std::fs::File::open(root)?.sync_all()?;
        Ok(())
    })();

    match result {
        Ok(()) => Ok(DispatchOutcome::Dispatched),
        Err(_) => {
            // A failure during the write leaves debris this process knows is
            // its own, so it is cleaned up here — the ambiguity a later
            // process must preserve is one it cannot attribute.
            let _ = std::fs::remove_file(&temp);
            Err(DispatchError::Unavailable)
        }
    }
}

/// What a later process should conclude about one intent, from the disk
/// alone.
///
/// Deliberately has no side effects. A recovery routine that tidied up as it
/// looked would destroy the evidence it is there to read.
pub fn recover(root: &Path, intent_id: EffectIntentId) -> DispatchOutcome {
    if temp_path(root, intent_id).exists() {
        return DispatchOutcome::Indeterminate;
    }
    if published_path(root, intent_id).exists() {
        return DispatchOutcome::Dispatched;
    }
    DispatchOutcome::Refused
}

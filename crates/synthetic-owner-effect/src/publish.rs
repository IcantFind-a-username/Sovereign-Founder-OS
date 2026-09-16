//! Owner-only same-directory publication. No replace, no retry.

use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::outcome::{hit_publish, PublishError, PublishFailpoint};
use crate::sealed::{EffectIntentId, SealedPayload};

pub fn published_path(root: &Path, intent_id: EffectIntentId) -> PathBuf {
    root.join(format!("{}.eml", intent_id.file_stem()))
}

pub fn temp_path(root: &Path, intent_id: EffectIntentId) -> PathBuf {
    root.join(format!("{}.eml.tmp", intent_id.file_stem()))
}

pub fn writer_io_observed(root: &Path, intent_id: EffectIntentId) -> bool {
    published_path(root, intent_id).exists() || temp_path(root, intent_id).exists()
}

/// Read-only observation of one intent's files. Never creates, deletes, or
/// rewrites anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublicationObservation {
    Identical,
    Absent,
    Different,
    WrongType,
    Unreadable,
    UncertainDurability,
}

pub(crate) fn observe_publication(
    root: &Path,
    intent_id: EffectIntentId,
    expected: &[u8],
) -> PublicationObservation {
    if temp_path(root, intent_id).exists() {
        return PublicationObservation::UncertainDurability;
    }
    let published = published_path(root, intent_id);
    let metadata = match published.symlink_metadata() {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return PublicationObservation::Absent;
        }
        Err(_) => return PublicationObservation::Unreadable,
    };
    if !metadata.file_type().is_file() {
        return PublicationObservation::WrongType;
    }
    match std::fs::read(&published) {
        Ok(bytes) if bytes == expected => PublicationObservation::Identical,
        Ok(_) => PublicationObservation::Different,
        Err(_) => PublicationObservation::Unreadable,
    }
}

pub(crate) fn expected_bytes(intent_id: EffectIntentId) -> Vec<u8> {
    SealedPayload::compose(intent_id).sealed_bytes().to_vec()
}

/// Exact bytes the writer would publish. Recovery compares against these.
/// This is not a writer entry.
pub fn expected_publication_bytes(intent_id: EffectIntentId) -> Vec<u8> {
    expected_bytes(intent_id)
}

/// Publish `<effect_intent_id>.eml` once. Caller must have committed
/// `Dispatching`. Failures after that point leave debris in place.
pub(crate) fn publish_exact(
    root: &Path,
    intent_id: EffectIntentId,
    bytes: &[u8],
) -> Result<(), PublishError> {
    let published = published_path(root, intent_id);
    let temp = temp_path(root, intent_id);
    if temp.exists() || published.exists() {
        return Err(PublishError::AlreadyPublished);
    }

    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }

    let mut file = options.open(&temp).map_err(|_| PublishError::Unavailable)?;
    if bytes.is_empty() {
        return Err(PublishError::Unavailable);
    }
    file.write_all(&bytes[..1])
        .map_err(|_| PublishError::Unavailable)?;
    hit_publish(PublishFailpoint::AfterFirstWrite)?;
    if bytes.len() > 1 {
        file.write_all(&bytes[1..])
            .map_err(|_| PublishError::Unavailable)?;
    }
    file.sync_all().map_err(|_| PublishError::Unavailable)?;
    drop(file);
    hit_publish(PublishFailpoint::AfterTempFlush)?;
    hit_publish(PublishFailpoint::BeforePublication)?;
    match std::fs::hard_link(&temp, &published) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            return Err(PublishError::AlreadyPublished);
        }
        Err(_) => return Err(PublishError::Unavailable),
    }
    hit_publish(PublishFailpoint::AfterPublicationBeforeDirFlush)?;
    let _ = std::fs::remove_file(&temp);
    #[cfg(unix)]
    File::open(root)
        .and_then(|directory| directory.sync_all())
        .map_err(|_| PublishError::Unavailable)?;
    Ok(())
}

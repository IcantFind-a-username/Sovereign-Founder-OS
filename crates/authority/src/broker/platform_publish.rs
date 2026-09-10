//! Moving a fixture store from one generation to the next.
//!
//! A migration writes a new store beside the old one and then publishes it by
//! rename. The reason to write it at all — rather than mutating in place — is
//! that a crash during an in-place migration leaves a store that is neither
//! the old shape nor the new one, and nothing on disk says which fields were
//! already converted.
//!
//! **The platform gate is the honest part.** The crash behaviour of this
//! publish was characterised on Linux x86_64 and nowhere else. Rename
//! atomicity across a directory, and what a crash between the fsync and the
//! rename leaves, are filesystem properties rather than language ones; APFS,
//! NTFS and a network mount each answer differently, and none of them has
//! been measured here.
//!
//! So on any other platform this refuses. Not "probably fine" and not "best
//! effort" — a migration that half-works is exactly the outcome the whole
//! design is arranged to avoid, and running an unmeasured mechanism to avoid
//! saying "unavailable" is how a maturity label becomes untrue.
//!
//! Widening the gate means measuring another platform and adding it here,
//! which is a change a reviewer sees.

use std::path::{Path, PathBuf};

/// Whether this build's platform has had the publish mechanism characterised.
///
/// A `const`, not a runtime flag: a platform list that could be overridden at
/// run time is a platform list an operator can be talked into widening.
pub const PLATFORM_IS_QUALIFIED: bool = cfg!(all(target_os = "linux", target_arch = "x86_64"));

/// The name a qualified platform reports, for a message that says which one
/// was refused rather than only that one was.
pub fn platform_name() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationError {
    /// This platform's publish behaviour has not been characterised.
    PlatformUnqualified,
    /// The source generation is not there.
    SourceMissing,
    /// The target generation already exists.
    TargetExists,
    /// The filesystem refused. Nothing was published.
    Unavailable,
}

fn generation_path(root: &Path, generation: u64) -> PathBuf {
    root.join(format!("authority.{generation}.redb"))
}

fn staging_path(root: &Path, generation: u64) -> PathBuf {
    root.join(format!("authority.{generation}.redb.staging"))
}

/// Migrate one generation to the next.
///
/// `convert` is handed the source bytes and returns the target's. It is a
/// closure rather than a fixed transformation so that this module owns the
/// *publishing* and knows nothing about the shape being migrated — the two
/// change for entirely different reasons.
pub fn migrate(
    root: &Path,
    from_generation: u64,
    to_generation: u64,
    convert: impl FnOnce(&[u8]) -> Vec<u8>,
) -> Result<(), MigrationError> {
    if !PLATFORM_IS_QUALIFIED {
        return Err(MigrationError::PlatformUnqualified);
    }

    let source = generation_path(root, from_generation);
    let target = generation_path(root, to_generation);
    if !source.is_file() {
        return Err(MigrationError::SourceMissing);
    }
    if target.exists() {
        return Err(MigrationError::TargetExists);
    }

    let bytes = std::fs::read(&source).map_err(|_| MigrationError::Unavailable)?;
    let converted = convert(&bytes);

    let staging = staging_path(root, to_generation);
    let publish = (|| -> std::io::Result<()> {
        {
            // Scoped so the handle is closed before the rename. A rename over
            // a file this process still holds open is a different operation
            // on every platform, and the point of the gate is to not rely on
            // behaviour nobody measured.
            use std::io::Write;
            let mut file = std::fs::File::create(&staging)?;
            file.write_all(&converted)?;
            file.sync_all()?;
        }
        std::fs::rename(&staging, &target)?;
        std::fs::File::open(root)?.sync_all()?;
        Ok(())
    })();

    match publish {
        Ok(()) => Ok(()),
        Err(_) => {
            // This process knows the staging file is its own, so removing it
            // is not destroying somebody else's evidence.
            let _ = std::fs::remove_file(&staging);
            Err(MigrationError::Unavailable)
        }
    }
}

/// Which generations exist, so a reader can tell one complete generation from
/// a half-finished migration.
pub fn generations(root: &Path) -> Vec<u64> {
    let Ok(entries) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    let mut found: Vec<u64> = entries
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            name.strip_prefix("authority.")
                .and_then(|rest| rest.strip_suffix(".redb"))
                .and_then(|number| number.parse::<u64>().ok())
        })
        .collect();
    found.sort_unstable();
    found
}

/// Whether an interrupted migration left staging debris.
pub fn has_staging_debris(root: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(root) else {
        return false;
    };
    entries.filter_map(|entry| entry.ok()).any(|entry| {
        entry
            .file_name()
            .to_string_lossy()
            .ends_with(".redb.staging")
    })
}

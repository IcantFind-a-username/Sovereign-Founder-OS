//! RFC 0007 Amendment 1 §d: freshness generation invalidates prior authority.
//!
//! Audit-chain verification is never consulted here. A `verify_chain` pass
//! over a restored ledger is not execute authority.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{unavailable, AuthorityError, AuthorityStore};

const GENERATION_FILE: &str = "freshness-generation.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct GenerationRecord {
    freshness_generation: u64,
}

impl AuthorityStore {
    fn store_root(&self) -> &Path {
        self.tokens
            .parent()
            .expect("authority token directory is nested under the store root")
    }

    fn generation_path(&self) -> std::path::PathBuf {
        self.store_root().join(GENERATION_FILE)
    }

    /// Bound generation, if this store has been enrolled or recovered.
    pub fn freshness_generation(&self) -> Result<Option<u64>, AuthorityError> {
        let path = self.generation_path();
        if !path.is_file() {
            return Ok(None);
        }
        let bytes = std::fs::read(&path).map_err(unavailable)?;
        let record: GenerationRecord =
            serde_json::from_slice(&bytes).map_err(|_| AuthorityError::CorruptRecord)?;
        if record.freshness_generation < 1 {
            return Err(AuthorityError::CorruptRecord);
        }
        Ok(Some(record.freshness_generation))
    }

    /// Persist the store's execute-path generation. Must be ≥ 1.
    pub fn bind_freshness_generation(&self, generation: u64) -> Result<(), AuthorityError> {
        if generation < 1 {
            return Err(AuthorityError::StaleGeneration);
        }
        let record = GenerationRecord {
            freshness_generation: generation,
        };
        let path = self.generation_path();
        let json = serde_json::to_vec_pretty(&record).map_err(unavailable)?;
        atomic_write_private(&path, &json)
    }

    /// Recovery / re-enrollment: new generation must be strictly above the
    /// current bound value (or 0 when unbound). Prior consume markers,
    /// dispatch handles, session epochs, and effect-intent authority stamped
    /// at the superseded generation become invalid for new execute paths.
    pub fn advance_freshness_generation(&self, new_generation: u64) -> Result<(), AuthorityError> {
        let current = self.freshness_generation()?.unwrap_or(0);
        if new_generation <= current {
            return Err(AuthorityError::StaleGeneration);
        }
        self.bind_freshness_generation(new_generation)
    }

    /// Gate for a new execute path at the enrolled generation.
    ///
    /// Deliberately takes only the enrolled number — not a ledger, not a
    /// `verify_chain` result. Callers that have only a verified audit chain
    /// still cannot authorize effects from that fact alone.
    pub fn authorize_new_execute(&self, enrolled_generation: u64) -> Result<(), AuthorityError> {
        match self.freshness_generation()? {
            Some(bound) if bound == enrolled_generation => Ok(()),
            _ => Err(AuthorityError::StaleGeneration),
        }
    }

    pub(crate) fn reject_stale_generation(
        &self,
        record_generation: Option<u64>,
    ) -> Result<(), AuthorityError> {
        match (self.freshness_generation()?, record_generation) {
            (None, _) => Ok(()),
            (Some(bound), Some(record)) if bound == record => Ok(()),
            (Some(_), _) => Err(AuthorityError::StaleGeneration),
        }
    }
}

fn atomic_write_private(path: &Path, bytes: &[u8]) -> Result<(), AuthorityError> {
    use std::io::Write;
    let temp_path = path.with_extension("tmp");
    let result = (|| {
        #[cfg(unix)]
        {
            use std::fs::OpenOptions;
            use std::os::unix::fs::OpenOptionsExt;
            let mut file = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(&temp_path)
                .map_err(unavailable)?;
            file.write_all(bytes).map_err(unavailable)?;
            file.sync_all().map_err(unavailable)?;
            drop(file);
        }
        #[cfg(not(unix))]
        {
            let mut file = std::fs::File::create(&temp_path).map_err(unavailable)?;
            file.write_all(bytes).map_err(unavailable)?;
            file.sync_all().map_err(unavailable)?;
            drop(file);
        }
        std::fs::rename(&temp_path, path).map_err(unavailable)?;
        #[cfg(unix)]
        if let Some(directory) = path.parent() {
            std::fs::File::open(directory)
                .map_err(unavailable)?
                .sync_all()
                .map_err(unavailable)?;
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temp_path);
    }
    result
}

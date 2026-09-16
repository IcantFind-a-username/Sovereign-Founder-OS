//! RFC 0007 Amendment 1: enrolled `freshness_generation`.
//!
//! This record is a separate store. It is never implied by `ledger.json` or
//! `ledger.head` alone. Co-located files are **not** independently protected
//! until RFC 0005 / Program 1C1 custody; the Target is a path that survives a
//! workspace-tree-only restore. Whole-device rollback stays Research.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::head::write_atomic_private;
use crate::{AuditLedger, LedgerError, LedgerHead};

pub const ENROLLED_FRESHNESS_FILENAME: &str = "freshness.enrolled";
pub const ENROLLMENT_CLAIMED_FILENAME: &str = "enrollment.claimed";

/// Enrolled latest-head generation for one workspace binding.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EnrolledFreshness {
    pub freshness_generation: u64,
    pub enrolled_at_unix: u64,
    pub workspace_binding: String,
}

impl EnrolledFreshness {
    pub fn enroll(
        freshness_generation: u64,
        workspace_binding: impl Into<String>,
        enrolled_at_unix: u64,
    ) -> Result<Self, LedgerError> {
        if freshness_generation < 1 {
            return Err(LedgerError::InvalidEnrolledGeneration);
        }
        Ok(Self {
            freshness_generation,
            enrolled_at_unix,
            workspace_binding: workspace_binding.into(),
        })
    }

    pub fn save(&self, path: &Path) -> Result<(), LedgerError> {
        if self.freshness_generation < 1 {
            return Err(LedgerError::InvalidEnrolledGeneration);
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_vec_pretty(self)?;
        write_atomic_private(path, &json)?;
        Ok(())
    }

    pub fn load(path: &Path) -> Result<Self, LedgerError> {
        let bytes = std::fs::read(path)?;
        let enrolled: Self = serde_json::from_slice(&bytes)?;
        if enrolled.freshness_generation < 1 {
            return Err(LedgerError::InvalidEnrolledGeneration);
        }
        Ok(enrolled)
    }
}

/// How open resolved Amendment 1 generation rules after the v0.1 anchor check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreshnessDisposition {
    Normal,
    /// Enrollment was claimed and the enrolled record is gone. Not a normal open.
    LimitedRecovery,
}

pub fn enrolled_freshness_path(store_dir: &Path) -> PathBuf {
    store_dir.join(ENROLLED_FRESHNESS_FILENAME)
}

pub fn enrollment_claimed_path(store_dir: &Path) -> PathBuf {
    store_dir.join(ENROLLMENT_CLAIMED_FILENAME)
}

pub fn enrollment_is_claimed(store_dir: &Path) -> bool {
    enrollment_claimed_path(store_dir).is_file()
}

/// Persist the "enrollment claimed" marker. Missing enrolled after this
/// marker exists is limited recovery, never a silent accept.
pub fn claim_enrollment(store_dir: &Path) -> Result<(), LedgerError> {
    std::fs::create_dir_all(store_dir)?;
    write_atomic_private(enrollment_claimed_path(store_dir).as_path(), b"enrolled\n")?;
    Ok(())
}

pub fn load_enrolled(store_dir: &Path) -> Result<Option<EnrolledFreshness>, LedgerError> {
    let path = enrolled_freshness_path(store_dir);
    if !path.is_file() {
        return Ok(None);
    }
    Ok(Some(EnrolledFreshness::load(&path)?))
}

/// Persist enrolled generation **before** treating an authoritative head as
/// successful. Caller then writes the matching v2 anchor.
pub fn persist_enrolled(store_dir: &Path, enrolled: &EnrolledFreshness) -> Result<(), LedgerError> {
    claim_enrollment(store_dir)?;
    enrolled.save(&enrolled_freshness_path(store_dir))
}

/// Open-time generation rules (RFC 0007 Amendment 1 §b / §c / §e).
/// Call after `verify_freshness` succeeds.
pub fn verify_generation(
    ledger: &AuditLedger,
    head: Option<&LedgerHead>,
    enrolled: Option<&EnrolledFreshness>,
    enrollment_claimed: bool,
) -> Result<FreshnessDisposition, LedgerError> {
    match enrolled {
        None if enrollment_claimed => Ok(FreshnessDisposition::LimitedRecovery),
        None => Ok(FreshnessDisposition::Normal),
        Some(enrolled) => {
            let Some(anchor) = head else {
                return Err(LedgerError::MissingAnchor);
            };
            if let Some(trusted) = ledger.trusted_device_public_key_b64() {
                if enrolled.workspace_binding != trusted
                    || anchor.workspace_binding != enrolled.workspace_binding
                {
                    return Err(LedgerError::AnchorDeviceMismatch);
                }
            }
            let Some(anchor_gen) = anchor.freshness_generation else {
                // Valid v1 signatures still do not satisfy a higher enrolled gen.
                return Err(LedgerError::GenerationDowngrade);
            };
            if anchor_gen < enrolled.freshness_generation {
                return Err(LedgerError::GenerationDowngrade);
            }
            if anchor_gen != enrolled.freshness_generation {
                return Err(LedgerError::GenerationMismatch);
            }
            if (ledger.events().len() as u64) > anchor.event_count {
                // Forward extension without a matching generation bump.
                return Err(LedgerError::GenerationMismatch);
            }
            Ok(FreshnessDisposition::Normal)
        }
    }
}

pub fn verify_freshness_and_generation(
    ledger: &AuditLedger,
    head: Option<&LedgerHead>,
    require_anchor: bool,
    enrolled: Option<&EnrolledFreshness>,
    enrollment_claimed: bool,
) -> Result<FreshnessDisposition, LedgerError> {
    crate::verify_freshness(ledger, head, require_anchor)?;
    verify_generation(ledger, head, enrolled, enrollment_claimed)
}

/// Owner limited-recovery bump: strictly above any generation found in
/// restored artifacts. Does not reuse pre-restore grants (caller invalidates
/// authority separately).
pub fn recover_enrolled_generation(
    workspace_binding: impl Into<String>,
    now_unix: u64,
    restored_heads: &[&LedgerHead],
    leftover: Option<&EnrolledFreshness>,
) -> Result<EnrolledFreshness, LedgerError> {
    let mut highest = 0u64;
    for head in restored_heads {
        if let Some(generation) = head.freshness_generation {
            highest = highest.max(generation);
        }
    }
    if let Some(enrolled) = leftover {
        highest = highest.max(enrolled.freshness_generation);
    }
    EnrolledFreshness::enroll(
        highest.saturating_add(1).max(1),
        workspace_binding,
        now_unix,
    )
}

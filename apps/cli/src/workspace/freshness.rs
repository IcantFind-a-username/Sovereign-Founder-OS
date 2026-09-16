//! RFC 0007 Amendment 1: enrolled generation at workspace open and persist.

use super::util::{now, storage};
use super::*;
use std::path::Path;

use sovereign_audit_ledger::{
    enrollment_is_claimed, ledger_head_path, load_enrolled, persist_enrolled,
    recover_enrolled_generation, verify_freshness_and_generation, AuditLedger, EnrolledFreshness,
    FreshnessDisposition, LedgerError, LedgerHead,
};
use sovereign_authority::AuthorityStore;
use sovereign_identity::DeviceIdentity;

impl Store {
    pub(super) fn verify_enrolled_generation(&self) -> Result<(), WorkspaceError> {
        let Some(enrollment_dir) = self.enrollment_dir.as_deref() else {
            return Ok(());
        };
        let ledger_path = self.root.join("ledger.json");
        let claimed = enrollment_is_claimed(enrollment_dir);
        let enrolled = load_enrolled(enrollment_dir).map_err(map_ledger)?;

        if !ledger_path.exists() {
            return match (claimed, enrolled) {
                (true, None) => Err(limited_recovery_missing()),
                (false, None) => self.enroll_fresh(enrollment_dir, 1),
                (_, Some(existing)) => {
                    self.bind_authority_generation(existing.freshness_generation)
                }
            };
        }

        let ledger =
            AuditLedger::load(&ledger_path, self.device.public_key_b64()).map_err(map_ledger)?;
        let head_path = ledger_head_path(&ledger_path);
        let head = if head_path.is_file() {
            Some(LedgerHead::load(&head_path).map_err(map_ledger)?)
        } else {
            None
        };

        match (claimed, enrolled) {
            (true, None) => Err(limited_recovery_missing()),
            (false, None) => {
                let generation = head
                    .as_ref()
                    .and_then(|anchor| anchor.freshness_generation)
                    .unwrap_or(0)
                    .saturating_add(1)
                    .max(1);
                persist_enrolled(
                    enrollment_dir,
                    &EnrolledFreshness::enroll(
                        generation,
                        self.device.public_key_b64(),
                        unix_now(),
                    )
                    .map_err(map_ledger)?,
                )
                .map_err(map_ledger)?;
                ledger
                    .save_with_generation(&ledger_path, &self.device, generation)
                    .map_err(map_ledger)?;
                self.bind_authority_generation(generation)
            }
            (_, Some(enrolled)) => {
                if enrolled.workspace_binding != self.device.public_key_b64() {
                    return Err(storage(LedgerError::AnchorDeviceMismatch));
                }
                match verify_freshness_and_generation(
                    &ledger,
                    head.as_ref(),
                    false,
                    Some(&enrolled),
                    claimed,
                )
                .map_err(map_ledger)?
                {
                    FreshnessDisposition::Normal => {
                        self.bind_authority_generation(enrolled.freshness_generation)
                    }
                    FreshnessDisposition::LimitedRecovery => Err(limited_recovery_missing()),
                }
            }
        }
    }

    pub(super) fn persist_authoritative_generation(
        &self,
        ledger: &AuditLedger,
        ledger_path: &Path,
    ) -> Result<(), WorkspaceError> {
        let Some(enrollment_dir) = self.enrollment_dir.as_deref() else {
            ledger.save(ledger_path, &self.device).map_err(storage)?;
            return Ok(());
        };
        let current = load_enrolled(enrollment_dir)
            .map_err(map_ledger)?
            .ok_or_else(limited_recovery_missing)?;
        let next = current
            .freshness_generation
            .checked_add(1)
            .ok_or_else(|| WorkspaceError::Storage("freshness generation overflow".into()))?;
        persist_enrolled(
            enrollment_dir,
            &EnrolledFreshness::enroll(next, self.device.public_key_b64(), unix_now())
                .map_err(map_ledger)?,
        )
        .map_err(map_ledger)?;
        ledger
            .save_with_generation(ledger_path, &self.device, next)
            .map_err(map_ledger)?;
        self.bind_authority_generation(next)
    }

    /// Owner ceremony after limited recovery: bump above every restored
    /// artifact and invalidate prior authority. Does not reuse pre-restore grants.
    pub fn complete_limited_recovery(
        root: &Path,
        enrollment_dir: &Path,
    ) -> Result<Self, WorkspaceError> {
        std::fs::create_dir_all(root).map_err(storage)?;
        let device_path = root.join("device.json");
        if !device_path.exists() {
            return Err(WorkspaceError::Storage(
                "limited recovery requires the workspace device identity".into(),
            ));
        }
        let device = DeviceIdentity::load(&device_path).map_err(storage)?;
        let ledger_path = root.join("ledger.json");
        let head = if ledger_head_path(&ledger_path).is_file() {
            Some(LedgerHead::load(&ledger_head_path(&ledger_path)).map_err(map_ledger)?)
        } else {
            None
        };
        let leftover = load_enrolled(enrollment_dir).map_err(map_ledger)?;
        let heads: Vec<&LedgerHead> = head.iter().collect();
        let recovered = recover_enrolled_generation(
            device.public_key_b64(),
            unix_now(),
            &heads,
            leftover.as_ref(),
        )
        .map_err(map_ledger)?;
        persist_enrolled(enrollment_dir, &recovered).map_err(map_ledger)?;
        if ledger_path.exists() {
            let ledger =
                AuditLedger::load(&ledger_path, device.public_key_b64()).map_err(map_ledger)?;
            ledger
                .save_with_generation(&ledger_path, &device, recovered.freshness_generation)
                .map_err(map_ledger)?;
        }
        let authority = AuthorityStore::open(root.join("authority")).map_err(storage)?;
        authority
            .advance_freshness_generation(recovered.freshness_generation)
            .or_else(|error| {
                if matches!(error, sovereign_authority::AuthorityError::StaleGeneration) {
                    authority.bind_freshness_generation(recovered.freshness_generation)
                } else {
                    Err(error)
                }
            })
            .map_err(storage)?;
        Self::open_enrolled(root, enrollment_dir)
    }

    fn enroll_fresh(&self, enrollment_dir: &Path, generation: u64) -> Result<(), WorkspaceError> {
        persist_enrolled(
            enrollment_dir,
            &EnrolledFreshness::enroll(generation, self.device.public_key_b64(), unix_now())
                .map_err(map_ledger)?,
        )
        .map_err(map_ledger)?;
        self.bind_authority_generation(generation)
    }

    fn bind_authority_generation(&self, generation: u64) -> Result<(), WorkspaceError> {
        let store = AuthorityStore::open(self.root.join("authority")).map_err(storage)?;
        match store.freshness_generation().map_err(storage)? {
            Some(bound) if bound == generation => Ok(()),
            Some(bound) if bound < generation => store
                .advance_freshness_generation(generation)
                .map_err(storage),
            Some(_) => Err(WorkspaceError::Storage(
                "authority freshness generation is ahead of the enrolled record".into(),
            )),
            None => store.bind_freshness_generation(generation).map_err(storage),
        }
    }
}

fn unix_now() -> u64 {
    u64::try_from(now()).unwrap_or(0)
}

fn limited_recovery_missing() -> WorkspaceError {
    WorkspaceError::LimitedRecovery(
        "enrolled freshness generation is missing after enrollment was claimed; \
         this is not a normal workspace open"
            .into(),
    )
}

fn map_ledger(error: LedgerError) -> WorkspaceError {
    match error {
        LedgerError::GenerationDowngrade | LedgerError::GenerationMismatch => {
            WorkspaceError::Storage(error.to_string())
        }
        other => storage(other),
    }
}

//! Durable Authority Store: crash-safe, cross-process one-use consumption.
//!
//! RFC 0002 requires that Capability V2 validation and consumption be a
//! single atomic operation that survives restarts and concurrent processes,
//! and that an unavailable store denies execution. This crate implements the
//! minimal durable core of that requirement for three kinds of authority:
//!
//! - one-use capability tokens (`consume_token`);
//! - one-use RFC 0003 approvals (`consume_approval`);
//! - idempotency keys bound to an invocation fingerprint
//!   (`bind_idempotency`), where re-presenting the same fingerprint is a
//!   replay and a different fingerprint is a conflict.
//!
//! ## Atomicity and durability
//!
//! A claim is a filesystem hard link into a content-addressed slot:
//! the record content is first written to an exclusive temporary file and
//! flushed, then `hard_link` publishes it under the authority id. Both POSIX
//! and Windows guarantee that linking to an existing name fails, so exactly
//! one process wins a race, and the winning record is always complete before
//! it becomes visible. Directories are flushed on Unix after publication.
//!
//! ## Bundle transactions (RFC 0003 Amendment 1)
//!
//! `consume_bundle` claims a token, an approval, and an idempotency key as
//! one recoverable transaction instead of three separate claims. The bundle
//! id is deterministic — a SHA-256 of the three subject ids plus the
//! invocation fingerprint — so a consumer that crashes mid-bundle and
//! retries with the same inputs reconstructs and resumes its own bundle
//! (every step is idempotent for the owning bundle) instead of burning its
//! earlier claims or colliding with a stranger's. Only a durable
//! `.committed` marker authorizes proceeding to the effect; recovery is
//! roll-forward only, so nothing is ever deleted to recover. Consumption
//! also fails closed on a durably revoked token or approval, but this crate
//! does not yet publish revocation records itself — `revoke_token` and
//! `revoke_approval` land in a follow-up entry, at which point today's
//! checks start observing real revocations.
//!
//! ## Honest limits
//!
//! Consumption is recorded before execution, so a crash between claim and
//! execution burns the authority without running anything — that is the
//! fail-closed direction and re-issuance is the recovery path. Expired
//! records can be purged; replaying a purged id is denied by the token or
//! approval expiry checks, never by this store. This store does not make
//! external effects safe by itself: crash-safe audit intent ordering and
//! reviewed host interfaces remain separate RFC 0002 requirements.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

const TOKENS_DIR: &str = "tokens";
const APPROVALS_DIR: &str = "approvals";
const IDEMPOTENCY_DIR: &str = "idempotency";
const BUNDLES_DIR: &str = "bundles";
const REVOKED_TOKENS_DIR: &str = "revoked-tokens";
const REVOKED_APPROVALS_DIR: &str = "revoked-approvals";
const MAX_RECORD_BYTES: u64 = 4 * 1024;
const BUNDLE_DOMAIN: &[u8] = b"sovereign:authority-bundle:v1";

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum AuthorityError {
    #[error("authority was already consumed")]
    AlreadyConsumed,
    #[error("idempotency key was already consumed for the same invocation")]
    IdempotencyReplay,
    #[error("idempotency key was consumed for a different invocation")]
    IdempotencyConflict,
    #[error("authority was revoked")]
    Revoked,
    #[error("authority record is invalid or corrupt")]
    CorruptRecord,
    #[error("authority store unavailable: {0}")]
    Unavailable(String),
}

#[derive(Debug, Serialize, Deserialize)]
struct AuthorityRecord {
    kind: String,
    fingerprint_hex: Option<String>,
    /// The bundle that claimed this record, if any. Absent on every record
    /// written before bundle transactions existed; old records keep parsing
    /// and old code ignores the field.
    #[serde(default)]
    bundle_hex: Option<String>,
    consumed_at_unix: i64,
    expires_at_unix: i64,
}

/// One part of an authority bundle: the subject id being claimed and its own
/// (kind-specific) expiry.
#[derive(Debug, Clone, Copy)]
pub struct BundlePart {
    pub id: Uuid,
    pub expires_at_unix: i64,
}

/// The bundle's grouping record (`bundles/<bundle_hex>` and its
/// `.committed` sibling both carry these fields).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct BundleIntentRecord {
    kind: String,
    token_id: Uuid,
    approval_id: Uuid,
    idempotency_key: Uuid,
    invocation_fingerprint_hex: String,
    created_at_unix: i64,
    expires_at_unix: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BundleCommittedRecord {
    kind: String,
    token_id: Uuid,
    approval_id: Uuid,
    idempotency_key: Uuid,
    invocation_fingerprint_hex: String,
    created_at_unix: i64,
    expires_at_unix: i64,
    consumed_at_unix: i64,
}

trait Expiring {
    fn expires_at_unix(&self) -> i64;
}

impl Expiring for AuthorityRecord {
    fn expires_at_unix(&self) -> i64 {
        self.expires_at_unix
    }
}

/// Reads just enough of a bundle intent or committed-marker record to purge
/// it by expiry, without caring which of the two shapes it is.
#[derive(Debug, Deserialize)]
struct BundleExpiry {
    expires_at_unix: i64,
}

impl Expiring for BundleExpiry {
    fn expires_at_unix(&self) -> i64 {
        self.expires_at_unix
    }
}

/// File-backed durable authority store. Every instance operating on the same
/// directory — in this process or another — observes the same consumption
/// state.
#[derive(Debug)]
pub struct AuthorityStore {
    root: PathBuf,
    tokens: PathBuf,
    approvals: PathBuf,
    idempotency: PathBuf,
    bundles: PathBuf,
}

impl AuthorityStore {
    pub fn open(root: impl AsRef<Path>) -> Result<Self, AuthorityError> {
        let root = root.as_ref().to_path_buf();
        let tokens = root.join(TOKENS_DIR);
        let approvals = root.join(APPROVALS_DIR);
        let idempotency = root.join(IDEMPOTENCY_DIR);
        let bundles = root.join(BUNDLES_DIR);
        for directory in [&tokens, &approvals, &idempotency, &bundles] {
            std::fs::create_dir_all(directory).map_err(unavailable)?;
        }
        Ok(Self {
            root,
            tokens,
            approvals,
            idempotency,
            bundles,
        })
    }

    /// Atomically consume a one-use token. Exactly one caller across all
    /// processes ever succeeds for a given id.
    pub fn consume_token(
        &self,
        token_id: Uuid,
        now_unix: i64,
        expires_at_unix: i64,
    ) -> Result<(), AuthorityError> {
        self.claim(
            &self.tokens,
            token_id,
            AuthorityRecord {
                kind: "token".into(),
                fingerprint_hex: None,
                bundle_hex: None,
                consumed_at_unix: now_unix,
                expires_at_unix,
            },
        )
        .map_err(|error| match error {
            ClaimError::Exists(_) => AuthorityError::AlreadyConsumed,
            ClaimError::Store(store_error) => store_error,
        })
    }

    /// Atomically consume a one-use RFC 0003 approval id.
    pub fn consume_approval(
        &self,
        approval_id: Uuid,
        now_unix: i64,
        expires_at_unix: i64,
    ) -> Result<(), AuthorityError> {
        self.claim(
            &self.approvals,
            approval_id,
            AuthorityRecord {
                kind: "approval".into(),
                fingerprint_hex: None,
                bundle_hex: None,
                consumed_at_unix: now_unix,
                expires_at_unix,
            },
        )
        .map_err(|error| match error {
            ClaimError::Exists(_) => AuthorityError::AlreadyConsumed,
            ClaimError::Store(store_error) => store_error,
        })
    }

    /// Atomically bind an idempotency key to an invocation fingerprint.
    /// A second binding with the same fingerprint is a replay; with a
    /// different fingerprint it is a conflict.
    pub fn bind_idempotency(
        &self,
        key: Uuid,
        fingerprint: &[u8; 32],
        now_unix: i64,
        expires_at_unix: i64,
    ) -> Result<(), AuthorityError> {
        let fingerprint_hex = hex::encode(fingerprint);
        match self.claim(
            &self.idempotency,
            key,
            AuthorityRecord {
                kind: "idempotency".into(),
                fingerprint_hex: Some(fingerprint_hex.clone()),
                bundle_hex: None,
                consumed_at_unix: now_unix,
                expires_at_unix,
            },
        ) {
            Ok(()) => Ok(()),
            Err(ClaimError::Exists(existing)) => match existing.fingerprint_hex.as_deref() {
                Some(existing_hex) if existing_hex == fingerprint_hex => {
                    Err(AuthorityError::IdempotencyReplay)
                }
                Some(_) => Err(AuthorityError::IdempotencyConflict),
                None => Err(AuthorityError::CorruptRecord),
            },
            Err(ClaimError::Store(store_error)) => Err(store_error),
        }
    }

    /// Atomically consume a token, an approval, and an idempotency key as
    /// one recoverable transaction (RFC 0003 Amendment 1, part a). `Ok(())`
    /// means this call observed the one `Authorized` outcome for the
    /// bundle — a durable `.committed` marker. Every other outcome,
    /// including a retry of a bundle someone else already committed, is
    /// `AlreadyConsumed`, `Revoked`, a replay/conflict error, or
    /// `CorruptRecord`.
    pub fn consume_bundle(
        &self,
        token: BundlePart,
        approval: BundlePart,
        idempotency: BundlePart,
        invocation_fingerprint: &[u8; 32],
        now_unix: i64,
    ) -> Result<(), AuthorityError> {
        let bundle_hex = compute_bundle_hex(
            token.id,
            approval.id,
            idempotency.id,
            invocation_fingerprint,
        );
        let intent = self.bundle_publish_intent(
            &bundle_hex,
            token,
            approval,
            idempotency,
            invocation_fingerprint,
            now_unix,
        )?;
        self.bundle_check_revocation(token.id, approval.id)?;
        self.bundle_claim_token(&bundle_hex, token, now_unix)?;
        self.bundle_bind_idempotency(&bundle_hex, idempotency, invocation_fingerprint, now_unix)?;
        self.bundle_claim_approval(&bundle_hex, approval, now_unix)?;
        self.bundle_commit(&bundle_hex, &intent, now_unix)
    }

    /// Step 1: publish (or retry-confirm) the bundle's intent record.
    fn bundle_publish_intent(
        &self,
        bundle_hex: &str,
        token: BundlePart,
        approval: BundlePart,
        idempotency: BundlePart,
        invocation_fingerprint: &[u8; 32],
        now_unix: i64,
    ) -> Result<BundleIntentRecord, AuthorityError> {
        let expires_at_unix = token
            .expires_at_unix
            .max(approval.expires_at_unix)
            .max(idempotency.expires_at_unix);
        let record = BundleIntentRecord {
            kind: "bundle-intent".into(),
            token_id: token.id,
            approval_id: approval.id,
            idempotency_key: idempotency.id,
            invocation_fingerprint_hex: hex::encode(invocation_fingerprint),
            created_at_unix: now_unix,
            expires_at_unix,
        };
        let final_path = self.bundles.join(bundle_hex);
        match publish_record(&self.bundles, &final_path, &record)? {
            None => Ok(record),
            Some(existing) => {
                let same_identity = existing.token_id == record.token_id
                    && existing.approval_id == record.approval_id
                    && existing.idempotency_key == record.idempotency_key
                    && existing.invocation_fingerprint_hex == record.invocation_fingerprint_hex;
                if same_identity {
                    // A retry: the durable intent (with its original
                    // created_at_unix) is authoritative, not this attempt's.
                    Ok(existing)
                } else {
                    Err(AuthorityError::CorruptRecord)
                }
            }
        }
    }

    /// Steps 2 and 6: fail closed if either subject has a durable
    /// revocation record. Nothing publishes into `revoked-tokens/` or
    /// `revoked-approvals/` yet (that is the follow-up revocation entry),
    /// so this check is a no-op until then.
    fn bundle_check_revocation(
        &self,
        token_id: Uuid,
        approval_id: Uuid,
    ) -> Result<(), AuthorityError> {
        let revoked = self
            .root
            .join(REVOKED_TOKENS_DIR)
            .join(token_id.to_string())
            .exists()
            || self
                .root
                .join(REVOKED_APPROVALS_DIR)
                .join(approval_id.to_string())
                .exists();
        if revoked {
            return Err(AuthorityError::Revoked);
        }
        Ok(())
    }

    /// Step 3: claim the token under this bundle, or confirm this bundle
    /// already holds it.
    fn bundle_claim_token(
        &self,
        bundle_hex: &str,
        token: BundlePart,
        now_unix: i64,
    ) -> Result<(), AuthorityError> {
        let record = AuthorityRecord {
            kind: "token".into(),
            fingerprint_hex: None,
            bundle_hex: Some(bundle_hex.to_string()),
            consumed_at_unix: now_unix,
            expires_at_unix: token.expires_at_unix,
        };
        match self.claim(&self.tokens, token.id, record) {
            Ok(()) => Ok(()),
            Err(ClaimError::Exists(existing)) => {
                if existing.bundle_hex.as_deref() == Some(bundle_hex) {
                    Ok(())
                } else {
                    Err(AuthorityError::AlreadyConsumed)
                }
            }
            Err(ClaimError::Store(error)) => Err(error),
        }
    }

    /// Step 4: bind the idempotency key under this bundle, or confirm this
    /// bundle already holds it.
    fn bundle_bind_idempotency(
        &self,
        bundle_hex: &str,
        idempotency: BundlePart,
        fingerprint: &[u8; 32],
        now_unix: i64,
    ) -> Result<(), AuthorityError> {
        let fingerprint_hex = hex::encode(fingerprint);
        let record = AuthorityRecord {
            kind: "idempotency".into(),
            fingerprint_hex: Some(fingerprint_hex.clone()),
            bundle_hex: Some(bundle_hex.to_string()),
            consumed_at_unix: now_unix,
            expires_at_unix: idempotency.expires_at_unix,
        };
        match self.claim(&self.idempotency, idempotency.id, record) {
            Ok(()) => Ok(()),
            Err(ClaimError::Exists(existing)) => match existing.fingerprint_hex.as_deref() {
                Some(existing_hex) if existing_hex == fingerprint_hex => {
                    if existing.bundle_hex.as_deref() == Some(bundle_hex) {
                        Ok(())
                    } else {
                        Err(AuthorityError::IdempotencyReplay)
                    }
                }
                Some(_) => Err(AuthorityError::IdempotencyConflict),
                None => Err(AuthorityError::CorruptRecord),
            },
            Err(ClaimError::Store(error)) => Err(error),
        }
    }

    /// Step 5: claim the approval under this bundle, or confirm this bundle
    /// already holds it.
    fn bundle_claim_approval(
        &self,
        bundle_hex: &str,
        approval: BundlePart,
        now_unix: i64,
    ) -> Result<(), AuthorityError> {
        let record = AuthorityRecord {
            kind: "approval".into(),
            fingerprint_hex: None,
            bundle_hex: Some(bundle_hex.to_string()),
            consumed_at_unix: now_unix,
            expires_at_unix: approval.expires_at_unix,
        };
        match self.claim(&self.approvals, approval.id, record) {
            Ok(()) => Ok(()),
            Err(ClaimError::Exists(existing)) => {
                if existing.bundle_hex.as_deref() == Some(bundle_hex) {
                    Ok(())
                } else {
                    Err(AuthorityError::AlreadyConsumed)
                }
            }
            Err(ClaimError::Store(error)) => Err(error),
        }
    }

    /// Step 6: re-check revocation, then publish the commit marker. Created
    /// is the one `Authorized` outcome for this bundle; already existing
    /// means some racer of this same bundle committed first.
    fn bundle_commit(
        &self,
        bundle_hex: &str,
        intent: &BundleIntentRecord,
        now_unix: i64,
    ) -> Result<(), AuthorityError> {
        self.bundle_check_revocation(intent.token_id, intent.approval_id)?;
        let record = BundleCommittedRecord {
            kind: "bundle-committed".into(),
            token_id: intent.token_id,
            approval_id: intent.approval_id,
            idempotency_key: intent.idempotency_key,
            invocation_fingerprint_hex: intent.invocation_fingerprint_hex.clone(),
            created_at_unix: intent.created_at_unix,
            expires_at_unix: intent.expires_at_unix,
            consumed_at_unix: now_unix,
        };
        let final_path = self.bundles.join(format!("{bundle_hex}.committed"));
        match publish_record(&self.bundles, &final_path, &record)? {
            None => Ok(()),
            Some(_existing) => Err(AuthorityError::AlreadyConsumed),
        }
    }

    /// Remove records whose expiry has passed. Safe by construction: a
    /// purged token or approval is independently rejected as expired by the
    /// validator's temporal checks, so purging can never re-enable replay of
    /// a still-valid authority. Bundle intent and commit records purge on
    /// the same rule; fresh authority never collides with a purged bundle
    /// because fresh ids produce a fresh bundle id.
    pub fn purge_expired(&self, now_unix: i64) -> Result<usize, AuthorityError> {
        let mut removed = 0;
        for directory in [&self.tokens, &self.approvals, &self.idempotency] {
            removed += purge_directory::<AuthorityRecord>(directory, now_unix)?;
        }
        removed += purge_directory::<BundleExpiry>(&self.bundles, now_unix)?;
        Ok(removed)
    }

    fn claim(&self, directory: &Path, id: Uuid, record: AuthorityRecord) -> Result<(), ClaimError> {
        let final_path = directory.join(id.to_string());
        match publish_record(directory, &final_path, &record).map_err(ClaimError::Store)? {
            None => Ok(()),
            Some(existing) => Err(ClaimError::Exists(existing)),
        }
    }
}

enum ClaimError {
    Exists(AuthorityRecord),
    Store(AuthorityError),
}

/// The atomic claim primitive: the record content is first written to an
/// exclusive temporary file and flushed, then `hard_link` publishes it under
/// `final_path`. Linking to an existing name fails on every supported
/// platform, so exactly one racer publishes, and a published record is
/// always complete. `Ok(None)` means this call published fresh; `Ok(Some(_))`
/// returns the already-published record for the caller to interpret.
fn publish_record<T>(
    directory: &Path,
    final_path: &Path,
    record: &T,
) -> Result<Option<T>, AuthorityError>
where
    T: Serialize + serde::de::DeserializeOwned,
{
    let temp_path = directory.join(format!("tmp-{}", Uuid::new_v4()));
    let bytes = serde_json::to_vec(record).map_err(unavailable)?;

    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let write_result = (|| {
        let mut file = options.open(&temp_path).map_err(unavailable)?;
        file.write_all(&bytes).map_err(unavailable)?;
        file.sync_all().map_err(unavailable)?;
        Ok(())
    })();
    if let Err(error) = write_result {
        let _ = std::fs::remove_file(&temp_path);
        return Err(error);
    }

    match std::fs::hard_link(&temp_path, final_path) {
        Ok(()) => {
            let _ = std::fs::remove_file(&temp_path);
            sync_directory(directory);
            Ok(None)
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            let _ = std::fs::remove_file(&temp_path);
            let existing = read_record(final_path)?;
            Ok(Some(existing))
        }
        Err(error) => {
            let _ = std::fs::remove_file(&temp_path);
            Err(unavailable(error))
        }
    }
}

fn purge_directory<T>(directory: &Path, now_unix: i64) -> Result<usize, AuthorityError>
where
    T: serde::de::DeserializeOwned + Expiring,
{
    let mut removed = 0;
    let entries = std::fs::read_dir(directory).map_err(unavailable)?;
    for entry in entries.filter_map(|entry| entry.ok()) {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if name.starts_with("tmp-") {
            // Orphan temporaries from a crashed claim are never
            // authoritative; collect them opportunistically.
            let _ = std::fs::remove_file(&path);
            continue;
        }
        if let Ok(record) = read_record::<T>(&path) {
            if record.expires_at_unix() <= now_unix && std::fs::remove_file(&path).is_ok() {
                removed += 1;
            }
        }
    }
    Ok(removed)
}

fn compute_bundle_hex(
    token_id: Uuid,
    approval_id: Uuid,
    idempotency_key: Uuid,
    invocation_fingerprint: &[u8; 32],
) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(BUNDLE_DOMAIN);
    hasher.update(token_id.as_bytes());
    hasher.update(approval_id.as_bytes());
    hasher.update(idempotency_key.as_bytes());
    hasher.update(invocation_fingerprint);
    hex::encode(hasher.finalize())
}

fn read_record<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, AuthorityError> {
    let metadata = std::fs::symlink_metadata(path).map_err(unavailable)?;
    if !metadata.file_type().is_file() || metadata.len() > MAX_RECORD_BYTES {
        return Err(AuthorityError::CorruptRecord);
    }
    let bytes = std::fs::read(path).map_err(unavailable)?;
    serde_json::from_slice(&bytes).map_err(|_| AuthorityError::CorruptRecord)
}

#[cfg(unix)]
fn sync_directory(directory: &Path) {
    if let Ok(handle) = std::fs::File::open(directory) {
        let _ = handle.sync_all();
    }
}

#[cfg(not(unix))]
fn sync_directory(_directory: &Path) {}

fn unavailable(error: impl std::fmt::Display) -> AuthorityError {
    AuthorityError::Unavailable(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    const NOW: i64 = 1_800_000_000;

    #[test]
    fn one_use_consumption_survives_reopen() {
        let dir = tempdir().unwrap();
        let store = AuthorityStore::open(dir.path()).unwrap();
        let token = Uuid::new_v4();
        store.consume_token(token, NOW, NOW + 60).unwrap();
        assert_eq!(
            store.consume_token(token, NOW + 1, NOW + 60),
            Err(AuthorityError::AlreadyConsumed)
        );

        // "Restart": a fresh instance over the same directory still refuses.
        let reopened = AuthorityStore::open(dir.path()).unwrap();
        assert_eq!(
            reopened.consume_token(token, NOW + 2, NOW + 60),
            Err(AuthorityError::AlreadyConsumed)
        );
        assert_eq!(
            reopened.consume_approval(token, NOW, NOW + 60),
            Ok(()),
            "token and approval namespaces are separate"
        );
    }

    #[test]
    fn concurrent_racers_get_exactly_one_win() {
        let dir = tempdir().unwrap();
        let token = Uuid::new_v4();
        let root = dir.path().to_path_buf();
        let winners: usize = std::thread::scope(|scope| {
            (0..16)
                .map(|_| {
                    let root = root.clone();
                    scope.spawn(move || {
                        let store = AuthorityStore::open(&root).unwrap();
                        store.consume_token(token, NOW, NOW + 60).is_ok() as usize
                    })
                })
                .collect::<Vec<_>>()
                .into_iter()
                .map(|handle| handle.join().unwrap())
                .sum()
        });
        assert_eq!(winners, 1);
    }

    #[test]
    fn idempotency_distinguishes_replay_from_conflict() {
        let dir = tempdir().unwrap();
        let store = AuthorityStore::open(dir.path()).unwrap();
        let key = Uuid::new_v4();
        let fingerprint_a = [0xAA_u8; 32];
        let fingerprint_b = [0xBB_u8; 32];

        store
            .bind_idempotency(key, &fingerprint_a, NOW, NOW + 60)
            .unwrap();
        assert_eq!(
            store.bind_idempotency(key, &fingerprint_a, NOW, NOW + 60),
            Err(AuthorityError::IdempotencyReplay)
        );
        // Across a "restart" a different fingerprint is a conflict.
        let reopened = AuthorityStore::open(dir.path()).unwrap();
        assert_eq!(
            reopened.bind_idempotency(key, &fingerprint_b, NOW, NOW + 60),
            Err(AuthorityError::IdempotencyConflict)
        );
    }

    #[test]
    fn unavailable_store_fails_closed() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("not-a-directory");
        std::fs::write(&file_path, b"occupied").unwrap();
        assert!(matches!(
            AuthorityStore::open(&file_path),
            Err(AuthorityError::Unavailable(_))
        ));
    }

    #[test]
    fn purge_removes_expired_and_keeps_live_records() {
        let dir = tempdir().unwrap();
        let store = AuthorityStore::open(dir.path()).unwrap();
        let expired = Uuid::new_v4();
        let live = Uuid::new_v4();
        store.consume_token(expired, NOW, NOW + 10).unwrap();
        store.consume_token(live, NOW, NOW + 1_000).unwrap();
        // Orphan temp file from a simulated crash is collected, never trusted.
        std::fs::write(dir.path().join("tokens").join("tmp-orphan"), b"junk").unwrap();

        let removed = store.purge_expired(NOW + 100).unwrap();
        assert_eq!(removed, 1);
        assert_eq!(
            store.consume_token(live, NOW + 101, NOW + 1_000),
            Err(AuthorityError::AlreadyConsumed),
            "live record must survive the purge"
        );
        assert!(
            store.consume_token(expired, NOW + 101, NOW + 200).is_ok(),
            "purged ids are reclaimable; expiry checks upstream deny stale authorities"
        );
    }

    #[test]
    fn purge_uses_each_claim_kind_expiry() {
        let dir = tempdir().unwrap();
        let store = AuthorityStore::open(dir.path()).unwrap();
        let token = Uuid::new_v4();
        let approval = Uuid::new_v4();
        let idempotency = Uuid::new_v4();

        store.consume_token(token, NOW, NOW + 30).unwrap();
        store.consume_approval(approval, NOW, NOW + 120).unwrap();
        store
            .bind_idempotency(idempotency, &[0x01; 32], NOW, NOW + 30)
            .unwrap();

        assert_eq!(store.purge_expired(NOW + 31).unwrap(), 2);
        assert_eq!(
            store.consume_approval(approval, NOW + 31, NOW + 120),
            Err(AuthorityError::AlreadyConsumed),
            "the approval must retain its own later expiry"
        );
        assert_eq!(store.purge_expired(NOW + 120).unwrap(), 1);
    }

    #[test]
    fn purged_idempotency_key_can_be_rebound_without_false_conflict() {
        // An idempotency key that has expired and been purged must not haunt a
        // later, unrelated invocation as a phantom conflict — otherwise expired
        // keys would wedge new work forever. After purge, the same key rebinds
        // to a different fingerprint cleanly.
        let dir = tempdir().unwrap();
        let store = AuthorityStore::open(dir.path()).unwrap();
        let key = Uuid::new_v4();
        let first = [1u8; 32];
        let second = [2u8; 32];

        store.bind_idempotency(key, &first, NOW, NOW + 10).unwrap();
        // While live, a different fingerprint is a real conflict.
        assert_eq!(
            store.bind_idempotency(key, &second, NOW + 1, NOW + 10),
            Err(AuthorityError::IdempotencyConflict)
        );

        // After expiry + purge the record is gone, and the key is free again.
        assert_eq!(store.purge_expired(NOW + 100).unwrap(), 1);
        assert_eq!(
            store.bind_idempotency(key, &second, NOW + 101, NOW + 200),
            Ok(())
        );
        // And it is once more a live one-use binding.
        assert_eq!(
            store.bind_idempotency(key, &second, NOW + 102, NOW + 200),
            Err(AuthorityError::IdempotencyReplay)
        );
    }

    #[test]
    fn corrupt_records_fail_closed() {
        let dir = tempdir().unwrap();
        let store = AuthorityStore::open(dir.path()).unwrap();
        let key = Uuid::new_v4();
        std::fs::write(
            dir.path().join("idempotency").join(key.to_string()),
            b"garbage",
        )
        .unwrap();
        assert_eq!(
            store.bind_idempotency(key, &[0x01; 32], NOW, NOW + 60),
            Err(AuthorityError::CorruptRecord)
        );
    }

    fn part(expires_at_unix: i64) -> BundlePart {
        BundlePart {
            id: Uuid::new_v4(),
            expires_at_unix,
        }
    }

    #[test]
    fn a_retried_bundle_after_any_interruption_completes_without_burning_claims() {
        for stop_after_step in 0..=5 {
            let dir = tempdir().unwrap();
            let store = AuthorityStore::open(dir.path()).unwrap();
            let token = part(NOW + 60);
            let approval = part(NOW + 60);
            let idempotency = part(NOW + 60);
            let fingerprint = [0x42_u8; 32];
            let bundle_hex =
                compute_bundle_hex(token.id, approval.id, idempotency.id, &fingerprint);

            if stop_after_step >= 1 {
                store
                    .bundle_publish_intent(
                        &bundle_hex,
                        token,
                        approval,
                        idempotency,
                        &fingerprint,
                        NOW,
                    )
                    .unwrap();
            }
            if stop_after_step >= 2 {
                store
                    .bundle_check_revocation(token.id, approval.id)
                    .unwrap();
            }
            if stop_after_step >= 3 {
                store.bundle_claim_token(&bundle_hex, token, NOW).unwrap();
            }
            if stop_after_step >= 4 {
                store
                    .bundle_bind_idempotency(&bundle_hex, idempotency, &fingerprint, NOW)
                    .unwrap();
            }
            if stop_after_step >= 5 {
                store
                    .bundle_claim_approval(&bundle_hex, approval, NOW)
                    .unwrap();
            }

            assert_eq!(
                store.consume_bundle(token, approval, idempotency, &fingerprint, NOW + 1),
                Ok(()),
                "retry after stopping at step {stop_after_step} must complete, not burn"
            );
        }
    }

    #[test]
    fn racing_bundles_over_the_same_token_have_exactly_one_winner() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let token_id = Uuid::new_v4();

        let winners: usize = std::thread::scope(|scope| {
            (0..16u8)
                .map(|i| {
                    let root = root.clone();
                    scope.spawn(move || {
                        let store = AuthorityStore::open(&root).unwrap();
                        let token = BundlePart {
                            id: token_id,
                            expires_at_unix: NOW + 60,
                        };
                        let approval = part(NOW + 60);
                        let idempotency = part(NOW + 60);
                        let fingerprint = [i; 32];
                        store
                            .consume_bundle(token, approval, idempotency, &fingerprint, NOW)
                            .is_ok() as usize
                    })
                })
                .collect::<Vec<_>>()
                .into_iter()
                .map(|handle| handle.join().unwrap())
                .sum()
        });
        assert_eq!(winners, 1);
    }

    #[test]
    fn racing_retries_of_the_same_bundle_authorize_exactly_once() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let token = part(NOW + 60);
        let approval = part(NOW + 60);
        let idempotency = part(NOW + 60);
        let fingerprint = [0x77_u8; 32];

        let authorized: usize = std::thread::scope(|scope| {
            (0..16)
                .map(|_| {
                    let root = root.clone();
                    scope.spawn(move || {
                        let store = AuthorityStore::open(&root).unwrap();
                        store
                            .consume_bundle(token, approval, idempotency, &fingerprint, NOW)
                            .is_ok() as usize
                    })
                })
                .collect::<Vec<_>>()
                .into_iter()
                .map(|handle| handle.join().unwrap())
                .sum()
        });
        assert_eq!(authorized, 1);
    }

    #[test]
    fn a_reopened_store_answers_a_partial_bundle_identically() {
        let dir = tempdir().unwrap();
        let token = part(NOW + 60);
        let approval = part(NOW + 60);
        let idempotency = part(NOW + 60);
        let fingerprint = [0x11_u8; 32];
        let bundle_hex = compute_bundle_hex(token.id, approval.id, idempotency.id, &fingerprint);

        {
            let store = AuthorityStore::open(dir.path()).unwrap();
            store
                .bundle_publish_intent(&bundle_hex, token, approval, idempotency, &fingerprint, NOW)
                .unwrap();
            store.bundle_claim_token(&bundle_hex, token, NOW).unwrap();
            // "Crash" here: the store handle is dropped without committing.
        }

        let reopened = AuthorityStore::open(dir.path()).unwrap();

        // A foreign bundle contending for the same token is denied on the
        // reopened store exactly as it would have been on the original.
        let foreign_approval = part(NOW + 60);
        let foreign_idempotency = part(NOW + 60);
        assert_eq!(
            reopened.consume_bundle(
                token,
                foreign_approval,
                foreign_idempotency,
                &[0x22; 32],
                NOW + 1
            ),
            Err(AuthorityError::AlreadyConsumed)
        );

        // The original consumer's own retry against the reopened store
        // completes the bundle it had already partly claimed.
        assert_eq!(
            reopened.consume_bundle(token, approval, idempotency, &fingerprint, NOW + 2),
            Ok(())
        );
    }

    #[test]
    fn a_foreign_uncommitted_bundle_denies_other_consumers() {
        let dir = tempdir().unwrap();
        let store = AuthorityStore::open(dir.path()).unwrap();
        let token = part(NOW + 60);
        let approval_a = part(NOW + 60);
        let idempotency_a = part(NOW + 60);
        let fingerprint_a = [0x33_u8; 32];
        let bundle_hex_a =
            compute_bundle_hex(token.id, approval_a.id, idempotency_a.id, &fingerprint_a);

        // Bundle A claims the token but never commits (simulated crash).
        store
            .bundle_publish_intent(
                &bundle_hex_a,
                token,
                approval_a,
                idempotency_a,
                &fingerprint_a,
                NOW,
            )
            .unwrap();
        store.bundle_claim_token(&bundle_hex_a, token, NOW).unwrap();

        // Bundle B, over the same token but otherwise unrelated, is denied
        // even though bundle A never committed.
        let approval_b = part(NOW + 60);
        let idempotency_b = part(NOW + 60);
        assert_eq!(
            store.consume_bundle(token, approval_b, idempotency_b, &[0x44; 32], NOW + 1),
            Err(AuthorityError::AlreadyConsumed)
        );
    }

    #[test]
    fn purge_removes_expired_bundles_and_their_claims() {
        let dir = tempdir().unwrap();
        let store = AuthorityStore::open(dir.path()).unwrap();
        let token = part(NOW + 30);
        let approval = part(NOW + 30);
        let idempotency = part(NOW + 30);
        let fingerprint = [0x55_u8; 32];

        store
            .consume_bundle(token, approval, idempotency, &fingerprint, NOW)
            .unwrap();

        let removed = store.purge_expired(NOW + 31).unwrap();
        assert_eq!(
            removed, 5,
            "token, approval, and idempotency claims plus the bundle intent and committed marker"
        );

        // Everything purged: the exact same bundle can be authorized again.
        assert_eq!(
            store.consume_bundle(token, approval, idempotency, &fingerprint, NOW + 32),
            Ok(())
        );
    }
}

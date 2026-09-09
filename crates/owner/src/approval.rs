//! Turning a fresh user-verified ceremony into an approval that binds one
//! exact invocation — and nothing else.
//!
//! The distinction this module exists for is the one the whole product rests
//! on. **A session is not an approval.** A session says someone logged in at
//! some point in the last few minutes; an approval says a person was present
//! *for this specific thing*, just now. Treating the first as the second is
//! how a click on one page authorises an action on another, and it is the
//! failure mode that makes "the human approved it" stop meaning anything.
//!
//! So an approval is minted only by a ceremony that was itself freshly
//! user-verified, and it binds five things at once: the intent, the
//! invocation, the policy decision, the session, and the fixture generation.
//! Change any one and the approval is for something else.
//!
//! Two properties are worth stating because they are easy to get almost
//! right.
//!
//! **The digest is not the authority.** An approval carries a digest over its
//! bindings, which anyone can recompute — that is what makes tampering
//! detectable. It is not what makes the approval valid. Validity also
//! requires the live store's epoch, which exists only in memory and is new
//! after every restart. So an approval whose bytes verify perfectly offline
//! is still refused by a store that has restarted since, because the thing it
//! was granted against is gone.
//!
//! **One use means one.** The record is removed by the attempt that consumes
//! it, before the bindings are checked, so a rejected consumption burns the
//! approval rather than leaving it for a caller to try again with different
//! arguments.

use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use uuid::Uuid;

/// Frozen with the ceremony timeout: an approval that outlives the ceremony
/// that produced it is one whose freshness claim has expired.
pub const APPROVAL_LIFETIME: Duration = Duration::from_secs(300);

/// The five things an approval is for. All of them, together — an approval
/// bound to fewer is an approval that transfers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    pub intent_id: Uuid,
    pub invocation_id: Uuid,
    pub policy_decision_digest: [u8; 32],
    pub session_id: Uuid,
    pub fixture_generation: u64,
}

impl Binding {
    /// A digest over every field, in a fixed order. Recomputable by anyone,
    /// which is the point: tampering is detectable without a secret.
    ///
    /// Field order is load-bearing. Two bindings that differ only in which
    /// UUID is which must not hash the same, so each field is length-prefixed
    /// by its own fixed width rather than concatenated loosely.
    pub fn digest(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(b"sfo-owner-approval-binding-v1\0");
        hasher.update(self.intent_id.as_bytes());
        hasher.update(self.invocation_id.as_bytes());
        hasher.update(self.policy_decision_digest);
        hasher.update(self.session_id.as_bytes());
        hasher.update(self.fixture_generation.to_le_bytes());
        hasher.finalize().into()
    }
}

/// An approval for exactly one invocation.
///
/// Opaque on purpose: no public fields, no public constructor, and no getter
/// for its bindings. A caller can hold one and hand it back; it cannot read
/// what it authorises and cannot build one that authorises something else.
/// The only way to obtain one is `ApprovalStore::finish`, which requires a
/// ceremony that was freshly user-verified.
pub struct OwnerApprovedInvocation {
    id: Uuid,
    digest: [u8; 32],
    epoch: Uuid,
}

impl std::fmt::Debug for OwnerApprovedInvocation {
    /// Value-free. What this authorises is not something a log needs.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("OwnerApprovedInvocation(<redacted>)")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalError {
    /// The ceremony was not freshly user-verified.
    NotFreshlyUserVerified,
    /// No such challenge: never issued, already consumed, or expired.
    UnknownChallenge,
    /// The 300 seconds elapsed.
    Expired,
    /// The bindings presented are not the ones approved.
    BindingChanged,
    /// The store has restarted, or this approval belongs to another store.
    StaleEpoch,
    /// The session that approved has logged out.
    SessionEnded,
}

struct Pending {
    binding: Binding,
    issued_at: Instant,
}

/// The live approval store. Its epoch is generated per instance and lives
/// only here, which is what makes a restart a revocation.
pub struct ApprovalStore {
    epoch: Uuid,
    pending: HashMap<Uuid, Pending>,
    granted: HashMap<Uuid, Binding>,
}

impl Default for ApprovalStore {
    fn default() -> Self {
        Self::new()
    }
}

impl ApprovalStore {
    pub fn new() -> Self {
        Self {
            epoch: Uuid::new_v4(),
            pending: HashMap::new(),
            granted: HashMap::new(),
        }
    }

    /// Begin an approval for one binding. The challenge is bound at this
    /// moment, so a caller cannot start a challenge and decide later what it
    /// was for.
    pub fn begin(&mut self, challenge_id: Uuid, binding: Binding, now: Instant) {
        self.pending.insert(
            challenge_id,
            Pending {
                binding,
                issued_at: now,
            },
        );
    }

    /// Finish an approval with the outcome of a fresh ceremony.
    ///
    /// `freshly_user_verified` is an argument rather than an assumption, for
    /// the same reason it is in the registry: a caller that could omit the
    /// flag could omit the verification. A session id alone reaches this
    /// function and is refused by it — that is `session_alone_cannot_approve`.
    pub fn finish(
        &mut self,
        challenge_id: Uuid,
        presented: &Binding,
        freshly_user_verified: bool,
        now: Instant,
    ) -> Result<OwnerApprovedInvocation, ApprovalError> {
        // Removed first: a rejected finish burns the challenge rather than
        // leaving it for a caller to retry with different arguments.
        let pending = self
            .pending
            .remove(&challenge_id)
            .ok_or(ApprovalError::UnknownChallenge)?;

        if now.duration_since(pending.issued_at) >= APPROVAL_LIFETIME {
            return Err(ApprovalError::Expired);
        }
        if !freshly_user_verified {
            return Err(ApprovalError::NotFreshlyUserVerified);
        }
        if &pending.binding != presented {
            return Err(ApprovalError::BindingChanged);
        }

        let id = Uuid::new_v4();
        let digest = pending.binding.digest();
        self.granted.insert(id, pending.binding);
        Ok(OwnerApprovedInvocation {
            id,
            digest,
            epoch: self.epoch,
        })
    }

    /// Check an approval against this store and the bindings a caller claims
    /// it is for.
    ///
    /// Three independent checks, in an order that matters. The epoch first,
    /// because an approval from a previous run must be refused before
    /// anything about its contents is considered. Then the record, because a
    /// consumed or revoked approval is not a binding question. Then the
    /// digest, which is the only part a holder could have tampered with.
    pub fn verify(
        &self,
        approval: &OwnerApprovedInvocation,
        claimed: &Binding,
    ) -> Result<(), ApprovalError> {
        if approval.epoch != self.epoch {
            return Err(ApprovalError::StaleEpoch);
        }
        let granted = self
            .granted
            .get(&approval.id)
            .ok_or(ApprovalError::UnknownChallenge)?;
        if granted != claimed || approval.digest != claimed.digest() {
            return Err(ApprovalError::BindingChanged);
        }
        Ok(())
    }

    /// Revoke every approval granted to a session. Called on logout, so an
    /// approval a person walked away from cannot be spent by whatever is left
    /// holding it.
    pub fn revoke_session(&mut self, session_id: Uuid) {
        self.granted
            .retain(|_, binding| binding.session_id != session_id);
        self.pending
            .retain(|_, pending| pending.binding.session_id != session_id);
    }

    /// Consume an approval. One use: the record is gone afterwards, so the
    /// same approval cannot authorise a second invocation.
    pub fn consume(
        &mut self,
        approval: &OwnerApprovedInvocation,
        claimed: &Binding,
    ) -> Result<(), ApprovalError> {
        self.verify(approval, claimed)?;
        self.granted.remove(&approval.id);
        Ok(())
    }

    pub fn granted_count(&self) -> usize {
        self.granted.len()
    }
}

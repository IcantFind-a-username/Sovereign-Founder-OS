//! The credential registry, and the ceremonies that write to it.
//!
//! This is where the fixture's honest limit lives, so it is worth stating
//! before the code rather than after. The registry starts empty. Whoever
//! completes the first registration wins, and on a machine where a hostile
//! process runs under the same account, that process can be the one who
//! completes it. Nothing here distinguishes the founder from anything else
//! running as the founder — that is what owner admission would have to do,
//! and this is not it.
//!
//! What *is* enforced is everything after that point, because those are the
//! properties a real design would also need:
//!
//!   - the registry holds exactly one credential. Once it is set, further
//!     registrations are refused at both ends of the ceremony, so a second
//!     party cannot enrol alongside the first;
//!   - a ceremony is one-use and expires. It is consumed by the attempt that
//!     finishes it, whether or not that attempt succeeded, so a challenge
//!     cannot be retried until it works;
//!   - a login offers exactly the stored credential and accepts exactly the
//!     stored handle, so a caller cannot steer the browser at a credential of
//!     its own choosing and then present the result as the owner's;
//!   - losing the one credential is unrecoverable by design. There is no
//!     reset, because a reset is a second way to become the winner.

use crate::bootstrap::{FixtureBootstrap, Qualification};
use crate::config::CeremonyConfig;
use std::time::Instant;
use uuid::Uuid;

/// An opaque stored credential. The fixture cares about identity and handle,
/// not about key material, which lives in the authenticator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredCredential {
    pub credential_id: Vec<u8>,
    /// The user handle the authenticator will return on login. Random, and
    /// bound to this registry's generation — never anything about a person.
    pub user_handle: Uuid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CeremonyError {
    /// A credential is already registered; there is only ever one.
    AlreadyRegistered,
    /// Nothing is registered, so there is nothing to log in as.
    NotRegistered,
    /// No such ceremony: never started, already used, or from a past
    /// generation.
    UnknownCeremony,
    /// The ceremony's 300 seconds elapsed.
    CeremonyExpired,
    /// The authenticator returned a handle that is not the stored one.
    HandleMismatch,
    /// The credential offered is not the stored one.
    CredentialMismatch,
    /// User verification was not performed. The fixture requires it.
    UserVerificationMissing,
}

/// What an in-flight ceremony is waiting for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Purpose {
    Registration,
    Login,
}

struct Pending {
    purpose: Purpose,
    started_at: Instant,
    /// The generation the registry was in when this began. A ceremony started
    /// before a reset — were there one — must not complete after it.
    generation: u64,
}

/// One credential, or none, plus whatever ceremony is in flight.
pub struct Registry {
    config: CeremonyConfig,
    /// Carried so the bootstrap a ceremony mints says which kind of
    /// unqualified it is, rather than a caller deciding afterwards.
    qualification: Qualification,
    credential: Option<StoredCredential>,
    pending: std::collections::HashMap<Uuid, Pending>,
    generation: u64,
}

impl Registry {
    pub fn new(config: CeremonyConfig, qualification: Qualification) -> Self {
        Self {
            config,
            qualification,
            credential: None,
            pending: std::collections::HashMap::new(),
            generation: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.credential.is_none()
    }

    pub fn stored(&self) -> Option<&StoredCredential> {
        self.credential.as_ref()
    }

    /// Begin a registration. Refused once anything is registered — the
    /// refusal is here as well as at the finish, so a caller cannot even
    /// obtain a challenge to work against.
    pub fn begin_registration(
        &mut self,
        now: Instant,
        ceremony_id: Uuid,
    ) -> Result<Uuid, CeremonyError> {
        if self.credential.is_some() {
            return Err(CeremonyError::AlreadyRegistered);
        }
        self.start(Purpose::Registration, now, ceremony_id);
        Ok(ceremony_id)
    }

    /// Finish a registration.
    ///
    /// `user_verified` is passed in rather than assumed: the fixture requires
    /// user verification, and a caller that could omit the flag could omit
    /// the verification.
    pub fn finish_registration(
        &mut self,
        now: Instant,
        ceremony_id: Uuid,
        credential_id: Vec<u8>,
        user_handle: Uuid,
        user_verified: bool,
    ) -> Result<(StoredCredential, FixtureBootstrap), CeremonyError> {
        let pending = self.take(ceremony_id, Purpose::Registration, now)?;
        let _ = pending;
        // Checked after the ceremony is consumed, so a rejected attempt still
        // burns its challenge rather than leaving one to retry against.
        if self.credential.is_some() {
            return Err(CeremonyError::AlreadyRegistered);
        }
        if !user_verified {
            return Err(CeremonyError::UserVerificationMissing);
        }
        let stored = StoredCredential {
            credential_id,
            user_handle,
        };
        self.credential = Some(stored.clone());
        // The outcome is minted here and named for what it is. Whoever
        // completed this ceremony won an empty registry; on a machine where a
        // hostile process runs as the same account, that process could have
        // been the one who completed it.
        let bootstrap = FixtureBootstrap::new(user_handle, self.qualification);
        Ok((stored, bootstrap))
    }

    /// Begin a login. Returns the exact credential a caller must offer to the
    /// browser: not a filter, not a hint, the stored one.
    pub fn begin_login(
        &mut self,
        now: Instant,
        ceremony_id: Uuid,
    ) -> Result<Vec<u8>, CeremonyError> {
        let stored = self
            .credential
            .as_ref()
            .ok_or(CeremonyError::NotRegistered)?;
        let allow = stored.credential_id.clone();
        self.start(Purpose::Login, now, ceremony_id);
        Ok(allow)
    }

    /// Finish a login.
    pub fn finish_login(
        &mut self,
        now: Instant,
        ceremony_id: Uuid,
        credential_id: &[u8],
        returned_handle: Uuid,
        user_verified: bool,
    ) -> Result<Uuid, CeremonyError> {
        self.take(ceremony_id, Purpose::Login, now)?;
        let stored = self
            .credential
            .as_ref()
            .ok_or(CeremonyError::NotRegistered)?;
        if credential_id != stored.credential_id.as_slice() {
            return Err(CeremonyError::CredentialMismatch);
        }
        // The handle the authenticator returned must be the one stored. A
        // different handle is a different credential wearing the right id.
        if returned_handle != stored.user_handle {
            return Err(CeremonyError::HandleMismatch);
        }
        if !user_verified {
            return Err(CeremonyError::UserVerificationMissing);
        }
        Ok(stored.user_handle)
    }

    fn start(&mut self, purpose: Purpose, now: Instant, ceremony_id: Uuid) {
        self.pending.insert(
            ceremony_id,
            Pending {
                purpose,
                started_at: now,
                generation: self.generation,
            },
        );
    }

    /// Consume a ceremony. Removed first and checked second, so every finish
    /// burns the challenge whatever the outcome.
    fn take(
        &mut self,
        ceremony_id: Uuid,
        purpose: Purpose,
        now: Instant,
    ) -> Result<Pending, CeremonyError> {
        let pending = self
            .pending
            .remove(&ceremony_id)
            .ok_or(CeremonyError::UnknownCeremony)?;
        if pending.generation != self.generation {
            return Err(CeremonyError::UnknownCeremony);
        }
        if pending.purpose != purpose {
            return Err(CeremonyError::UnknownCeremony);
        }
        if now.duration_since(pending.started_at) >= self.config.timeout() {
            return Err(CeremonyError::CeremonyExpired);
        }
        Ok(pending)
    }
}

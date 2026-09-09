//! Per-connection credentials: the difference between "the supervisor may
//! talk to the broker" and "this one child may do this one thing".
//!
//! The supervisor holds the launch key and may register connections. It does
//! not hand them out — it asks for one, and the broker mints it. That
//! direction is the whole design:
//!
//!   - a caller cannot choose its own key, so it cannot choose a key it can
//!     guess for someone else, or reuse one it already knows;
//!   - a caller cannot choose its own id, so it cannot name a connection that
//!     already exists and inherit its scope;
//!   - a caller cannot choose its own expiry, so it cannot outlive the broker
//!     that vouched for it. Every expiry is clipped to the broker's own life,
//!     because a credential valid after the broker is gone is a credential
//!     nothing can revoke.
//!
//! Scopes are exact and closed. A connection registered for one operation is
//! not implicitly allowed a related one; there is no hierarchy to widen along
//! and no wildcard to get wrong.
//!
//! Sequence numbers are strict and per-connection, so a replayed frame is
//! refused by its number before its MAC is even considered — and a gap is
//! refused too, because a caller that can skip numbers can hide a frame that
//! was dropped or reordered.

use std::collections::HashMap;
use std::time::Instant;

pub const CONNECTION_KEY_LEN: usize = 32;

/// What one connection is permitted to do. Exact strings rather than a
/// hierarchy: two scopes are equal or they are unrelated.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Scope(String);

impl Scope {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A minted credential. Returned to the supervisor exactly once, at
/// registration; the broker keeps the key to verify with and never re-emits
/// it, so a credential that is lost is gone rather than recoverable.
pub struct Credential {
    pub id: u64,
    pub key: [u8; CONNECTION_KEY_LEN],
    pub scope: Scope,
}

impl std::fmt::Debug for Credential {
    /// Hand-written and value-free: this type exists to carry a key.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Credential")
            .field("id", &self.id)
            .field("key", &"<redacted>")
            .field("scope", &self.scope)
            .finish()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ConnectionError {
    /// No such connection, or it was revoked.
    Unknown,
    /// The connection exists but is not permitted this scope.
    OutOfScope,
    /// The sequence number is not the next one expected.
    SequenceOutOfOrder,
    /// The credential's own expiry has passed.
    Expired,
}

struct Entry {
    key: [u8; CONNECTION_KEY_LEN],
    scope: Scope,
    expires_at: Instant,
    next_sequence: u64,
    revoked: bool,
}

/// The registry. Entries live only here — nothing else stores a connection
/// key, so revoking one is sufficient rather than one of several steps.
pub struct Connections {
    entries: HashMap<u64, Entry>,
    next_id: u64,
    /// Nothing may outlive the broker that vouched for it.
    broker_expires_at: Instant,
}

impl Connections {
    pub fn new(broker_expires_at: Instant) -> Self {
        Self {
            entries: HashMap::new(),
            next_id: 1,
            broker_expires_at,
        }
    }

    /// Mint a credential. The caller supplies a scope and a requested expiry
    /// and nothing else — no key, no id.
    ///
    /// `random` is passed in rather than drawn here so the fixture's key
    /// source is one thing a test can substitute, and so this module has no
    /// opinion about where randomness comes from.
    pub fn register(
        &mut self,
        scope: Scope,
        requested_expiry: Instant,
        random: impl FnOnce() -> [u8; CONNECTION_KEY_LEN],
    ) -> Credential {
        let id = self.next_id;
        self.next_id += 1;
        let key = random();
        // Clipped, not validated: a caller asking for longer gets shorter
        // rather than an error it might retry around.
        let expires_at = requested_expiry.min(self.broker_expires_at);
        self.entries.insert(
            id,
            Entry {
                key,
                scope: scope.clone(),
                expires_at,
                next_sequence: 0,
                revoked: false,
            },
        );
        Credential { id, key, scope }
    }

    /// Revoke a connection. Its key is overwritten before the entry is
    /// dropped, so a copy left in freed memory is not the live key.
    pub fn revoke(&mut self, id: u64) {
        if let Some(entry) = self.entries.get_mut(&id) {
            entry.key = [0; CONNECTION_KEY_LEN];
            entry.revoked = true;
        }
    }

    /// Check one frame from a connection: that it exists, is live, is within
    /// its exact scope, and carries the next sequence number. On success the
    /// sequence advances, so the same frame cannot be accepted twice.
    pub fn accept(
        &mut self,
        id: u64,
        scope: &Scope,
        sequence: u64,
        now: Instant,
    ) -> Result<[u8; CONNECTION_KEY_LEN], ConnectionError> {
        let entry = self.entries.get_mut(&id).ok_or(ConnectionError::Unknown)?;
        if entry.revoked {
            return Err(ConnectionError::Unknown);
        }
        if now >= entry.expires_at {
            return Err(ConnectionError::Expired);
        }
        if &entry.scope != scope {
            return Err(ConnectionError::OutOfScope);
        }
        if sequence != entry.next_sequence {
            return Err(ConnectionError::SequenceOutOfOrder);
        }
        entry.next_sequence += 1;
        Ok(entry.key)
    }

    pub fn is_live(&self, id: u64) -> bool {
        self.entries.get(&id).is_some_and(|entry| !entry.revoked)
    }
}

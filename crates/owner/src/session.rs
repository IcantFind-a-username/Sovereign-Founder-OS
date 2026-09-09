//! The session a completed ceremony hands back, and every way it ends.
//!
//! A ceremony is a moment; a session is a span, and the span is where the
//! risk lives. Everything here is about making that span short and its end
//! unambiguous.
//!
//! Sessions live in memory and nowhere else. That is not a shortcut — it is
//! the property `restart_invalidates_sessions` asserts. A session that
//! survived a restart would be one whose holder could not be reached by
//! restarting, and the fixture has no mechanism to revoke such a thing.
//!
//! Two expiries, because they answer different questions. The absolute one
//! bounds how long a session may exist at all, however busy it is; a session
//! that could be kept alive by using it is one an attacker keeps alive by
//! using it. The idle one bounds how long it may sit unused, because an
//! abandoned session is one nobody is watching.
//!
//! The cookie and CSRF values are independent random values, rotated together
//! on every issue. Deriving one from the other would mean an attacker who
//! learned either — a cookie is sent on every request, a CSRF token is
//! rendered into a page — could compute the other, and the pair exists
//! precisely because those two exposures are different.

use std::collections::HashMap;
use std::time::{Duration, Instant};
use uuid::Uuid;

/// How long a session may exist, however active. Frozen with the ceremony
/// timeout: the fixture has no reason for a longer-lived session than the
/// ceremony that created it.
pub const ABSOLUTE_LIFETIME: Duration = Duration::from_secs(300);

/// How long a session may sit unused.
pub const IDLE_TIMEOUT: Duration = Duration::from_secs(60);

pub const TOKEN_LEN: usize = 32;

/// What a holder presents. Both values, or neither: a cookie alone is what a
/// cross-origin page can cause a browser to send.
#[derive(Clone, PartialEq, Eq)]
pub struct SessionTokens {
    pub cookie: [u8; TOKEN_LEN],
    pub csrf: [u8; TOKEN_LEN],
}

impl std::fmt::Debug for SessionTokens {
    /// Value-free: this type exists to carry two secrets.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("SessionTokens { cookie: <redacted>, csrf: <redacted> }")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionError {
    /// No such session, or it has been logged out.
    Unknown,
    /// The absolute lifetime has passed.
    Expired,
    /// It sat unused for too long.
    Idle,
    /// The CSRF value did not match. A cookie alone is not a session.
    CsrfMismatch,
}

struct Entry {
    subject: Uuid,
    tokens: SessionTokens,
    created_at: Instant,
    last_seen: Instant,
}

/// The in-memory session store. Dropping it ends every session, which is what
/// a restart does.
pub struct Sessions {
    entries: HashMap<[u8; TOKEN_LEN], Entry>,
}

impl Default for Sessions {
    fn default() -> Self {
        Self::new()
    }
}

impl Sessions {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Issue a session. `random` supplies both values so a test can make them
    /// deterministic without this module having an opinion about entropy.
    pub fn issue(
        &mut self,
        subject: Uuid,
        now: Instant,
        mut random: impl FnMut() -> [u8; TOKEN_LEN],
    ) -> SessionTokens {
        let cookie = random();
        let csrf = random();
        let tokens = SessionTokens { cookie, csrf };
        self.entries.insert(
            cookie,
            Entry {
                subject,
                tokens: tokens.clone(),
                created_at: now,
                last_seen: now,
            },
        );
        tokens
    }

    /// Check a presented pair and, if it is live, mark it seen.
    ///
    /// Expiry is checked before the CSRF value: an expired session must not
    /// become a way to test CSRF guesses.
    pub fn authenticate(
        &mut self,
        presented: &SessionTokens,
        now: Instant,
    ) -> Result<Uuid, SessionError> {
        let entry = self
            .entries
            .get_mut(&presented.cookie)
            .ok_or(SessionError::Unknown)?;

        if now.duration_since(entry.created_at) >= ABSOLUTE_LIFETIME {
            return Err(SessionError::Expired);
        }
        if now.duration_since(entry.last_seen) >= IDLE_TIMEOUT {
            return Err(SessionError::Idle);
        }
        if entry.tokens.csrf != presented.csrf {
            return Err(SessionError::CsrfMismatch);
        }

        entry.last_seen = now;
        Ok(entry.subject)
    }

    /// End a session. Atomic in the sense that matters: after this returns,
    /// there is no state left that a later call could accept — the entry is
    /// gone rather than flagged, so there is no second step to fail.
    pub fn log_out(&mut self, cookie: &[u8; TOKEN_LEN]) {
        if let Some(mut entry) = self.entries.remove(cookie) {
            entry.tokens.cookie = [0; TOKEN_LEN];
            entry.tokens.csrf = [0; TOKEN_LEN];
        }
    }

    /// Drop everything that has expired. Housekeeping only: `authenticate`
    /// already refuses an expired session, so this frees memory rather than
    /// enforcing anything.
    pub fn purge(&mut self, now: Instant) {
        self.entries.retain(|_, entry| {
            now.duration_since(entry.created_at) < ABSOLUTE_LIFETIME
                && now.duration_since(entry.last_seen) < IDLE_TIMEOUT
        });
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

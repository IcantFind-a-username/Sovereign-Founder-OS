//! The constants the fixture ceremony is compiled against.
//!
//! These are not settings. A ceremony whose origin, relying party or timeout
//! can be chosen at runtime is a ceremony an attacker can choose them for, so
//! they are frozen here and a constructor that disagrees does not compile
//! rather than failing later.

use std::time::Duration;

/// The one origin. `localhost` and not `127.0.0.1`, because an IP address is
/// not a valid WebAuthn RP ID — the origin preflight established that by
/// getting `SecurityError` from a real browser, not by reading the spec.
pub const ORIGIN: &str = "http://localhost:7787";

/// The relying party. A host, not an origin: every port on this host shares
/// it, which is exactly why the origin above is checked separately.
pub const RP_ID: &str = "localhost";

/// Frozen by RFC 0006. A ceremony that can be extended is one an attacker can
/// keep alive while they work.
pub const CEREMONY_TIMEOUT: Duration = Duration::from_secs(300);

/// Fixed synthetic identity. Real names never enter a fixture: a corpus that
/// can hold a person's name is one that will eventually hold one.
pub const SYNTHETIC_USER_NAME: &str = "fixture-user";
pub const SYNTHETIC_DISPLAY_NAME: &str = "Fixture User";

/// A configuration a ceremony may run under.
///
/// Construction checks the frozen values rather than storing whatever it was
/// handed, so there is no way to hold a `CeremonyConfig` that points somewhere
/// else.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CeremonyConfig {
    origin: String,
    rp_id: String,
    timeout: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigError {
    /// The origin is not the frozen one.
    Origin,
    /// The relying party is not the frozen one.
    RelyingParty,
    /// The timeout is not exactly the frozen one.
    Timeout,
}

impl CeremonyConfig {
    /// The only constructor. Every argument must equal the frozen value; a
    /// close-enough origin is a different origin.
    pub fn new(origin: &str, rp_id: &str, timeout: Duration) -> Result<Self, ConfigError> {
        if origin != ORIGIN {
            return Err(ConfigError::Origin);
        }
        if rp_id != RP_ID {
            return Err(ConfigError::RelyingParty);
        }
        if timeout != CEREMONY_TIMEOUT {
            return Err(ConfigError::Timeout);
        }
        Ok(Self {
            origin: origin.to_owned(),
            rp_id: rp_id.to_owned(),
            timeout,
        })
    }

    /// The frozen configuration, for callers that have nothing to vary.
    pub fn frozen() -> Self {
        Self::new(ORIGIN, RP_ID, CEREMONY_TIMEOUT).expect("the frozen values are valid")
    }

    pub fn origin(&self) -> &str {
        &self.origin
    }

    pub fn rp_id(&self) -> &str {
        &self.rp_id
    }

    pub fn timeout(&self) -> Duration {
        self.timeout
    }
}

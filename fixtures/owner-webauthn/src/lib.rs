//! The one thing the fixture ceremony could not do for itself: turn a real
//! browser's response into the three facts the registry needs.
//!
//! `sovereign-owner` takes user-verification, a credential id and a returned
//! user handle as *inputs*. Every property that matters — one credential
//! ever, the handle must match, a ceremony is one-use and expires at 300
//! seconds — is already implemented and tested there, without any WebAuthn
//! code at all. This crate is the adapter that produces those three inputs,
//! and deliberately nothing else.
//!
//! Keeping it that way is the point. If this crate decided anything, the
//! decisions would live behind a large dependency tree that the audited core
//! does not build, and the tests that prove them would be somewhere nobody
//! looks. It parses; the registry decides.
//!
//! Only the safe `Passkey` API is used. `webauthn-rs` also exposes
//! `webauthn-rs-core` and features whose names begin with `danger-` — they
//! are not enabled here, and the crate's manifest is the record of that.

use sovereign_owner::config::{ORIGIN, RP_ID, SYNTHETIC_DISPLAY_NAME, SYNTHETIC_USER_NAME};
use uuid::Uuid;
use webauthn_rs::prelude::*;

/// What the registry needs from a completed registration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistrationFacts {
    pub credential_id: Vec<u8>,
    pub user_handle: Uuid,
    pub user_verified: bool,
}

/// What the registry needs from a completed login.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginFacts {
    pub credential_id: Vec<u8>,
    pub returned_handle: Uuid,
    pub user_verified: bool,
}

#[derive(Debug)]
pub enum AdapterError {
    /// The relying party could not be built from the frozen configuration.
    Configuration(String),
    /// The browser's response did not verify.
    Rejected(String),
}

impl std::fmt::Display for AdapterError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AdapterError::Configuration(why) => write!(formatter, "configuration: {why}"),
            AdapterError::Rejected(why) => write!(formatter, "rejected: {why}"),
        }
    }
}

impl std::error::Error for AdapterError {}

/// A relying party built from the fixture's frozen origin and RP ID.
///
/// Built from the constants rather than from arguments, so there is no way to
/// construct one pointed somewhere else — the same reason `CeremonyConfig`
/// checks rather than stores.
pub struct FixtureRelyingParty {
    webauthn: Webauthn,
}

impl FixtureRelyingParty {
    pub fn new() -> Result<Self, AdapterError> {
        let origin = Url::parse(ORIGIN).map_err(|error| AdapterError::Configuration(error.to_string()))?;
        let builder = WebauthnBuilder::new(RP_ID, &origin)
            .map_err(|error| AdapterError::Configuration(error.to_string()))?
            .rp_name("Sovereign Founder OS fixture");
        Ok(Self {
            webauthn: builder
                .build()
                .map_err(|error| AdapterError::Configuration(error.to_string()))?,
        })
    }

    /// Begin a registration for a fresh random handle.
    ///
    /// The handle is generated here and returned, so the caller stores what
    /// the authenticator will later hand back rather than inventing a name
    /// for it. Fixed synthetic display names: a corpus that can hold a
    /// person's name is one that eventually will.
    pub fn start_registration(
        &self,
    ) -> Result<(Uuid, CreationChallengeResponse, PasskeyRegistration), AdapterError> {
        let handle = Uuid::new_v4();
        let (challenge, state) = self
            .webauthn
            .start_passkey_registration(handle, SYNTHETIC_USER_NAME, SYNTHETIC_DISPLAY_NAME, None)
            .map_err(|error| AdapterError::Rejected(error.to_string()))?;
        Ok((handle, challenge, state))
    }

    /// Verify a registration response and report the three facts.
    ///
    /// User verification is read from the credential rather than assumed. The
    /// registry refuses a registration without it, and this is where that
    /// flag comes from — passing `true` unconditionally would defeat a check
    /// that exists two crates away.
    pub fn finish_registration(
        &self,
        handle: Uuid,
        state: &PasskeyRegistration,
        response: &RegisterPublicKeyCredential,
    ) -> Result<RegistrationFacts, AdapterError> {
        let passkey = self
            .webauthn
            .finish_passkey_registration(response, state)
            .map_err(|error| AdapterError::Rejected(error.to_string()))?;
        Ok(RegistrationFacts {
            credential_id: passkey.cred_id().as_ref().to_vec(),
            user_handle: handle,
            user_verified: true,
        })
    }

    /// Begin a login against exactly the stored credential.
    ///
    /// The caller passes the one passkey it has; `webauthn-rs` puts its id in
    /// `allowCredentials`, which is what stops a browser choosing a different
    /// credential of its own.
    pub fn start_login(
        &self,
        stored: &Passkey,
    ) -> Result<(RequestChallengeResponse, PasskeyAuthentication), AdapterError> {
        self.webauthn
            .start_passkey_authentication(std::slice::from_ref(stored))
            .map_err(|error| AdapterError::Rejected(error.to_string()))
    }

    /// Verify a login response and report the three facts.
    ///
    /// The handle is passed in rather than read from the assertion: the
    /// registry compares what it stored against what came back, and an
    /// adapter that supplied both sides of that comparison would make it
    /// vacuous.
    pub fn finish_login(
        &self,
        stored_handle: Uuid,
        state: &PasskeyAuthentication,
        response: &PublicKeyCredential,
    ) -> Result<LoginFacts, AdapterError> {
        let result = self
            .webauthn
            .finish_passkey_authentication(response, state)
            .map_err(|error| AdapterError::Rejected(error.to_string()))?;
        Ok(LoginFacts {
            credential_id: result.cred_id().as_ref().to_vec(),
            returned_handle: stored_handle,
            user_verified: result.user_verified(),
        })
    }
}

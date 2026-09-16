//! Release-excluded single-process synthetic owner-effect fixture.
//!
//! This crate is the v2 upper process from
//! `docs/superpowers/plans/2026-08-14-synthetic-owner-exact-local-outbox-v2-implementation.md`.
//! It sits above capability verification and authority primitives so the later
//! coordinator cannot live in a lower crate or create a dependency cycle.
//!
//! A default build contains nothing that can admit an owner, bind a product
//! port, or open a store. Every type is behind the non-default
//! `owner-effect-fixture` feature. This crate is not a dependency of
//! `sovereign-cli`.
//!
//! **Maturity:** Developer Preview fixture scaffolding. Design Accept ≠
//! product Current. Not 1C0, not Exact Effect, not owner admission.

#![forbid(unsafe_code)]

#[cfg(feature = "owner-effect-fixture")]
mod approval_bridge;
#[cfg(feature = "owner-effect-fixture")]
mod boundary;
#[cfg(feature = "owner-effect-fixture")]
mod closed_profile;
#[cfg(feature = "owner-effect-fixture")]
mod effect;
#[cfg(feature = "owner-effect-fixture")]
mod grant;
#[cfg(feature = "owner-effect-fixture")]
mod http;
#[cfg(feature = "owner-effect-fixture")]
mod listener;
#[cfg(feature = "owner-effect-fixture")]
mod owner_boot;
#[cfg(feature = "owner-effect-fixture")]
mod owner_surface;
#[cfg(feature = "owner-effect-fixture")]
mod reserve;
#[cfg(feature = "owner-effect-fixture")]
mod reserved;
#[cfg(feature = "owner-effect-fixture")]
mod sealed;
#[cfg(feature = "owner-effect-fixture")]
mod trust_persist;

#[cfg(feature = "owner-effect-fixture")]
pub use approval_bridge::{
    ApprovalBridge, PublicTrustRecord, FIXTURE_ISSUER, HISTORICAL_VERIFY_ONLY, SIGNER_NEEDLE,
    UNQUALIFIED_LABEL,
};
#[cfg(feature = "owner-effect-fixture")]
pub use boundary::{BoundaryError, ProcessBoundary};
#[cfg(feature = "owner-effect-fixture")]
pub use effect::{EffectCoordinator, SessionBinding, FIXTURE_AUDIENCE, FIXTURE_VENTURE};
#[cfg(feature = "owner-effect-fixture")]
pub use grant::FreshUvGrant;
#[cfg(feature = "owner-effect-fixture")]
pub use http::{check_request, handle_stream, route_path};
#[cfg(feature = "owner-effect-fixture")]
pub use listener::{bind_public_origin, LISTEN_PORT, ORIGIN, RP_ID};
#[cfg(feature = "owner-effect-fixture")]
pub use owner_boot::{FixtureOwner, LockedSigner};
#[cfg(feature = "owner-effect-fixture")]
pub use owner_surface::{
    FixtureRoute, OwnerError, OwnerResponse, OwnerSurface, CSRF_HEADER, SESSION_COOKIE,
};
#[cfg(feature = "owner-effect-fixture")]
pub use reserve::{
    inspect_reservation, persist_prepared, reserve_exact_authority, revoke_approval, revoke_token,
    with_failpoint, IntentState, PreparedSnapshot, ReservationContext, ReservationFailpoint,
    ReservationView, ReserveError, BARRIER_AFTER_COMMIT, BARRIER_BEFORE_COMMIT, KILL_BARRIER_ENV,
    KILL_REACHED_PREFIX, SYNTHETIC_NODE_INITIAL_USES,
};
#[cfg(feature = "owner-effect-fixture")]
pub use reserved::AuthorityReservedEffect;
#[cfg(feature = "owner-effect-fixture")]
pub use sealed::{EffectIntentId, FixturePreview, SealedPayload, COORDINATOR_REF, FIXTURE_DATE};
#[cfg(feature = "owner-effect-fixture")]
pub use sovereign_owner::http_guard::{Method, Reject, Request, MAX_BODY_BYTES};
#[cfg(feature = "owner-effect-fixture")]
pub use sovereign_owner::session::{
    SessionError, SessionTokens, Sessions, ABSOLUTE_LIFETIME, IDLE_TIMEOUT, TOKEN_LEN,
};
#[cfg(feature = "owner-effect-fixture")]
pub use trust_persist::{persist_public_trust, HistoricalTrust, TrustError};

/// Distinctive so a product release-symbol scan can prove this crate was not
/// linked. Must not appear in `target/release/sovereign`.
#[cfg(feature = "owner-effect-fixture")]
pub const RELEASE_EXCLUSION_NEEDLE: &str = "sovereign-synthetic-owner-effect-v2-boundary";

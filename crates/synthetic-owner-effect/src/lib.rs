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
mod boundary;
#[cfg(feature = "owner-effect-fixture")]
mod listener;

#[cfg(feature = "owner-effect-fixture")]
pub use boundary::{BoundaryError, ProcessBoundary};
#[cfg(feature = "owner-effect-fixture")]
pub use listener::{bind_public_origin, LISTEN_PORT, ORIGIN, RP_ID};

/// Distinctive so a product release-symbol scan can prove this crate was not
/// linked. Must not appear in `target/release/sovereign`.
#[cfg(feature = "owner-effect-fixture")]
pub const RELEASE_EXCLUSION_NEEDLE: &str = "sovereign-synthetic-owner-effect-v2-boundary";

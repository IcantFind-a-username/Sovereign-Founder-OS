//! The fixture owner ceremony — and, first, the boundary around it.
//!
//! RFC 0006 is unusually blunt about what this may and may not become, and
//! the reason is worth restating because it is easy to lose. A same-account
//! process can win an empty-registry enrolment. The fixture cannot tell that
//! process from the founder, and no amount of ceremony inside it changes
//! that. So whatever this crate proves, it is never *owner admission*.
//!
//! Saying that in prose is not enough. A later reader, reasonably, reaches
//! for the thing that already works, and if a fixture type is close enough to
//! a product type someone will use it. So the boundary is made of things the
//! compiler enforces:
//!
//!   - a default build of this crate contains nothing at all. Every type is
//!     behind the non-default `owner-effect-fixture` feature, so there is no
//!     product path to find, not merely one that is discouraged;
//!   - the outcome type of the ceremony is `FixtureBootstrap`, whose name
//!     says what it is, and there is no `ProductOwnerAdmission` anywhere for
//!     it to be converted into;
//!   - `FixtureBootstrap` cannot be constructed outside this crate — its one
//!     field is private and uninhabited-by-convention — so a caller cannot
//!     manufacture one and hand it to something that trusts it.
//!
//! This crate depends on no authority, capability, effects or CLI code. It
//! cannot reach a store, a claim, or an effect, and nothing here is reachable
//! from the product's dependency graph.

#[cfg(feature = "owner-effect-fixture")]
pub mod config;

#[cfg(feature = "owner-effect-fixture")]
mod bootstrap;

#[cfg(feature = "owner-effect-fixture")]
pub use bootstrap::{FixtureBootstrap, Qualification};

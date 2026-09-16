//! Verified capability proofs are not Clone, Debug, or Serialize.

use sovereign_capability::v2::VerifiedCapabilityV2;

fn require_clone<T: Clone>() {}
fn require_debug<T: std::fmt::Debug>() {}
fn require_serialize<T: serde::Serialize>() {}

fn main() {
    require_clone::<VerifiedCapabilityV2>();
    require_debug::<VerifiedCapabilityV2>();
    require_serialize::<VerifiedCapabilityV2>();
}

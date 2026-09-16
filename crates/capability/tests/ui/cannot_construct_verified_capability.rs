//! Downstream code must not forge a verified capability proof.

use sovereign_capability::v2::VerifiedCapabilityV2;

fn main() {
    let claims: sovereign_capability::v2::CapabilityClaimsV2 = unreachable!();
    let _proof = VerifiedCapabilityV2 { claims };
}

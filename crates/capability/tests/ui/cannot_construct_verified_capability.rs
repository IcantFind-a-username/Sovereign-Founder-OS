//! Downstream code must not forge a verified capability proof.

use sovereign_capability::v2::VerifiedCapabilityV2;

fn main() {
    let _proof = VerifiedCapabilityV2 { claims: loop {} };
}

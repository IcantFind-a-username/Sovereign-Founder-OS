//! Downstream code must not reconstruct a reserved-effect handle.

use sovereign_synthetic_owner_effect::AuthorityReservedEffect;

fn main() {
    let _proof = AuthorityReservedEffect::from_intent_id(unreachable!());
}

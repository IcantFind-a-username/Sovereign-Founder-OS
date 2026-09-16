//! Downstream code must not forge a reserved-effect handle.

use sovereign_synthetic_owner_effect::AuthorityReservedEffect;

fn main() {
    let intent_id = unreachable!();
    let _proof = AuthorityReservedEffect { intent_id };
}

//! Downstream code must not destructure a reserved-effect handle.

use sovereign_synthetic_owner_effect::AuthorityReservedEffect;

fn main() {
    let reserved: AuthorityReservedEffect = unreachable!();
    let AuthorityReservedEffect { intent_id } = reserved;
    let _ = intent_id;
}

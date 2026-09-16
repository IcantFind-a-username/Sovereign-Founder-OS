//! Reserved-effect handles are not Clone, Debug, or Serialize.

use sovereign_synthetic_owner_effect::AuthorityReservedEffect;

fn require_clone<T: Clone>() {}
fn require_debug<T: std::fmt::Debug>() {}
fn require_serialize<T: serde::Serialize>() {}

fn main() {
    require_clone::<AuthorityReservedEffect>();
    require_debug::<AuthorityReservedEffect>();
    require_serialize::<AuthorityReservedEffect>();
}

//! Reserved handles cannot be used after move.

use sovereign_synthetic_owner_effect::AuthorityReservedEffect;

fn take() -> AuthorityReservedEffect {
    loop {}
}

fn consume(_handle: AuthorityReservedEffect) {}

fn main() {
    let handle = take();
    consume(handle);
    consume(handle);
}

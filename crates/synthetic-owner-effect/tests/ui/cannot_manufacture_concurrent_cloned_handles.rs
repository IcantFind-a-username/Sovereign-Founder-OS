//! Concurrent cloned writer handles cannot be manufactured.

use sovereign_synthetic_owner_effect::AuthorityReservedEffect;

fn require_clone<T: Clone>() {}

fn take() -> AuthorityReservedEffect {
    loop {}
}

fn consume(_handle: AuthorityReservedEffect) {}

fn main() {
    require_clone::<AuthorityReservedEffect>();
    let handle = take();
    let cloned = handle.clone();
    std::thread::spawn(move || consume(handle));
    std::thread::spawn(move || consume(cloned));
}

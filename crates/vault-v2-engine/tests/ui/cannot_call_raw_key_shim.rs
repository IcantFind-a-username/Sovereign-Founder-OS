//! The raw-key FFI shim is not a public entry point.

use sovereign_vault_v2_engine::VAULT_V2_FORMAT_VERSION;

fn main() {
    let _ = VAULT_V2_FORMAT_VERSION;
    let _ = sovereign_vault_v2_engine::open_keyed_hardened;
}

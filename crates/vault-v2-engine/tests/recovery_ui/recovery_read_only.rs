//! The protocol library must not link the private process engine module.

use sovereign_vault_v2_engine::ENGINE_PROTOCOL_VERSION;

fn main() {
    let _ = ENGINE_PROTOCOL_VERSION;
    let _engine = sovereign_vault_v2_engine::engine;
}

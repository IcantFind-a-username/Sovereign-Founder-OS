//! A hardened connection and its raw SQLite handle stay private to the process.

use sovereign_vault_v2_engine::VAULT_V2_FORMAT_VERSION;

fn main() {
    let _ = VAULT_V2_FORMAT_VERSION;
    let _handle: sovereign_vault_v2_engine::HardenedConnection;
}

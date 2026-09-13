//! Downstream code must not name the database key holder.

use sovereign_vault_v2_engine::VAULT_V2_FORMAT_VERSION;
use sovereign_vault_v2_engine::DbKey;

fn main() {
    let _ = VAULT_V2_FORMAT_VERSION;
    let _key: DbKey;
}

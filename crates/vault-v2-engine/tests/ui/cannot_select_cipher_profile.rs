//! The pinned cipher profile is not selectable from downstream code.

use sovereign_vault_v2_engine::VAULT_V2_FORMAT_VERSION;
use sovereign_vault_v2_engine::SqlcipherProfile;

fn main() {
    let _ = VAULT_V2_FORMAT_VERSION;
    let _profile = SqlcipherProfile::Pinned;
}

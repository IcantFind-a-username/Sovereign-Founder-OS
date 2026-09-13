//! Create mode is engine-internal; callers may not name it.

use sovereign_vault_v2_engine::VAULT_V2_FORMAT_VERSION;
use sovereign_vault_v2_engine::ConnectionMode;

fn main() {
    let _ = VAULT_V2_FORMAT_VERSION;
    let _ = ConnectionMode::ReadWriteCreateInternal;
}

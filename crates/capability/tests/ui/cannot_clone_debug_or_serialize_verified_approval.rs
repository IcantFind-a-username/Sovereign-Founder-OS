//! Verified approval proofs are not Clone, Debug, or Serialize.

use sovereign_capability::v2::VerifiedApprovalV1;

fn require_clone<T: Clone>() {}
fn require_debug<T: std::fmt::Debug>() {}
fn require_serialize<T: serde::Serialize>() {}

fn main() {
    require_clone::<VerifiedApprovalV1>();
    require_debug::<VerifiedApprovalV1>();
    require_serialize::<VerifiedApprovalV1>();
}

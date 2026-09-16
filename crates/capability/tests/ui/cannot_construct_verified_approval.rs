//! Downstream code must not forge a verified approval proof.

use sovereign_capability::v2::VerifiedApprovalV1;

fn main() {
    let claims: sovereign_capability::approval::ApprovalClaimsV1 = unreachable!();
    let _proof = VerifiedApprovalV1 { claims };
}

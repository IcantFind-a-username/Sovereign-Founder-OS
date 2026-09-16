//! Route evidence is broker-derived. Callers cannot mint it from a string
//! or name a public-cloud provider identity.

use sovereign_privacy::{AttemptOutcome, ClosedProviderId, Placement, Purpose, RouteEvidence};

fn require_from_str<T: From<&'static str>>() {}

fn main() {
    require_from_str::<RouteEvidence>();
    let _forged = RouteEvidence {
        placement: Placement::PublicProjection,
        provider: ClosedProviderId::PublicCloud,
        outcome: AttemptOutcome::Succeeded,
        policy_digest: "i-used-the-cloud".into(),
        purpose: Purpose::DraftDiscoverySummary,
        stand_in_kind: "cloud-assisted",
    };
}

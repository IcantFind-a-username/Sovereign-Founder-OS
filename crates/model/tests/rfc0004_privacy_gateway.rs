//! RFC 0004 v0.2 privacy slice 2: the closed privacy gateway is reachable
//! only as a high-level re-export from this crate. Slice 1's raw-request
//! tests stay in `rfc0004_legacy_egress.rs` and are not reopened here.

use sovereign_model::{LocalCapability, PrivacyGateway};
use sovereign_privacy::{
    PolicySnapshot, Preset, Provenance, Purpose, SourceRecord, TrustedValue, WorkflowOutcome,
};

const NOW: i64 = 1_800_000_000;

fn discovery_record() -> SourceRecord {
    SourceRecord::new()
        .with(
            "customer.discovery_notes",
            TrustedValue::protected(
                "Acme Ltd spends six hours a week on reporting.",
                Provenance::OwnerEntered,
            ),
        )
        .with(
            "venture.service",
            TrustedValue::public_content("Reporting clarity sprints"),
        )
}

#[test]
fn privacy_gateway_local_only_workflow_produces_zero_public_adapter_observations() {
    let gateway = PrivacyGateway::new();
    let outcome = gateway
        .run(
            &discovery_record(),
            Purpose::DraftDiscoverySummary,
            PolicySnapshot::new(Preset::LocalOnly, NOW),
            NOW,
            LocalCapability::stand_in_available(),
        )
        .unwrap();
    assert!(matches!(outcome, WorkflowOutcome::Local { .. }));
    assert_eq!(gateway.public_adapter_observations(), 0);
    assert_eq!(gateway.owned_node_observations(), 0);
    assert_eq!(gateway.public_jobs_created(), 0);
}

#[test]
fn owned_mesh_activation_reexport_is_rejected() {
    assert!(sovereign_model::activate_owned_mesh().is_err());
}

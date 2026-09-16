//! RFC 0004 v0.2 privacy slice 2: `LocalOnly` zero-public-observation path,
//! OwnedMesh rejection, honest on-device stand-in labels, and value-free
//! evidence canaries.
//!
//! This is **not** full RFC 0004, **not** a real local-model sandbox, **not**
//! ActiveV2, **not** product Exact Effect / 1C0, and **not** Secure Mesh.

use sovereign_privacy::{
    activate_owned_mesh, ActivationError, AttemptOutcome, ClosedProviderId, ComputeUnavailable,
    GatewayError, LocalCapability, Placement, PolicySnapshot, Preset, PrivacyGateway, Provenance,
    Purpose, SourceRecord, TrustedValue, WorkflowOutcome, STAND_IN_EXPLANATION, STAND_IN_KIND,
    STAND_IN_LABEL,
};

const NOW: i64 = 1_800_000_000;
const CANARY: &str = "CANARY-8fbd41c0-do-not-leak";

fn owner(value: &str) -> TrustedValue {
    TrustedValue::protected(value, Provenance::OwnerEntered)
}

fn discovery_record() -> SourceRecord {
    SourceRecord::new()
        .with("customer.name", owner("Acme Ltd"))
        .with("customer.contact_name", owner("Alex Chen"))
        .with("customer.email", owner("alex.chen@acme.test"))
        .with(
            "customer.discovery_notes",
            owner(
                "Acme Ltd spends six hours a week on reporting. Alex Chen says finance must approve. Budget SGD 3,000-5,000.",
            ),
        )
        .with(
            "venture.service",
            TrustedValue::public_content("Reporting clarity sprints"),
        )
        .with("venture.currency", TrustedValue::public_content("SGD"))
}

fn canary_record() -> SourceRecord {
    SourceRecord::new().with(
        "customer.discovery_notes",
        owner(&format!("{CANARY} discovery notes")),
    )
}

#[test]
fn local_only_workflow_produces_zero_public_adapter_observations() {
    let gateway = PrivacyGateway::new();
    let policy = PolicySnapshot::new(Preset::LocalOnly, NOW);
    let outcome = gateway
        .run(
            &discovery_record(),
            Purpose::DraftDiscoverySummary,
            policy,
            NOW,
            LocalCapability::stand_in_available(),
        )
        .unwrap();

    let WorkflowOutcome::Local { result, evidence } = outcome else {
        panic!("expected an on-device local outcome, got {outcome:?}");
    };
    assert_eq!(evidence.placement(), Placement::Local);
    assert_eq!(evidence.provider(), ClosedProviderId::OnDeviceDeterministic);
    assert_eq!(evidence.outcome(), AttemptOutcome::Succeeded);
    assert_eq!(evidence.stand_in_kind(), STAND_IN_KIND);
    assert_eq!(result.kind(), STAND_IN_KIND);
    assert!(result
        .owner_view()
        .contains("six hours a week on reporting"));
    assert_eq!(gateway.public_adapter_observations(), 0);
    assert_eq!(gateway.owned_node_observations(), 0);
    assert_eq!(gateway.public_jobs_created(), 0);
}

#[test]
fn local_compute_failure_cannot_create_or_reach_a_public_request_under_local_only() {
    let gateway = PrivacyGateway::new();
    let policy = PolicySnapshot::new(Preset::LocalOnly, NOW);

    // Missing local compute: queue, never compile a public job.
    let queued = gateway
        .run(
            &discovery_record(),
            Purpose::DraftDiscoverySummary,
            policy,
            NOW,
            LocalCapability::none(),
        )
        .unwrap();
    let WorkflowOutcome::Queued {
        reason,
        alternatives,
        evidence,
    } = queued
    else {
        panic!("expected a queued outcome, got {queued:?}");
    };
    assert_eq!(reason, ComputeUnavailable::LocalOnlyNoLocalCompute);
    assert!(!alternatives
        .iter()
        .any(|option| option.as_str().contains("owned")));
    assert_eq!(evidence.outcome(), AttemptOutcome::Queued);
    assert_eq!(gateway.public_adapter_observations(), 0);
    assert_eq!(gateway.owned_node_observations(), 0);
    assert_eq!(gateway.public_jobs_created(), 0);

    // Stand-in present but the record cannot be processed: fail closed,
    // still no public request.
    let failed = gateway.run(
        &SourceRecord::new(),
        Purpose::DraftDiscoverySummary,
        policy,
        NOW,
        LocalCapability::stand_in_available(),
    );
    assert!(matches!(failed, Err(GatewayError::LocalComputeFailed(_))));
    assert_eq!(gateway.public_adapter_observations(), 0);
    assert_eq!(gateway.owned_node_observations(), 0);
    assert_eq!(gateway.public_jobs_created(), 0);

    // Contrast: AutoProtect without local compute does compile a projection
    // for the in-process stand-in — LocalOnly must not take that path.
    let auto = PrivacyGateway::new();
    let auto_policy = PolicySnapshot::new(Preset::AutoProtect, NOW);
    let projected = auto
        .run(
            &discovery_record(),
            Purpose::DraftDiscoverySummary,
            auto_policy,
            NOW,
            LocalCapability::none(),
        )
        .unwrap();
    assert!(matches!(projected, WorkflowOutcome::Projection { .. }));
    assert_eq!(auto.public_jobs_created(), 1);
    assert_eq!(auto.public_adapter_observations(), 1);
    assert_eq!(auto.owned_node_observations(), 0);
}

#[test]
fn owned_mesh_public_activation_rejected_in_this_version() {
    assert_eq!(
        activate_owned_mesh(),
        Err(ActivationError::OwnedMeshNotAvailableInThisVersion)
    );
    for label in [
        "owned_mesh",
        "OwnedMesh",
        "my_devices",
        "company_nodes",
        "My Devices & Company Nodes",
        CANARY,
    ] {
        let error = Preset::parse_selectable(label).unwrap_err();
        assert!(
            !error.to_string().contains(CANARY),
            "activation errors must be value-free: {error}"
        );
        if label == CANARY {
            assert_eq!(error, ActivationError::UnknownPreset);
        } else {
            assert_eq!(error, ActivationError::OwnedMeshNotAvailableInThisVersion);
        }
    }
    assert_eq!(
        Preset::parse_selectable("local_only").unwrap(),
        Preset::LocalOnly
    );
    assert_eq!(
        Preset::parse_selectable("auto_protect").unwrap(),
        Preset::AutoProtect
    );
    assert_eq!(Preset::default(), Preset::AutoProtect);

    let gateway = PrivacyGateway::new();
    assert_eq!(gateway.owned_node_observations(), 0);
}

#[test]
fn deterministic_stand_in_labelled_as_on_device_demonstration() {
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
    let WorkflowOutcome::Local { result, evidence } = outcome else {
        panic!("expected local outcome, got {outcome:?}");
    };

    assert_eq!(result.kind(), STAND_IN_KIND);
    assert_eq!(result.label(), STAND_IN_LABEL);
    assert_eq!(result.explanation(), STAND_IN_EXPLANATION);
    assert_eq!(evidence.stand_in_kind(), STAND_IN_KIND);

    let kind = result.kind().to_lowercase();
    assert!(kind.contains("on_device") || kind.contains("on-device"));
    assert!(kind.contains("demonstration"));
    let label = result.label().to_lowercase();
    assert!(label.contains("on-device"));
    assert!(!label.contains("cloud-assisted"));
    let explanation = result.explanation().to_lowercase();
    assert!(explanation.contains("on-device demonstration"));
    assert!(explanation.contains("not cloud-assisted"));
    assert!(explanation.contains("not real ai"));
    assert!(
        !explanation.contains("cloud-assisted inference")
            || explanation.contains("not cloud-assisted")
    );
}

#[test]
fn debug_display_errors_and_serialized_evidence_stay_value_free() {
    let gateway = PrivacyGateway::new();
    let outcome = gateway
        .run(
            &canary_record(),
            Purpose::DraftDiscoverySummary,
            PolicySnapshot::new(Preset::LocalOnly, NOW),
            NOW,
            LocalCapability::stand_in_available(),
        )
        .unwrap();
    let WorkflowOutcome::Local { result, evidence } = outcome else {
        panic!("expected local outcome, got {outcome:?}");
    };

    assert!(
        result.owner_view().contains(CANARY),
        "the owner-facing view may show the processed text"
    );

    let printed_result = format!("{result:?}");
    assert!(!printed_result.contains(CANARY), "{printed_result}");
    let printed_evidence = format!("{evidence:?}");
    assert!(!printed_evidence.contains(CANARY), "{printed_evidence}");
    let serialized = serde_json::to_string(&evidence).unwrap();
    assert!(!serialized.contains(CANARY), "{serialized}");
    let parsed: serde_json::Value = serde_json::from_str(&serialized).unwrap();
    let object = parsed.as_object().expect("evidence is an object");
    for key in [
        "placement",
        "provider",
        "outcome",
        "policy_digest",
        "purpose",
        "stand_in_kind",
    ] {
        assert!(object.contains_key(key), "missing {key} in {serialized}");
    }

    let error = GatewayError::LocalComputeFailed(sovereign_privacy::LocalError::MissingField(
        "customer.discovery_notes".into(),
    ));
    assert!(!error.to_string().contains(CANARY));
    assert!(!format!("{error:?}").contains(CANARY));
    assert!(!activate_owned_mesh()
        .unwrap_err()
        .to_string()
        .contains(CANARY));
}

#[test]
fn privacy_crate_manifest_does_not_depend_on_forbidden_crates() {
    let manifest = include_str!("../Cargo.toml");
    for forbidden in [
        "sovereign-model",
        "sovereign-policy",
        "sovereign-workflow",
        "sovereign-cli",
    ] {
        assert!(
            !manifest.contains(forbidden),
            "sovereign-privacy must not depend on {forbidden}"
        );
    }
    assert!(
        !manifest.contains("NonDisclosableSecret"),
        "this crate must not own NonDisclosableSecret"
    );
}

#[test]
fn auto_protect_prefers_local_and_creates_no_public_job() {
    let gateway = PrivacyGateway::new();
    let outcome = gateway
        .run(
            &discovery_record(),
            Purpose::DraftDiscoverySummary,
            PolicySnapshot::new(Preset::AutoProtect, NOW),
            NOW,
            LocalCapability::stand_in_available(),
        )
        .unwrap();
    assert!(matches!(outcome, WorkflowOutcome::Local { .. }));
    assert_eq!(gateway.public_adapter_observations(), 0);
    assert_eq!(gateway.public_jobs_created(), 0);
}

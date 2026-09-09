//! The boundary's adversarial tests (RFC 0004, "Required security tests"),
//! at the fidelity this slice ships: the compiler is the only door, an
//! unnamed field cannot leave, payloads do not vary with protected values,
//! a canary planted in every field never surfaces in evidence or in a
//! printed error, and a public route cannot be reached under `LocalOnly`.

use sovereign_privacy::{
    accept, compile, CompileError, PlacementDecision, PolicySnapshot, Preset, Provenance, Purpose,
    Recipient, ResponseError, SourceRecord, TrustedValue, MAX_RESPONSE_CHARS,
};

const NOW: i64 = 1_800_000_000;
/// A string that must never appear anywhere but the outbound preview.
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
            owner("Acme Ltd spends six hours a week on reporting. Alex Chen says finance must approve. Budget SGD 3,000-5,000."),
        )
        .with("venture.service", TrustedValue::public_content("Reporting clarity sprints"))
        .with("venture.currency", TrustedValue::public_content("SGD"))
}

fn auto_protect() -> PolicySnapshot {
    PolicySnapshot::new(Preset::AutoProtect, NOW)
}

#[test]
fn the_compiler_replaces_names_everywhere_and_never_sends_the_address() {
    let (job, preview) = compile(
        &discovery_record(),
        Purpose::DraftDiscoverySummary,
        auto_protect(),
        NOW,
    )
    .unwrap();
    let sent = preview.outbound_text.clone();

    // Identifying values are gone, including from inside the free text.
    for secret in [
        "Acme Ltd",
        "Acme",
        "Alex Chen",
        "alex.chen@acme.test",
        "acme",
    ] {
        assert!(
            !sent.to_lowercase().contains(&secret.to_lowercase()),
            "the projection leaked {secret}:\n{sent}"
        );
    }
    // The substance survives.
    assert!(sent.contains("six hours a week on reporting"));
    assert!(sent.contains("SGD 3,000-5,000"));
    assert!(sent.contains("[ORG_1]") && sent.contains("[PERSON_1]"));
    assert!(
        sent.contains("Reporting clarity sprints"),
        "own service is ours to send"
    );

    // The preview tells the founder what happened to each field.
    let outcome = |field: &str| {
        preview
            .rows
            .iter()
            .find(|row| row.field == field)
            .unwrap_or_else(|| panic!("no preview row for {field}"))
            .outcome
    };
    assert_eq!(outcome("customer.name"), "replaced");
    assert_eq!(outcome("customer.email"), "omitted");
    assert_eq!(outcome("customer.discovery_notes"), "sent");
    assert_eq!(outcome("venture.currency"), "sent");
    assert_eq!(job.grant().recipient, Recipient::PublicComputeProjection);
    assert_eq!(job.purpose(), Purpose::DraftDiscoverySummary);
}

#[test]
fn a_field_the_transform_does_not_name_can_never_leave() {
    // The caller offers a field that no transform names — the shape of a
    // future workspace field added without reviewing the transforms.
    let record = discovery_record()
        .with("customer.bank_account", owner(CANARY))
        .with("customer.private_note", owner("we can push them to 8000"));
    let (job, preview) =
        compile(&record, Purpose::DraftDiscoverySummary, auto_protect(), NOW).unwrap();

    assert!(!preview.outbound_text.contains(CANARY));
    assert!(!preview.outbound_text.contains("push them to 8000"));
    // It is not even mentioned as absent: an unnamed field is invisible.
    assert!(preview
        .rows
        .iter()
        .all(|row| row.field != "customer.bank_account"));
    let manifest = serde_json::to_string(&job.manifest()).unwrap();
    assert!(!manifest.contains(CANARY));
}

#[test]
fn payloads_do_not_vary_with_the_protected_values_they_stand_for() {
    let notes = "The team spends six hours a week on reporting. Finance must approve.";
    let build = |org: &str, person: &str| {
        SourceRecord::new()
            .with("customer.name", owner(org))
            .with("customer.contact_name", owner(person))
            .with("customer.email", owner("someone@example.test"))
            .with("customer.discovery_notes", owner(notes))
            .with(
                "venture.service",
                TrustedValue::public_content("Reporting clarity sprints"),
            )
            .with("venture.currency", TrustedValue::public_content("SGD"))
    };
    let short = compile(
        &build("Bo", "Li"),
        Purpose::DraftDiscoverySummary,
        auto_protect(),
        NOW,
    )
    .unwrap()
    .1;
    let long = compile(
        &build(
            "A Very Long Organisation Name Limited Partnership",
            "Bartholomew Featherstonehaugh",
        ),
        Purpose::DraftDiscoverySummary,
        auto_protect(),
        NOW,
    )
    .unwrap()
    .1;
    assert_eq!(
        short.outbound_text, long.outbound_text,
        "neither the value nor its length may be inferable from the payload"
    );
    assert_eq!(short.outbound_bytes, long.outbound_bytes);
}

#[test]
fn a_name_too_short_to_scrub_is_flagged_rather_than_silently_missed() {
    let record = SourceRecord::new()
        .with("customer.name", owner("Bo"))
        .with(
            "customer.discovery_notes",
            owner("Bo needs weekly reporting to take less time."),
        )
        .with("venture.service", TrustedValue::public_content("Sprints"))
        .with("venture.currency", TrustedValue::public_content("SGD"));
    let (_, preview) =
        compile(&record, Purpose::DraftDiscoverySummary, auto_protect(), NOW).unwrap();
    let notes = preview
        .rows
        .iter()
        .find(|row| row.field == "customer.discovery_notes")
        .unwrap();
    assert!(
        notes.warning.is_some(),
        "a two-letter name cannot be removed from prose reliably, and the founder must be told"
    );
    // The field itself is still replaced, and the text is still shown in full.
    assert!(preview.outbound_text.contains("[ORG_1]"));
}

#[test]
fn local_only_can_never_reach_a_public_projection() {
    let local_only = PolicySnapshot::new(Preset::LocalOnly, NOW);
    assert_eq!(
        compile(
            &discovery_record(),
            Purpose::DraftDiscoverySummary,
            local_only,
            NOW
        )
        .unwrap_err(),
        CompileError::PresetForbidsPublicCompute
    );
    // Losing local compute queues the task; it never widens the route.
    assert!(matches!(
        local_only.placement_for(false),
        PlacementDecision::Queued { .. }
    ));
    assert!(matches!(
        local_only.placement_for(true),
        PlacementDecision::Run(sovereign_privacy::Placement::Local)
    ));
    // Auto Protect prefers local when local is available.
    assert!(matches!(
        auto_protect().placement_for(true),
        PlacementDecision::Run(sovereign_privacy::Placement::Local)
    ));
}

#[test]
fn evidence_and_errors_are_value_free_under_a_canary_in_every_field() {
    let mut record = SourceRecord::new();
    for field in [
        "customer.name",
        "customer.contact_name",
        "customer.email",
        "customer.discovery_notes",
    ] {
        record = record.with(field, owner(&format!("{CANARY} {field}")));
    }
    record = record
        .with("venture.service", TrustedValue::public_content("Sprints"))
        .with("venture.currency", TrustedValue::public_content("SGD"));

    let (job, _) = compile(&record, Purpose::DraftDiscoverySummary, auto_protect(), NOW).unwrap();

    // The manifest is what gets persisted and logged.
    let manifest = job.manifest();
    let serialized = serde_json::to_string(&manifest).unwrap();
    assert!(!serialized.contains(CANARY), "{serialized}");
    assert!(!format!("{manifest:?}").contains(CANARY));
    // The job itself must not print protected values either: its Debug is
    // hand-written for exactly this reason, and a derived one leaks the
    // payload and the placeholder map.
    let printed = format!("{job:?}");
    assert!(!printed.contains(CANARY), "{printed}");
    assert!(
        printed.contains("outbound_digest"),
        "shape is still useful: {printed}"
    );
    // Nor may the preview, which is allowed to show values on screen but
    // never to be dumped into a log.
    let (_, preview) =
        compile(&record, Purpose::DraftDiscoverySummary, auto_protect(), NOW).unwrap();
    assert!(!format!("{preview:?}").contains(CANARY));
    assert!(
        preview.outbound_text.contains(CANARY),
        "the screen view still shows it"
    );
    // Nor may a value printed on its own.
    let value = owner(CANARY);
    assert!(!format!("{value:?}").contains(CANARY));
    // Errors name fields, never contents.
    let too_long = CompileError::FieldTooLong("customer.discovery_notes".into());
    assert!(!too_long.to_string().contains(CANARY));
}

#[test]
fn a_response_is_untrusted_bounded_and_cannot_invent_a_placeholder() {
    let (job, _) = compile(
        &discovery_record(),
        Purpose::DraftDiscoverySummary,
        auto_protect(),
        NOW,
    )
    .unwrap();

    // A well-behaved answer is rehydrated locally.
    let good = accept(
        &job,
        "[ORG_1] spends six hours weekly. Ask [PERSON_1] who approves budget.",
        NOW + 5,
    )
    .unwrap();
    let text = format!("{:?}", good.text());
    assert!(!text.contains("Acme"), "the value must not print: {text}");
    assert_eq!(good.substituted.len(), 2);

    // A label this job never issued is a refusal, not a substitution.
    assert_eq!(
        accept(&job, "Contact [ORG_7] instead.", NOW + 5).unwrap_err(),
        ResponseError::UnknownPlaceholder("[ORG_7]".into())
    );
    // Bounds and shape.
    assert_eq!(
        accept(&job, "   ", NOW + 5).unwrap_err(),
        ResponseError::Empty
    );
    let huge = "x".repeat(MAX_RESPONSE_CHARS + 1);
    assert!(matches!(
        accept(&job, &huge, NOW + 5).unwrap_err(),
        ResponseError::TooLarge { .. }
    ));
    assert_eq!(
        accept(&job, "text with a \u{0} byte", NOW + 5).unwrap_err(),
        ResponseError::ControlCharacters
    );
}

#[test]
fn a_job_expires_and_cannot_be_replayed_later() {
    let (job, _) = compile(
        &discovery_record(),
        Purpose::DraftDiscoverySummary,
        auto_protect(),
        NOW,
    )
    .unwrap();
    assert!(job.is_live_at(NOW));
    assert!(job.is_live_at(job.expires_at_unix() - 1));
    assert!(!job.is_live_at(job.expires_at_unix()));
    assert!(
        !job.is_live_at(NOW - 1),
        "a job is not live before it existed"
    );
    assert_eq!(
        accept(&job, "[ORG_1] is fine.", job.expires_at_unix()).unwrap_err(),
        ResponseError::JobExpired
    );
}

#[test]
fn model_output_cannot_be_fed_back_in_as_a_public_field() {
    // Taint propagation: a value the model produced is RestrictedDerived and
    // must not be re-sent as though the transform's author had vetted it.
    let record = discovery_record().with(
        "venture.service",
        TrustedValue::restricted_derived("service description the model wrote"),
    );
    assert_eq!(
        compile(&record, Purpose::DraftDiscoverySummary, auto_protect(), NOW).unwrap_err(),
        CompileError::UntrustedInput("venture.service".into())
    );
}

#[test]
fn every_purpose_compiles_and_two_jobs_are_never_the_same_job() {
    let record = discovery_record()
        .with("proposal.amount", TrustedValue::public_content("4000.00"))
        .with(
            "document.body",
            owner("Offer for Acme Ltd. Scope to be confirmed."),
        )
        .with("document.kind", TrustedValue::public_content("offer"));
    let mut digests = Vec::new();
    for purpose in [
        Purpose::DraftDiscoverySummary,
        Purpose::DraftProposal,
        Purpose::ReviewDraft,
    ] {
        let (job, preview) = compile(&record, purpose, auto_protect(), NOW).unwrap();
        assert_eq!(job.purpose(), purpose);
        assert!(!preview.outbound_text.to_lowercase().contains("acme"));
        digests.push(job.outbound_digest());
        // Distinct identity per compilation, even for the same inputs.
        let (again, _) = compile(&record, purpose, auto_protect(), NOW).unwrap();
        assert_ne!(job.id(), again.id());
        assert_eq!(
            job.outbound_digest(),
            again.outbound_digest(),
            "bytes are deterministic"
        );
    }
    digests.dedup();
    assert_eq!(digests.len(), 3, "each purpose sends different bytes");
}

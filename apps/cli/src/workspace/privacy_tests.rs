//! The boundary as the product uses it: the founder's route selection is
//! durable and audited, and a preview shows exactly what a public model
//! would receive — with the identifying values gone, including from the
//! prose that mentions them.

use super::*;
use sovereign_audit_ledger::AuditLedger;
use sovereign_identity::DeviceIdentity;
use tempfile::tempdir;

fn seeded() -> (tempfile::TempDir, Store, uuid::Uuid) {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    store
        .update_venture_profile(VentureProfileInput {
            name: "Acme Consulting".into(),
            service: "Reporting clarity sprints".into(),
            jurisdiction: "SG".into(),
            currency: "SGD".into(),
            ..VentureProfileInput::default()
        })
        .unwrap();
    let workspace = store
        .add_customer(
            "Northwind Traders",
            "ada.lovelace@northwind.test",
            "met at expo",
        )
        .unwrap();
    let customer_id = workspace.customers[0].id;
    store
        .update_customer(
            customer_id,
            CustomerInput {
                name: "Northwind Traders".into(),
                email: "ada.lovelace@northwind.test".into(),
                notes: "met at expo".into(),
                discovery_notes:
                    "Northwind Traders spends six hours a week on reporting. Budget SGD 3,000-5,000."
                        .into(),
                jurisdiction: "SG".into(),
                ..CustomerInput::default()
            },
        )
        .unwrap();
    (dir, store, customer_id)
}

fn actions(dir: &tempfile::TempDir) -> Vec<String> {
    let device = DeviceIdentity::load(&dir.path().join("device.json")).unwrap();
    let ledger =
        AuditLedger::load(&dir.path().join("ledger.json"), device.public_key_b64()).unwrap();
    ledger.verify_chain().unwrap();
    ledger.events().iter().map(|e| e.action.clone()).collect()
}

#[test]
fn the_route_defaults_to_auto_protect_and_a_change_is_audited() {
    let (dir, store, _) = seeded();
    assert_eq!(store.privacy_preset(), StoredPreset::AutoProtect);

    store.set_privacy_preset(StoredPreset::LocalOnly).unwrap();
    assert_eq!(store.privacy_preset(), StoredPreset::LocalOnly);
    assert_eq!(actions(&dir).last().unwrap(), "privacy.preset");

    // Selecting the same route again is not an event.
    let before = actions(&dir).len();
    store.set_privacy_preset(StoredPreset::LocalOnly).unwrap();
    assert_eq!(actions(&dir).len(), before);

    // It survives a reopen: this is device policy, not session state.
    let reopened = Store::open(dir.path()).unwrap();
    assert_eq!(reopened.privacy_preset(), StoredPreset::LocalOnly);
}

#[test]
fn a_preview_shows_the_exact_bytes_with_the_identifying_values_gone() {
    let (_dir, store, customer_id) = seeded();
    let view = store
        .exposure_preview("draft_proposal", Some(customer_id), None)
        .unwrap();

    for secret in ["Northwind", "ada.lovelace", "northwind.test"] {
        assert!(
            !view
                .outbound_text
                .to_lowercase()
                .contains(&secret.to_lowercase()),
            "the projection leaked {secret}:\n{}",
            view.outbound_text
        );
    }
    // The substance the task needs is still there.
    assert!(view.outbound_text.contains("six hours a week on reporting"));
    assert!(view.outbound_text.contains("SGD 3,000-5,000"));
    assert!(view.outbound_text.contains("[ORG_1]"));
    assert_eq!(view.transform_id, "consulting.proposal-draft@1");
    assert!(view.outbound_bytes > 0 && view.outbound_digest.len() == 64);
    assert!(!view.dispatch_available, "no public adapter exists yet");

    let outcome = |field: &str| {
        view.fields
            .iter()
            .find(|row| row.field == field)
            .unwrap_or_else(|| panic!("no row for {field}"))
            .outcome
            .clone()
    };
    assert_eq!(outcome("customer.name"), "replaced");
    assert_eq!(outcome("customer.email"), "omitted");
    assert_eq!(outcome("customer.discovery_notes"), "sent");
    // A field the workspace holds but no transform names is not even listed.
    assert!(view.fields.iter().all(|row| row.field != "customer.notes"));
}

#[test]
fn local_only_still_answers_what_would_leave_but_routes_the_work_here() {
    let (_dir, store, customer_id) = seeded();
    let auto = store
        .exposure_preview("draft_discovery_summary", Some(customer_id), None)
        .unwrap();
    assert_eq!(
        auto.placement, "local",
        "a healthy local provider serves it"
    );

    store.set_privacy_preset(StoredPreset::LocalOnly).unwrap();
    let local_only = store
        .exposure_preview("draft_discovery_summary", Some(customer_id), None)
        .unwrap();
    // The founder is entitled to see what a public route would carry before
    // choosing one, so the preview still compiles under Local Only...
    assert_eq!(local_only.outbound_text, auto.outbound_text);
    // ...and the route it reports is still this device.
    assert_eq!(local_only.placement, "local");
}

#[test]
fn a_document_review_previews_the_draft_and_fails_closed_on_bad_input() {
    let (_dir, store, customer_id) = seeded();
    let workspace = store
        .create_document(DocumentKind::Offer, customer_id, None, "en")
        .unwrap();
    let document_id = workspace.documents[0].id;

    let view = store
        .exposure_preview("review_draft", Some(customer_id), Some(document_id))
        .unwrap();
    assert_eq!(view.transform_id, "consulting.draft-review@1");
    assert!(!view.outbound_text.to_lowercase().contains("northwind"));
    assert!(view.outbound_text.contains("[ORG_1]"));

    assert!(store
        .exposure_preview("draft_world_peace", None, None)
        .is_err());
    assert!(store
        .exposure_preview("draft_proposal", Some(uuid::Uuid::new_v4()), None)
        .is_err());
}

#[test]
fn a_preview_needs_a_company_profile_first() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let error = store
        .exposure_preview("draft_proposal", None, None)
        .unwrap_err();
    assert!(error.to_string().contains("company profile"));
}

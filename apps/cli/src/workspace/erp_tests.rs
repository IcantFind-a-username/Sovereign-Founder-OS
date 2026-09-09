use super::*;
use sovereign_audit_ledger::AuditLedger;
use sovereign_identity::DeviceIdentity;
use tempfile::tempdir;
use uuid::Uuid;

fn store() -> (tempfile::TempDir, Store) {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    (dir, store)
}

fn unix_now() -> i64 {
    chrono::Utc::now().timestamp()
}

fn actions(dir: &tempfile::TempDir) -> Vec<String> {
    let device = DeviceIdentity::load(&dir.path().join("device.json")).unwrap();
    let ledger =
        AuditLedger::load(&dir.path().join("ledger.json"), device.public_key_b64()).unwrap();
    ledger.verify_chain().unwrap();
    ledger
        .events()
        .iter()
        .map(|event| event.action.clone())
        .collect()
}

/// A customer with one sent (approved) offer and one issued invoice.
fn seeded() -> (tempfile::TempDir, Store, Uuid, Uuid, Uuid) {
    let (dir, store) = store();
    store
        .set_venture("Acme Consulting", "Reporting sprints")
        .unwrap();
    let workspace = store
        .add_customer("Acme Ltd", "alex@example.com", "met at expo")
        .unwrap();
    let customer_id = workspace.customers[0].id;
    let workspace = store
        .create_document(DocumentKind::Offer, customer_id, None, "en")
        .unwrap();
    let offer_id = workspace.documents[0].id;
    let workspace = store.request_send(offer_id).unwrap();
    store.decide(workspace.approvals[0].id, true).unwrap();
    let workspace = store
        .create_document(DocumentKind::Invoice, customer_id, Some(350_000), "en")
        .unwrap();
    let invoice_id = workspace.documents[1].id;
    let workspace = store.request_send(invoice_id).unwrap();
    let approval = workspace
        .approvals
        .iter()
        .find(|approval| approval.document_id == invoice_id)
        .unwrap()
        .id;
    store.decide(approval, true).unwrap();
    (dir, store, customer_id, offer_id, invoice_id)
}

#[test]
fn a_version_one_vault_loads_with_defaults_and_is_stamped_on_commit() {
    let (dir, store) = store();
    let customer_id = Uuid::new_v4();
    let legacy = serde_json::json!({
        "version": 1,
        "venture": { "name": "Acme", "service": "Landing pages", "updated_at": 1 },
        "customers": [{ "id": customer_id, "name": "Dr. Tan", "email": "dr.tan@example.com", "notes": "", "created_at": 1 }],
        "documents": [{ "id": Uuid::new_v4(), "kind": "offer", "customer_id": customer_id, "title": "Offer", "body": "x", "amount_cents": null, "status": "draft", "created_at": 1 }],
        "approvals": []
    });
    let mut vault = sovereign_vault::Vault::init(dir.path().join("vault")).unwrap();
    vault
        .put(WORKSPACE_VAULT_ENTRY, &serde_json::to_vec(&legacy).unwrap())
        .unwrap();

    let loaded = store.load().unwrap();
    assert_eq!(loaded.version, WORKSPACE_VERSION);
    let venture = loaded.venture.as_ref().unwrap();
    assert_eq!(venture.currency, "SGD");
    assert_eq!(venture.fiscal_year_end_month, 12);
    assert!(venture.jurisdiction.is_empty());
    // Pre-stage records were customers, not leads.
    assert_eq!(loaded.customers[0].stage, CustomerStage::Customer);
    assert_eq!(loaded.documents[0].revision, 1);
    assert!(loaded.projects.is_empty() && loaded.payments.is_empty());

    // Nothing was rewritten on read; the next commit persists version 2.
    let raw = sovereign_vault::Vault::init(dir.path().join("vault"))
        .unwrap()
        .get(WORKSPACE_VAULT_ENTRY)
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&raw).unwrap()["version"],
        1
    );
    store
        .add_follow_up(customer_id, unix_now() + 3600, "call back")
        .unwrap();
    let raw = sovereign_vault::Vault::init(dir.path().join("vault"))
        .unwrap()
        .get(WORKSPACE_VAULT_ENTRY)
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&raw).unwrap()["version"],
        2
    );
}

#[test]
fn profile_update_keeps_registration_facts_and_legacy_set_venture_keeps_them_too() {
    let (dir, store) = store();
    let workspace = store
        .update_venture_profile(VentureProfileInput {
            name: "Acme Consulting".into(),
            service: "Reporting sprints".into(),
            jurisdiction: "sg".into(),
            currency: "".into(),
            uen: "202412345A".into(),
            gst_registered: true,
            incorporated_at: Some(1_700_000_000),
            fiscal_year_end_month: Some(3),
            revenue_estimate_cents: Some(50_000_000),
        })
        .unwrap();
    let venture = workspace.venture.unwrap();
    assert_eq!(venture.jurisdiction, "SG");
    assert_eq!(venture.currency, "SGD");
    assert!(venture.gst_registered);
    assert_eq!(venture.fiscal_year_end_month, 3);

    // The old name/service path must not wipe the registration facts.
    let workspace = store.set_venture("Acme Consulting Pte", "Sprints").unwrap();
    let venture = workspace.venture.unwrap();
    assert_eq!(venture.name, "Acme Consulting Pte");
    assert_eq!(venture.uen, "202412345A");
    assert!(venture.gst_registered);

    assert!(store
        .update_venture_profile(VentureProfileInput {
            name: "X".into(),
            service: "Y".into(),
            jurisdiction: "singapore".into(),
            ..VentureProfileInput::default()
        })
        .is_err());
    assert_eq!(
        actions(&dir)
            .iter()
            .filter(|a| *a == "venture.update")
            .count(),
        2
    );
}

#[test]
fn customer_edits_are_audited_and_no_op_saves_commit_nothing() {
    let (dir, store) = store();
    store.set_venture("Acme", "Service").unwrap();
    let workspace = store.add_customer("Acme Ltd", "", "").unwrap();
    let customer_id = workspace.customers[0].id;
    assert_eq!(workspace.customers[0].stage, CustomerStage::Lead);

    let input = CustomerInput {
        name: "Acme Ltd".into(),
        email: "alex@example.com".into(),
        notes: "".into(),
        discovery_notes: "Weekly reporting takes six hours; budget SGD 3,000–5,000.".into(),
        stage: None,
        jurisdiction: "sg".into(),
        personal_data_consent: true,
    };
    let workspace = store.update_customer(customer_id, input.clone()).unwrap();
    let customer = &workspace.customers[0];
    assert_eq!(customer.email, "alex@example.com");
    assert_eq!(customer.jurisdiction, "SG");
    assert!(customer.personal_data_consent);
    assert!(customer.updated_at >= customer.created_at);

    let before = actions(&dir).len();
    store.update_customer(customer_id, input).unwrap();
    assert_eq!(
        actions(&dir).len(),
        before,
        "identical save must not append an event"
    );
    assert_eq!(actions(&dir).last().unwrap(), "customer.update");

    assert!(store
        .update_customer(Uuid::new_v4(), CustomerInput::default())
        .is_err());
    assert!(store
        .update_customer(
            customer_id,
            CustomerInput {
                name: "Acme".into(),
                email: "not an email".into(),
                ..CustomerInput::default()
            }
        )
        .is_err());
}

#[test]
fn draft_edits_bump_the_revision_and_sent_documents_are_immutable() {
    let (dir, store) = store();
    store.set_venture("Acme", "Service").unwrap();
    let workspace = store.add_customer("Acme Ltd", "", "").unwrap();
    let customer_id = workspace.customers[0].id;
    let workspace = store
        .create_document(DocumentKind::Offer, customer_id, None, "en")
        .unwrap();
    let document_id = workspace.documents[0].id;

    let workspace = store
        .update_document(
            document_id,
            "Reporting clarity sprint",
            "Scope: assess and improve.",
            None,
        )
        .unwrap();
    assert_eq!(workspace.documents[0].revision, 2);
    assert_eq!(workspace.documents[0].title, "Reporting clarity sprint");
    assert_eq!(actions(&dir).last().unwrap(), "document.update");

    let workspace = store.request_send(document_id).unwrap();
    assert_eq!(
        workspace.documents[0].status,
        DocumentStatus::PendingApproval
    );
    let error = store
        .update_document(document_id, "Changed after send", "x", None)
        .unwrap_err();
    assert!(error.to_string().contains("only drafts"));
    assert_eq!(
        store.load().unwrap().documents[0].title,
        "Reporting clarity sprint"
    );

    // An invoice edit must keep an amount.
    let workspace = store
        .create_document(DocumentKind::Invoice, customer_id, Some(1000), "en")
        .unwrap();
    let invoice_id = workspace.documents[1].id;
    assert!(store
        .update_document(invoice_id, "Invoice", "x", None)
        .is_err());
}

#[test]
fn accepting_a_sent_offer_promotes_the_lead_and_opens_the_project_flow() {
    let (dir, store, customer_id, offer_id, _invoice_id) = seeded();
    assert_eq!(
        store.load().unwrap().customers[0].stage,
        CustomerStage::Lead
    );

    let workspace = store.record_offer_accepted(offer_id).unwrap();
    assert!(workspace.documents[0].accepted_at.is_some());
    assert_eq!(workspace.customers[0].stage, CustomerStage::Customer);
    assert!(
        store.record_offer_accepted(offer_id).is_err(),
        "cannot accept twice"
    );
    let events = actions(&dir);
    assert!(events.contains(&"offer.accepted".to_owned()));

    // Guidance: an accepted offer with no project asks the founder to start one.
    let summary = store.command_center().unwrap();
    assert!(summary
        .guidance
        .iter()
        .any(|g| g.kind == "start_project_for" && g.subject == "Acme Ltd"));

    let workspace = store
        .add_project(
            customer_id,
            "Reporting clarity sprint",
            Some(offer_id),
            Some(350_000),
        )
        .unwrap();
    let project_id = workspace.projects[0].id;
    assert_eq!(workspace.projects[0].status, ProjectStatus::Proposed);
    let workspace = store
        .set_project_status(project_id, ProjectStatus::Active)
        .unwrap();
    assert_eq!(workspace.projects[0].status, ProjectStatus::Active);

    let workspace = store
        .add_task(
            project_id,
            "Interview the finance lead",
            Some(unix_now() + 86_400),
        )
        .unwrap();
    let task_id = workspace.tasks[0].id;
    assert_eq!(workspace.tasks[0].origin, "founder");
    let error = store
        .set_project_status(project_id, ProjectStatus::Done)
        .unwrap_err();
    assert!(error.to_string().contains("finish every task"));
    store.complete_task(task_id).unwrap();
    let workspace = store
        .set_project_status(project_id, ProjectStatus::Done)
        .unwrap();
    assert!(workspace.projects[0].done_at.is_some());
    assert!(store
        .set_project_status(project_id, ProjectStatus::Active)
        .is_err());
    assert!(store.add_task(project_id, "late", None).is_err());

    let summary = store.command_center().unwrap();
    assert!(summary
        .guidance
        .iter()
        .any(|g| g.kind == "invoice_done_project"));
    assert_eq!(summary.counts.projects_active, 0);
    assert_eq!(summary.counts.tasks_open, 0);
}

#[test]
fn payments_never_exceed_the_invoice_and_receivables_add_up() {
    let (dir, store, customer_id, _offer_id, invoice_id) = seeded();
    let owed = store.receivables().unwrap();
    assert_eq!(owed.len(), 1);
    assert_eq!(owed[0].outstanding_cents, 350_000);
    assert_eq!(owed[0].status, "open");

    let summary = store.command_center().unwrap();
    assert_eq!(summary.counts.receivable_cents, 350_000);
    let collect = summary
        .guidance
        .iter()
        .find(|g| g.kind == "collect_receivables")
        .unwrap();
    assert_eq!(collect.count, 1);
    assert_eq!(collect.subject, "3,500.00");

    store
        .record_payment(invoice_id, 100_000, unix_now(), "deposit")
        .unwrap();
    assert!(store
        .record_payment(invoice_id, 300_000, unix_now(), "too much")
        .is_err());
    assert!(store
        .record_payment(invoice_id, 0, unix_now(), "nothing")
        .is_err());
    let owed = store.receivables().unwrap();
    assert_eq!(owed[0].paid_cents, 100_000);
    assert_eq!(owed[0].status, "partial");
    store
        .record_payment(invoice_id, 250_000, unix_now(), "balance")
        .unwrap();
    let owed = store.receivables().unwrap();
    assert_eq!(owed[0].status, "paid");
    assert_eq!(store.command_center().unwrap().counts.receivable_cents, 0);

    // A draft invoice is not receivable and cannot take money.
    let workspace = store
        .create_document(DocumentKind::Invoice, customer_id, Some(1000), "en")
        .unwrap();
    let draft = workspace.documents.last().unwrap().id;
    assert!(store.record_payment(draft, 100, unix_now(), "").is_err());
    assert_eq!(store.receivables().unwrap().len(), 1);
    assert_eq!(
        actions(&dir)
            .iter()
            .filter(|a| *a == "payment.record")
            .count(),
        2
    );
}

#[test]
fn overdue_follow_ups_surface_and_the_timeline_lists_only_signed_events() {
    let (dir, store, customer_id, offer_id, _invoice_id) = seeded();
    let workspace = store
        .add_follow_up(customer_id, unix_now() - 3600, "chase the scoping call")
        .unwrap();
    let follow_up_id = workspace.follow_ups[0].id;
    let summary = store.command_center().unwrap();
    assert_eq!(summary.counts.follow_ups_overdue, 1);
    assert!(summary
        .guidance
        .iter()
        .any(|g| g.kind == "follow_up_overdue" && g.count == 1));

    store.complete_follow_up(follow_up_id).unwrap();
    assert_eq!(store.command_center().unwrap().counts.follow_ups_overdue, 0);
    assert!(store
        .add_follow_up(Uuid::new_v4(), unix_now(), "x")
        .is_err());
    assert!(store.add_follow_up(customer_id, 0, "x").is_err());

    let timeline = store.customer_timeline(customer_id).unwrap();
    let listed: Vec<&str> = timeline.iter().map(|entry| entry.action.as_str()).collect();
    assert!(listed.contains(&"customer.create"));
    assert!(listed.contains(&"approval.granted"));
    assert!(listed.contains(&"follow_up.create"));
    assert!(listed.contains(&"follow_up.done"));
    assert!(timeline
        .iter()
        .any(|entry| entry.resource == format!("document:{offer_id}")
            && entry.subject.starts_with("Offer")));
    // Events about another customer never leak in.
    let other = store.add_customer("Other Co", "", "").unwrap();
    let other_id = other.customers[1].id;
    let other_timeline = store.customer_timeline(other_id).unwrap();
    assert_eq!(other_timeline.len(), 1);
    assert_eq!(other_timeline[0].action, "customer.create");
    // The whole chain still verifies after every mutation above.
    actions(&dir);
}

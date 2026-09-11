use super::compliance::{ComplianceReport, FindingStatus};
use super::test_support::fake_ollama;
use super::*;
use sovereign_audit_ledger::AuditLedger;
use sovereign_identity::DeviceIdentity;
use tempfile::tempdir;

fn store() -> (tempfile::TempDir, Store) {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    (dir, store)
}

fn actions(dir: &tempfile::TempDir) -> Vec<String> {
    let device = DeviceIdentity::load(&dir.path().join("device.json")).unwrap();
    let ledger =
        AuditLedger::load(&dir.path().join("ledger.json"), device.public_key_b64()).unwrap();
    ledger.verify_chain().unwrap();
    ledger.events().iter().map(|e| e.action.clone()).collect()
}

/// The subject the chain names for the latest model call.
fn last_model_resource(dir: &tempfile::TempDir) -> String {
    let device = DeviceIdentity::load(&dir.path().join("device.json")).unwrap();
    let ledger =
        AuditLedger::load(&dir.path().join("ledger.json"), device.public_key_b64()).unwrap();
    ledger
        .events()
        .iter()
        .rev()
        .find(|e| e.action == "model.drafted")
        .map(|e| e.resource.clone())
        .unwrap()
}

fn profile(
    jurisdiction: &str,
    gst: bool,
    revenue: Option<u64>,
    incorporated_at: Option<i64>,
) -> VentureProfileInput {
    VentureProfileInput {
        name: "Acme Consulting".into(),
        service: "Reporting clarity sprints".into(),
        jurisdiction: jurisdiction.into(),
        currency: "SGD".into(),
        uen: String::new(),
        gst_registered: gst,
        incorporated_at,
        fiscal_year_end_month: Some(12),
        revenue_estimate_cents: revenue,
    }
}

fn status_of(report: &ComplianceReport, rule_id: &str) -> FindingStatus {
    report
        .findings
        .iter()
        .find(|finding| finding.rule_id == rule_id)
        .unwrap_or_else(|| panic!("no finding for {rule_id}"))
        .status
}

#[test]
fn an_unknown_jurisdiction_yields_an_unknown_report_and_never_a_pass() {
    let (_dir, store) = store();
    store
        .update_venture_profile(profile("", false, None, None))
        .unwrap();
    let report = store
        .run_compliance_check(ComplianceSubject::default(), "en")
        .unwrap();
    assert_eq!(report.coverage, "unknown_jurisdiction");
    assert_eq!(report.overall, "unknown");
    assert_eq!(report.findings.len(), 1);
    assert_eq!(report.findings[0].rule_id, "SG-COV-01");
    assert_eq!(report.findings[0].status, FindingStatus::Unknown);
    assert!(report.pack_id.is_empty());
    assert!(!report.model_backed);
}

#[test]
fn an_uncovered_jurisdiction_is_reported_not_guessed() {
    let (_dir, store) = store();
    store
        .update_venture_profile(profile("US", false, None, None))
        .unwrap();
    let report = store
        .run_compliance_check(ComplianceSubject::default(), "zh")
        .unwrap();
    assert_eq!(report.coverage, "not_covered");
    assert_eq!(report.overall, "not_covered");
    assert_eq!(report.findings[0].status, FindingStatus::Unknown);
    assert!(report.findings[0].detail.contains("US"));
}

#[test]
fn a_singapore_company_report_checks_gst_pdpa_contracts_and_deadlines() {
    let (dir, store) = store();
    // Incorporated 2024-01-15, financial year ends in December: at least one
    // year has ended, so the ECI and AGM windows are computable.
    store
        .update_venture_profile(profile("SG", false, Some(120_000_000), Some(1_705_276_800)))
        .unwrap();
    let workspace = store
        .add_customer("Acme Ltd", "alex@example.com", "")
        .unwrap();
    let acme = workspace.customers[0].id;
    let workspace = store.add_customer("Berlin GmbH", "", "").unwrap();
    let berlin = workspace.customers[1].id;
    store
        .update_customer(
            berlin,
            CustomerInput {
                name: "Berlin GmbH".into(),
                jurisdiction: "DE".into(),
                ..CustomerInput::default()
            },
        )
        .unwrap();
    // A sent template offer: it names scope but says nothing about
    // confidentiality, IP, or liability.
    let workspace = store
        .create_document(DocumentKind::Offer, acme, None, "en")
        .unwrap();
    let offer_id = workspace.documents[0].id;
    let workspace = store.request_send(offer_id).unwrap();
    store.decide(workspace.approvals[0].id, true).unwrap();

    let report = store
        .run_compliance_check(ComplianceSubject::default(), "en")
        .unwrap();
    assert_eq!(report.coverage, "covered");
    assert_eq!(report.pack_id, "sg-consulting-demo");
    assert!(report.review_status.contains("Unreviewed"));
    assert_eq!(report.overall, "attention");
    assert_eq!(status_of(&report, "SG-COV-01"), FindingStatus::Pass);
    // Revenue estimate above S$1M and not registered → action.
    assert_eq!(status_of(&report, "SG-GST-01"), FindingStatus::Attention);
    // One contact with details and no consent → action.
    let pdpa = report
        .findings
        .iter()
        .find(|f| f.rule_id == "SG-PDPA-01")
        .unwrap();
    assert_eq!(pdpa.status, FindingStatus::Attention);
    assert!(pdpa.detail.contains("1 of 1"));
    assert_eq!(status_of(&report, "SG-PDPA-02"), FindingStatus::NeedsReview);
    assert_eq!(status_of(&report, "SG-REC-01"), FindingStatus::NeedsReview);
    let contract = report
        .findings
        .iter()
        .find(|f| f.rule_id == "SG-CON-01")
        .unwrap();
    assert_eq!(contract.status, FindingStatus::Attention);
    assert!(contract.detail.contains("confidentiality"));
    assert!(contract.detail.contains("liability"));
    let cross = report
        .findings
        .iter()
        .find(|f| f.rule_id == "SG-COV-02")
        .unwrap();
    assert_eq!(cross.status, FindingStatus::NeedsReview);
    assert!(cross.detail.contains("Berlin GmbH (DE)"));
    assert!(matches!(
        status_of(&report, "SG-CIT-01"),
        FindingStatus::Attention | FindingStatus::NeedsReview
    ));
    assert!(matches!(
        status_of(&report, "SG-ACRA-01"),
        FindingStatus::Attention | FindingStatus::NeedsReview
    ));
    assert!(matches!(
        status_of(&report, "SG-CIT-02"),
        FindingStatus::Attention | FindingStatus::Pass
    ));
    // Not GST-registered: the e-invoicing rule does not apply.
    assert!(report.findings.iter().all(|f| f.rule_id != "SG-GST-03"));
    // Every finding cites its source and names the facts it read.
    assert!(report
        .findings
        .iter()
        .all(|f| !f.source_title.is_empty() && !f.facts_used.is_empty()));
    assert!(report
        .findings
        .iter()
        .any(|f| f.source_url.contains("iras.gov.sg")));
    // The stand-in provider was consulted for the summary: disclosed, but a
    // stand-in never becomes a model summary.
    assert_eq!(report.provider_id, "local-drafter");
    assert!(report.model_summary.is_none());
    let workspace = store.load().unwrap();
    assert_eq!(workspace.compliance_reports.len(), 1);
    assert_eq!(
        workspace.disclosures.last().unwrap().task,
        "crew.compliance_checker"
    );
    let events = actions(&dir);
    assert_eq!(
        &events[events.len() - 2..],
        ["model.drafted", "compliance.checked"]
    );
    // A company check shows the model the company's findings, not any one
    // customer's record: the chain says so instead of naming a nil customer.
    assert_eq!(last_model_resource(&dir), "venture:profile");
    assert!(workspace.disclosures.last().unwrap().customer_id.is_nil());

    // Recording consent and registering for GST changes the facts and the
    // verdicts; the digest changes with them.
    store
        .update_customer(
            acme,
            CustomerInput {
                name: "Acme Ltd".into(),
                email: "alex@example.com".into(),
                personal_data_consent: true,
                ..CustomerInput::default()
            },
        )
        .unwrap();
    store
        .update_venture_profile(profile("SG", true, Some(120_000_000), Some(1_705_276_800)))
        .unwrap();
    let second = store
        .run_compliance_check(ComplianceSubject::default(), "en")
        .unwrap();
    assert_ne!(second.facts_digest, report.facts_digest);
    assert_eq!(status_of(&second, "SG-GST-01"), FindingStatus::Pass);
    assert_eq!(status_of(&second, "SG-PDPA-01"), FindingStatus::Pass);
    assert!(second.findings.iter().any(|f| f.rule_id == "SG-GST-03"));
}

#[test]
fn an_invoice_subject_checks_essentials_and_gst_particulars() {
    let (dir, store) = store();
    store
        .update_venture_profile(profile("SG", true, None, Some(1_705_276_800)))
        .unwrap();
    let workspace = store
        .add_customer("Acme Ltd", "alex@example.com", "")
        .unwrap();
    let customer_id = workspace.customers[0].id;
    let workspace = store
        .create_document(DocumentKind::Invoice, customer_id, Some(350_000), "en")
        .unwrap();
    let invoice_id = workspace.documents[0].id;

    let report = store
        .run_compliance_check(
            ComplianceSubject {
                document_id: Some(invoice_id),
            },
            "en",
        )
        .unwrap();
    assert_eq!(report.subject, format!("document:{invoice_id}"));
    // The model saw this invoice's findings: the disclosure names its
    // customer, on the chain and in the founder's log.
    assert_eq!(last_model_resource(&dir), format!("customer:{customer_id}"));
    assert_eq!(
        store
            .load()
            .unwrap()
            .disclosures
            .last()
            .unwrap()
            .customer_id,
        customer_id
    );
    // Document reports carry only document-level rules.
    let ids: Vec<&str> = report.findings.iter().map(|f| f.rule_id.as_str()).collect();
    assert!(ids.contains(&"SG-INV-01") && ids.contains(&"SG-GST-02"));
    assert!(!ids.contains(&"SG-GST-01") && !ids.contains(&"SG-PDPA-01"));
    assert_eq!(status_of(&report, "SG-INV-01"), FindingStatus::Pass);
    // GST-registered with no UEN and no GST line on the invoice → action.
    let gst = report
        .findings
        .iter()
        .find(|f| f.rule_id == "SG-GST-02")
        .unwrap();
    assert_eq!(gst.status, FindingStatus::Attention);
    assert!(gst.detail.contains("UEN"));

    // Add the registration number and a GST statement: the remaining
    // particulars still need a human against the IRAS list.
    store
        .update_venture_profile(VentureProfileInput {
            uen: "202401234A".into(),
            ..profile("SG", true, None, Some(1_705_276_800))
        })
        .unwrap();
    let body = format!(
        "{}\nGST: to be added per IRAS tax invoice rules.",
        workspace.documents[0].body
    );
    store
        .update_document(
            invoice_id,
            "Invoice — Acme Consulting to Acme Ltd",
            &body,
            Some(350_000),
        )
        .unwrap();
    let report = store
        .run_compliance_check(
            ComplianceSubject {
                document_id: Some(invoice_id),
            },
            "zh",
        )
        .unwrap();
    assert_eq!(status_of(&report, "SG-GST-02"), FindingStatus::NeedsReview);
    assert_eq!(report.overall, "needs_review");
    assert!(store
        .run_compliance_check(
            ComplianceSubject {
                document_id: Some(uuid::Uuid::new_v4())
            },
            "en"
        )
        .is_err());
}

#[test]
fn rule_search_returns_citations_and_the_pack_declares_its_review_status() {
    let (_dir, store) = store();
    let hits = store.search_rules("GST registration threshold", "en");
    assert_eq!(hits[0].rule_id, "SG-GST-01");
    assert!(hits[0].source_url.starts_with("https://www.iras.gov.sg/"));
    assert!(hits[0].matched.contains(&"gst".to_owned()));
    assert!(store.search_rules("", "en").is_empty());
    let packs = packs();
    assert_eq!(packs.len(), 1);
    assert!(packs[0].review_status.contains("not legal or tax advice"));
    assert!(packs[0]
        .rules
        .iter()
        .all(|rule| !rule.source.url.is_empty()));
    assert!(packs[0].rules.iter().any(|rule| rule.basis == "demo_rule"));
}

#[test]
fn a_model_summary_is_kept_only_when_it_cites_retrieved_rules() {
    let (dir, store) = store();
    store
        .update_venture_profile(profile("SG", false, Some(120_000_000), Some(1_705_276_800)))
        .unwrap();
    let good = serde_json::json!({
        "response": serde_json::json!({ "summary": "Register for GST soon (SG-GST-01). Nothing here is legal advice." }).to_string(),
        "done": true
    })
    .to_string();
    let bad = serde_json::json!({
        "response": serde_json::json!({ "summary": "You are fully compliant per SG-MAGIC-42." }).to_string(),
        "done": true
    })
    .to_string();
    let port = fake_ollama(vec![
        r#"{"models":[]}"#.to_owned(),
        good,
        r#"{"models":[]}"#.to_owned(),
        bad,
    ]);
    std::fs::write(
        dir.path().join(MODEL_CONFIG_FILE),
        format!(r#"{{"ollama":{{"enabled":true,"base_url":"http://127.0.0.1:{port}","model":"test-model"}}}}"#),
    )
    .unwrap();

    let report = store
        .run_compliance_check(ComplianceSubject::default(), "en")
        .unwrap();
    assert!(report.model_backed);
    assert_eq!(report.provider_id, "ollama:test-model");
    assert!(report
        .model_summary
        .as_deref()
        .unwrap()
        .contains("SG-GST-01"));
    // The model's summary never changes a deterministic verdict.
    assert_eq!(status_of(&report, "SG-GST-01"), FindingStatus::Attention);

    let report = store
        .run_compliance_check(ComplianceSubject::default(), "en")
        .unwrap();
    assert!(
        !report.model_backed,
        "a summary citing an unknown rule is dropped"
    );
    assert!(report.model_summary.is_none());
    assert_eq!(report.overall, "attention");
}

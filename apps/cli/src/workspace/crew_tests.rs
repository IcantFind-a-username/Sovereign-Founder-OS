use super::crew_roles::{build_input, parse_model_change, prompt_for};
use super::test_support::fake_ollama;
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
    ledger.events().iter().map(|e| e.action.clone()).collect()
}

/// A GST-registered Singapore consultancy with one lead and discovery notes.
fn seeded() -> (tempfile::TempDir, Store, Uuid) {
    let (dir, store) = store();
    store
        .update_venture_profile(VentureProfileInput {
            name: "Acme Consulting".into(),
            service: "Reporting clarity sprints".into(),
            jurisdiction: "SG".into(),
            gst_registered: true,
            ..VentureProfileInput::default()
        })
        .unwrap();
    let workspace = store
        .add_customer("Acme Ltd", "alex@example.com", "met at expo")
        .unwrap();
    let customer_id = workspace.customers[0].id;
    store
        .update_customer(
            customer_id,
            CustomerInput {
                name: "Acme Ltd".into(),
                email: "alex@example.com".into(),
                notes: "met at expo".into(),
                discovery_notes: "Weekly reporting takes six hours. Budget is SGD 3,000–5,000. Finance must approve before we start.".into(),
                stage: None,
                jurisdiction: "SG".into(),
                personal_data_consent: true,
            },
        )
        .unwrap();
    (dir, store, customer_id)
}

fn hired(store: &Store, role: RoleId) -> Uuid {
    let workspace = store.hire_employee(role, "Ada").unwrap();
    workspace
        .employees
        .iter()
        .find(|employee| employee.role == role)
        .unwrap()
        .id
}

fn customer_subject(customer_id: Uuid) -> RunSubject {
    RunSubject {
        customer_id: Some(customer_id),
        document_id: None,
        project_id: None,
    }
}

/// An accepted, sent offer for the seeded lead: the state the planner needs.
fn accepted_offer(store: &Store, customer_id: Uuid) -> Uuid {
    let workspace = store
        .create_document(DocumentKind::Offer, customer_id, Some(400_000), "en")
        .unwrap();
    let offer_id = workspace.documents.last().unwrap().id;
    let workspace = store.request_send(offer_id).unwrap();
    let approval = workspace
        .approvals
        .iter()
        .find(|approval| approval.document_id == offer_id)
        .unwrap()
        .id;
    store.decide(approval, true).unwrap();
    store.record_offer_accepted(offer_id).unwrap();
    offer_id
}

#[test]
fn hiring_is_audited_and_one_active_employee_per_role() {
    let (dir, store, _) = seeded();
    assert_eq!(role_cards().len(), 6);
    assert_eq!(RoleId::parse("analyst"), Some(RoleId::Analyst));
    assert_eq!(RoleId::parse("boss"), None);

    let workspace = store.hire_employee(RoleId::Analyst, "Ada").unwrap();
    assert_eq!(workspace.employees[0].status, EmployeeStatus::Hired);
    assert_eq!(workspace.employees[0].runs, 0);
    let error = store.hire_employee(RoleId::Analyst, "Bob").unwrap_err();
    assert!(error.to_string().contains("already hired"));

    let employee_id = workspace.employees[0].id;
    let workspace = store
        .set_employee_status(employee_id, EmployeeStatus::Paused)
        .unwrap();
    assert_eq!(workspace.employees[0].status, EmployeeStatus::Paused);
    // Re-hiring a paused role resumes it instead of duplicating it.
    let workspace = store.hire_employee(RoleId::Analyst, "Ada").unwrap();
    assert_eq!(workspace.employees.len(), 1);
    assert_eq!(workspace.employees[0].status, EmployeeStatus::Hired);
    assert!(store.hire_employee(RoleId::Analyst, "").is_err());

    let events = actions(&dir);
    assert!(events.contains(&"employee.hired".to_owned()));
    assert_eq!(events.iter().filter(|a| *a == "employee.status").count(), 2);
}

#[test]
fn analyst_run_creates_a_pending_decision_with_a_disclosure_and_no_state_change() {
    let (dir, store, customer_id) = seeded();
    let employee_id = hired(&store, RoleId::Analyst);
    let before = store.load().unwrap();

    let decision = store
        .run_employee(employee_id, customer_subject(customer_id), "en")
        .unwrap();
    assert_eq!(decision.status, DecisionStatus::Pending);
    assert_eq!(decision.role, RoleId::Analyst);
    assert!(!decision.model_backed, "no model.json: the template path");
    assert_eq!(decision.provider_id, "local-drafter");
    assert!(decision
        .evidence
        .contains(&"customer.discovery_notes".to_owned()));
    assert!(decision
        .title
        .starts_with("Requirements Analyst · Acme Ltd"));
    match &decision.change {
        ProposedChange::DiscoverySummary {
            customer_id: subject,
            problems,
            constraints,
            budget,
            open_questions,
            assumptions,
        } => {
            assert_eq!(*subject, customer_id);
            assert!(problems.iter().any(|p| p.contains("six hours")));
            assert!(constraints
                .iter()
                .any(|c| c.contains("Finance must approve")));
            assert!(budget.contains("3,000"));
            assert_eq!(open_questions.len(), 3);
            assert!(!assumptions.is_empty());
        }
        other => panic!("unexpected change {other:?}"),
    }

    let after = store.load().unwrap();
    assert_eq!(
        after.customers[0].discovery_notes,
        before.customers[0].discovery_notes
    );
    assert_eq!(after.decisions.len(), 1);
    assert_eq!(after.disclosures.len(), 1);
    assert_eq!(after.disclosures[0].task, "crew.analyst");
    assert!(after.disclosures[0].stayed_local);
    assert_eq!(after.employees[0].runs, 1);
    let events = actions(&dir);
    let tail = &events[events.len() - 3..];
    assert_eq!(tail, ["model.drafted", "employee.ran", "decision.proposed"]);

    assert!(store
        .run_employee(employee_id, RunSubject::default(), "en")
        .is_err());
    assert_eq!(store.command_center().unwrap().counts.proposals_pending, 1);
    assert!(store
        .command_center()
        .unwrap()
        .guidance
        .iter()
        .any(|g| g.kind == "decide_proposals" && g.count == 1));
}

#[test]
fn approving_a_discovery_summary_appends_notes_and_is_audited() {
    let (dir, store, customer_id) = seeded();
    let employee_id = hired(&store, RoleId::Analyst);
    let suggestions = store.work_suggestions().unwrap();
    assert!(suggestions
        .iter()
        .any(|s| s.role == RoleId::Analyst && s.reason == "analyse_discovery"));

    let decision = store
        .run_employee(employee_id, customer_subject(customer_id), "en")
        .unwrap();
    let workspace = store.decide_proposal(decision.id, true).unwrap();
    assert_eq!(workspace.decisions[0].status, DecisionStatus::Approved);
    assert_eq!(
        workspace.decisions[0].outcome.as_deref(),
        Some(format!("customer:{customer_id}").as_str())
    );
    let notes = &workspace.customers[0].discovery_notes;
    assert!(notes.starts_with("Weekly reporting takes six hours."));
    assert!(notes.contains("--- Discovery summary (AI Analyst, "));
    assert!(notes.contains("Budget: "));
    let events = actions(&dir);
    assert_eq!(
        &events[events.len() - 2..],
        ["decision.approved", "customer.update"]
    );
    let error = store.decide_proposal(decision.id, true).unwrap_err();
    assert!(error.to_string().contains("already made"));
    assert!(!store
        .work_suggestions()
        .unwrap()
        .iter()
        .any(|s| s.role == RoleId::Analyst));
}

#[test]
fn proposal_writer_creates_a_draft_offer_on_approval_only() {
    let (dir, store, customer_id) = seeded();
    let employee_id = hired(&store, RoleId::ProposalWriter);
    assert!(store
        .work_suggestions()
        .unwrap()
        .iter()
        .any(|s| s.reason == "draft_proposal" && s.subject_name == "Acme Ltd"));

    let decision = store
        .run_employee(employee_id, customer_subject(customer_id), "en")
        .unwrap();
    match &decision.change {
        ProposedChange::OfferDraft {
            title,
            amount_cents,
            body,
            ..
        } => {
            assert!(title.contains("Acme Ltd"));
            assert_eq!(*amount_cents, Some(400_000), "midpoint of 3,000–5,000");
            assert!(body.contains("Scope"));
        }
        other => panic!("unexpected change {other:?}"),
    }
    assert!(store.load().unwrap().documents.is_empty());

    let workspace = store.decide_proposal(decision.id, true).unwrap();
    assert_eq!(workspace.documents.len(), 1);
    let offer = &workspace.documents[0];
    assert_eq!(offer.kind, DocumentKind::Offer);
    assert_eq!(offer.status, DocumentStatus::Draft);
    assert_eq!(offer.amount_cents, Some(400_000));
    assert!(offer.body.contains("Assumptions"));
    assert_eq!(
        workspace.decisions[0].outcome.as_deref(),
        Some(format!("document:{}", offer.id).as_str())
    );
    let events = actions(&dir);
    assert_eq!(
        &events[events.len() - 2..],
        ["decision.approved", "document.draft"]
    );
    // The usual send path still applies to the employee's draft.
    store.request_send(offer.id).unwrap();
}

#[test]
fn delivery_planner_opens_a_project_with_dated_tasks() {
    let (dir, store, customer_id) = seeded();
    let offer_id = accepted_offer(&store, customer_id);
    let employee_id = hired(&store, RoleId::DeliveryPlanner);
    let suggestion = store
        .work_suggestions()
        .unwrap()
        .into_iter()
        .find(|s| s.reason == "plan_delivery")
        .unwrap();
    assert_eq!(suggestion.subject.document_id, Some(offer_id));

    let decision = store
        .run_employee(
            employee_id,
            RunSubject {
                customer_id: Some(customer_id),
                document_id: Some(offer_id),
                project_id: None,
            },
            "en",
        )
        .unwrap();
    match &decision.change {
        ProposedChange::DeliveryPlan {
            tasks,
            acceptance_criteria,
            offer_id: linked,
            ..
        } => {
            assert_eq!(tasks.len(), 6);
            assert_eq!(acceptance_criteria.len(), 3);
            assert_eq!(*linked, Some(offer_id));
        }
        other => panic!("unexpected change {other:?}"),
    }
    let workspace = store.decide_proposal(decision.id, true).unwrap();
    assert_eq!(workspace.projects.len(), 1);
    let project = &workspace.projects[0];
    assert_eq!(project.status, ProjectStatus::Active);
    assert_eq!(project.offer_id, Some(offer_id));
    assert_eq!(project.budget_cents, Some(400_000));
    assert_eq!(project.acceptance_criteria.len(), 3);
    assert_eq!(workspace.tasks.len(), 6);
    assert!(workspace.tasks.iter().all(
        |task| task.origin == "employee:delivery_planner" && task.due_at.unwrap() > unix_now()
    ));
    let events = actions(&dir);
    assert_eq!(events.iter().filter(|a| *a == "task.create").count(), 6);
    assert!(events.contains(&"project.create".to_owned()));
    assert!(!store
        .work_suggestions()
        .unwrap()
        .iter()
        .any(|s| s.reason == "plan_delivery"));
}

#[test]
fn invoice_clerk_needs_a_finished_project_and_raises_a_dated_draft() {
    let (_dir, store, customer_id) = seeded();
    let offer_id = accepted_offer(&store, customer_id);
    let workspace = store
        .add_project(
            customer_id,
            "Reporting clarity sprint",
            Some(offer_id),
            None,
        )
        .unwrap();
    let project_id = workspace.projects[0].id;
    let employee_id = hired(&store, RoleId::InvoiceClerk);
    let error = store
        .run_employee(employee_id, customer_subject(customer_id), "en")
        .unwrap_err();
    assert!(error.to_string().contains("no finished project"));

    store
        .set_project_status(project_id, ProjectStatus::Done)
        .unwrap();
    assert!(store
        .work_suggestions()
        .unwrap()
        .iter()
        .any(|s| s.reason == "raise_invoice" && s.subject.project_id == Some(project_id)));
    let decision = store
        .run_employee(
            employee_id,
            RunSubject {
                customer_id: None,
                document_id: None,
                project_id: Some(project_id),
            },
            "zh",
        )
        .unwrap();
    match &decision.change {
        ProposedChange::InvoiceDraft {
            amount_cents,
            due_in_days,
            body,
            ..
        } => {
            assert_eq!(*amount_cents, 400_000, "the accepted offer's amount");
            assert_eq!(*due_in_days, 30);
            assert!(body.contains("GST"));
        }
        other => panic!("unexpected change {other:?}"),
    }
    let workspace = store.decide_proposal(decision.id, true).unwrap();
    let invoice = workspace
        .documents
        .iter()
        .find(|document| document.kind == DocumentKind::Invoice)
        .unwrap();
    assert_eq!(invoice.status, DocumentStatus::Draft);
    assert_eq!(invoice.project_id, Some(project_id));
    let due = invoice.due_at.unwrap();
    assert!(due > unix_now() + 29 * 86_400 && due <= unix_now() + 30 * 86_400 + 5);
    assert!(!store
        .command_center()
        .unwrap()
        .guidance
        .iter()
        .any(|g| g.kind == "invoice_done_project"));
}

#[test]
fn quality_checker_findings_are_recorded_without_changing_the_draft() {
    let (_dir, store, customer_id) = seeded();
    let workspace = store
        .create_document(DocumentKind::Offer, customer_id, None, "en")
        .unwrap();
    let document_id = workspace.documents[0].id;
    let employee_id = hired(&store, RoleId::QualityChecker);
    assert!(store
        .work_suggestions()
        .unwrap()
        .iter()
        .any(|s| s.reason == "check_draft" && s.subject.document_id == Some(document_id)));

    let decision = store
        .run_employee(
            employee_id,
            RunSubject {
                customer_id: None,
                document_id: Some(document_id),
                project_id: None,
            },
            "en",
        )
        .unwrap();
    let findings = match &decision.change {
        ProposedChange::ReviewFindings { findings, .. } => findings.clone(),
        other => panic!("unexpected change {other:?}"),
    };
    // The template offer says "to be confirmed": a placeholder finding.
    assert!(findings.iter().any(|f| f.kind == "placeholder"));
    let before = store.load().unwrap().documents[0].clone();
    let workspace = store.decide_proposal(decision.id, true).unwrap();
    assert_eq!(workspace.documents[0].body, before.body);
    assert_eq!(workspace.documents[0].revision, 1);
    assert!(workspace.decisions[0]
        .outcome
        .as_deref()
        .unwrap()
        .starts_with("acknowledged:"));
    assert!(!store
        .work_suggestions()
        .unwrap()
        .iter()
        .any(|s| s.reason == "check_draft"));
}

#[test]
fn rejection_records_the_decision_and_changes_nothing() {
    let (dir, store, customer_id) = seeded();
    let employee_id = hired(&store, RoleId::ProposalWriter);
    let decision = store
        .run_employee(employee_id, customer_subject(customer_id), "en")
        .unwrap();
    let workspace = store.decide_proposal(decision.id, false).unwrap();
    assert_eq!(workspace.decisions[0].status, DecisionStatus::Rejected);
    assert!(workspace.decisions[0].decided_at.is_some());
    assert!(workspace.documents.is_empty());
    assert_eq!(actions(&dir).last().unwrap(), "decision.rejected");
    assert_eq!(store.command_center().unwrap().counts.proposals_pending, 0);
}

#[test]
fn a_paused_employee_cannot_run_and_the_compliance_checker_reports_through_a_decision() {
    let (_dir, store, customer_id) = seeded();
    let analyst = hired(&store, RoleId::Analyst);
    store
        .set_employee_status(analyst, EmployeeStatus::Paused)
        .unwrap();
    let error = store
        .run_employee(analyst, customer_subject(customer_id), "en")
        .unwrap_err();
    assert!(error.to_string().contains("paused"));
    let checker = hired(&store, RoleId::ComplianceChecker);
    let decision = store
        .run_employee(checker, RunSubject::default(), "en")
        .unwrap();
    let report_id = match decision.change {
        ProposedChange::ComplianceReport { report_id } => report_id,
        other => panic!("unexpected change {other:?}"),
    };
    let workspace = store.load().unwrap();
    assert_eq!(workspace.compliance_reports.last().unwrap().id, report_id);
    let workspace = store.decide_proposal(decision.id, true).unwrap();
    assert!(workspace
        .decisions
        .last()
        .unwrap()
        .outcome
        .as_deref()
        .unwrap()
        .starts_with("acknowledged:compliance_report:"));
    assert!(store
        .run_employee(Uuid::new_v4(), RunSubject::default(), "en")
        .is_err());
}

#[test]
fn model_output_is_validated_before_it_becomes_a_proposal() {
    let (_dir, store, customer_id) = seeded();
    let workspace = store.load().unwrap();
    let input = build_input(
        &workspace,
        RoleId::ProposalWriter,
        customer_subject(customer_id),
        "en",
    )
    .unwrap();
    assert!(parse_model_change(&input, r#"{"title":"x","body":"y","amount":"abc"}"#).is_none());
    assert!(parse_model_change(
        &input,
        "prose {\"title\":\"x\",\"body\":\"y\",\"amount\":null} more"
    )
    .is_some());
    assert!(parse_model_change(&input, r#"{"title":"","body":"y"}"#).is_none());
    let too_many = format!(
        r#"{{"title":"x","body":"y","assumptions":[{}]}}"#,
        (0..13).map(|_| "\"a\"").collect::<Vec<_>>().join(",")
    );
    assert!(parse_model_change(&input, &too_many).is_none());
    assert!(parse_model_change(&input, "no json here").is_none());

    let plan_input = build_input(
        &workspace,
        RoleId::DeliveryPlanner,
        customer_subject(customer_id),
        "en",
    )
    .unwrap();
    assert!(parse_model_change(
        &plan_input,
        r#"{"project_name":"p","tasks":[{"title":"t","due_in_days":0}]}"#
    )
    .is_none());
    assert!(parse_model_change(
        &plan_input,
        r#"{"project_name":"p","tasks":[{"title":"t","due_in_days":3}]}"#
    )
    .is_some());

    let offer = store
        .create_document(DocumentKind::Offer, customer_id, None, "en")
        .unwrap()
        .documents[0]
        .id;
    let workspace = store.load().unwrap();
    let review_input = build_input(
        &workspace,
        RoleId::QualityChecker,
        RunSubject {
            customer_id: None,
            document_id: Some(offer),
            project_id: None,
        },
        "en",
    )
    .unwrap();
    assert!(parse_model_change(
        &review_input,
        r#"{"findings":[{"kind":"silly","detail":"x"}]}"#
    )
    .is_none());
    assert!(parse_model_change(
        &review_input,
        r#"{"findings":[{"kind":"GAP","detail":"no price"}]}"#
    )
    .is_some());
}

#[test]
fn a_real_model_answer_is_validated_and_marked_model_backed() {
    let (_dir, store, customer_id) = seeded();
    let good = serde_json::json!({
        "response": serde_json::json!({
            "problems": ["Reporting takes too long"],
            "constraints": ["Finance approval needed"],
            "budget": "SGD 3,000–5,000",
            "open_questions": ["Who owns the report?"],
            "assumptions": ["One team is involved"]
        }).to_string(),
        "done": true
    })
    .to_string();
    let bad =
        serde_json::json!({ "response": "I cannot help with that.", "done": true }).to_string();
    let port = fake_ollama(vec![
        r#"{"models":[]}"#.to_owned(),
        good,
        r#"{"models":[]}"#.to_owned(),
        bad,
    ]);
    std::fs::write(
        _dir.path().join(MODEL_CONFIG_FILE),
        format!(r#"{{"ollama":{{"enabled":true,"base_url":"http://127.0.0.1:{port}","model":"test-model"}}}}"#),
    )
    .unwrap();
    let employee_id = hired(&store, RoleId::Analyst);

    let decision = store
        .run_employee(employee_id, customer_subject(customer_id), "en")
        .unwrap();
    assert!(decision.model_backed);
    assert_eq!(decision.provider_id, "ollama:test-model");
    assert_eq!(decision.provider_trust, "local");
    match &decision.change {
        ProposedChange::DiscoverySummary { problems, .. } => {
            assert_eq!(problems, &vec!["Reporting takes too long".to_owned()]);
        }
        other => panic!("unexpected change {other:?}"),
    }
    assert!(decision.summary.starts_with("The model found"));
    let disclosure = store.load().unwrap().disclosures.pop().unwrap();
    assert_eq!(disclosure.provider_id, "ollama:test-model");
    assert!(disclosure.stayed_local);

    // Unusable model text: the template draft is used and the decision says so.
    let decision = store
        .run_employee(employee_id, customer_subject(customer_id), "en")
        .unwrap();
    assert!(!decision.model_backed);
    assert_eq!(decision.provider_id, "ollama:test-model");
    assert!(decision.summary.contains("failed validation"));
}

/// The schema a prompt shows is itself an instruction, and a model copies its
/// shape. Writing the allowed values as `"kind":"gap"|"contradiction"|…` read
/// as a list of bare values: against qwen2.5:7b, four of six measured
/// validation failures came back as `{"placeholder","detail":…}` with the key
/// name dropped, and the whole finding list was refused for it.
///
/// So the enum belongs in the task text, and the schema shows one field of
/// one type. This test pins that split — the shape that misled the model must
/// not come back, and the four words must still be stated somewhere.
#[test]
fn the_quality_checker_prompt_states_its_enum_in_prose_not_in_the_schema() {
    let (_dir, store) = store();
    store.set_venture("Acme", "Service").unwrap();
    let workspace = store.add_customer("Acme Ltd", "", "notes").unwrap();
    let customer_id = workspace.customers[0].id;
    let workspace = store
        .create_document(DocumentKind::Offer, customer_id, None, "en")
        .unwrap();
    let document_id = workspace.documents[0].id;
    let input = build_input(
        &workspace,
        RoleId::QualityChecker,
        RunSubject {
            customer_id: None,
            document_id: Some(document_id),
            project_id: None,
        },
        "en",
    )
    .unwrap();
    let prompt = prompt_for(&input);

    assert!(
        prompt.contains(r#"{"findings":[{"kind":string,"detail":string}]}"#),
        "the schema no longer shows one typed field per key: {prompt}"
    );
    assert!(
        !prompt.contains(r#""kind":"gap""#),
        "the alternation that misled the model is back in the schema"
    );
    for word in ["gap", "contradiction", "placeholder", "risk"] {
        assert!(prompt.contains(word), "the prompt stopped naming {word}");
    }

    // And the parser still refuses the malformed shape, whatever the prompt
    // says — a prompt is a request, not a guarantee.
    assert!(parse_model_change(&input, r#"{"findings":[{"placeholder","detail":"x"}]}"#).is_none());
}

/// A model that answers, then repeats the same answer inside a code fence.
/// Measured against qwen2.5:7b; the two objects are byte-identical and both
/// valid, and spanning the first `{` to the last `}` produced a string that
/// is neither. The answer was thrown away and the template used instead.
#[test]
fn an_answer_repeated_after_itself_is_still_read() {
    let (_dir, store) = store();
    store.set_venture("Acme", "Service").unwrap();
    let workspace = store.add_customer("Acme Ltd", "", "notes").unwrap();
    let customer_id = workspace.customers[0].id;
    let workspace = store
        .create_document(DocumentKind::Offer, customer_id, None, "en")
        .unwrap();
    let document_id = workspace.documents[0].id;
    let input = build_input(
        &workspace,
        RoleId::QualityChecker,
        RunSubject {
            customer_id: None,
            document_id: Some(document_id),
            project_id: None,
        },
        "en",
    )
    .unwrap();

    let once = r#"{"findings":[{"kind":"gap","detail":"no timeline"}]}"#;
    let twice = format!("{once}```json\n{once}\n```");
    let parsed = parse_model_change(&input, &twice).expect("the repeated answer was discarded");
    match parsed {
        ProposedChange::ReviewFindings { findings, .. } => {
            assert_eq!(findings.len(), 1, "{findings:?}");
            assert_eq!(findings[0].kind, "gap");
        }
        other => panic!("wrong change: {other:?}"),
    }

    // Reading only the first value must not soften any check: a first value
    // that fails validation is still refused, whatever follows it.
    let bad_then_good = format!(r#"{{"findings":[{{"kind":"not-a-kind","detail":"x"}}]}}{once}"#);
    assert!(parse_model_change(&input, &bad_then_good).is_none());
}

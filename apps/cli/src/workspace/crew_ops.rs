//! Hiring, running, and deciding on AI employees. The control path is the
//! blueprint's: founder goal → allowed input snapshot → constrained runner →
//! structured proposal with evidence → founder decision → the exact change
//! applied under policy with signed evidence. Employees hold no keys and
//! change nothing until the founder approves.

use super::compliance::{ComplianceSubject, FindingStatus};
use super::crew_roles::{
    build_input, deterministic_change, parse_model_change, prompt_for, role_card, summarize_change,
};
use super::crew_types::*;
use super::model_config::providers_for;
use super::store::AuditEntry;
use super::util::{clean_text, now};
use super::*;

use sovereign_contracts::{AutomationLevel, DataClass};
use sovereign_model::{ModelGateway, ModelRequest};
use uuid::Uuid;

const DAY_SECONDS: i64 = 86_400;
const MODEL_OUTPUT_CEILING_CHARS: usize = 24_000;

fn invalid(message: &str) -> WorkspaceError {
    WorkspaceError::Invalid(message.to_owned())
}

impl Store {
    fn crew_gate(&self, operation: &str, resource: &str) -> Result<(), WorkspaceError> {
        let (allowed, _, reason) = self.check_policy(
            "crew",
            operation,
            resource,
            DataClass::Amber,
            AutomationLevel::L1Draft,
        );
        if allowed {
            Ok(())
        } else {
            Err(WorkspaceError::PolicyDenied(reason))
        }
    }

    /// Hire one employee per role. Re-hiring a paused role resumes it.
    pub fn hire_employee(&self, role: RoleId, name: &str) -> Result<Workspace, WorkspaceError> {
        let name = clean_text("name", name)?;
        self.crew_gate("hire", &format!("role:{}", role.as_str()))?;
        let mut workspace = self.load()?;
        let at = now();
        if let Some(existing) = workspace
            .employees
            .iter_mut()
            .find(|employee| employee.role == role)
        {
            if existing.status == EmployeeStatus::Hired {
                return Err(invalid("that role is already hired"));
            }
            existing.status = EmployeeStatus::Hired;
            existing.name = name.clone();
            let resource = format!("employee:{}", existing.id);
            self.commit(
                &workspace,
                vec![AuditEntry {
                    action: "employee.status".into(),
                    resource,
                    payload: serde_json::json!({ "status": "hired", "role": role.as_str() }),
                }],
            )?;
            return Ok(workspace);
        }
        if workspace.employees.len() >= MAX_EMPLOYEES {
            return Err(invalid("employee limit reached"));
        }
        let employee = Employee {
            id: Uuid::new_v4(),
            role,
            name: name.clone(),
            status: EmployeeStatus::Hired,
            hired_at: at,
            runs: 0,
            last_run_at: None,
        };
        let resource = format!("employee:{}", employee.id);
        workspace.employees.push(employee);
        self.commit(
            &workspace,
            vec![AuditEntry {
                action: "employee.hired".into(),
                resource,
                payload: serde_json::json!({ "role": role.as_str(), "name": name }),
            }],
        )?;
        Ok(workspace)
    }

    pub fn set_employee_status(
        &self,
        employee_id: Uuid,
        status: EmployeeStatus,
    ) -> Result<Workspace, WorkspaceError> {
        let resource = format!("employee:{employee_id}");
        self.crew_gate("update", &resource)?;
        let mut workspace = self.load()?;
        let employee = workspace
            .employees
            .iter_mut()
            .find(|employee| employee.id == employee_id)
            .ok_or_else(|| WorkspaceError::NotFound("employee".into()))?;
        if employee.status == status {
            return Ok(workspace);
        }
        employee.status = status;
        self.commit(
            &workspace,
            vec![AuditEntry {
                action: "employee.status".into(),
                resource,
                payload: serde_json::json!({ "status": status }),
            }],
        )?;
        Ok(workspace)
    }

    /// Run one employee on one subject. The result is a pending decision:
    /// the founder sees the exact proposed change, which facts were used,
    /// which provider produced it, and whether a real model or the template
    /// did. Business state is untouched until approval.
    pub fn run_employee(
        &self,
        employee_id: Uuid,
        subject: RunSubject,
        lang: &str,
    ) -> Result<Decision, WorkspaceError> {
        let workspace = self.load()?;
        let employee = workspace
            .employees
            .iter()
            .find(|employee| employee.id == employee_id)
            .cloned()
            .ok_or_else(|| WorkspaceError::NotFound("employee".into()))?;
        if employee.status != EmployeeStatus::Hired {
            return Err(invalid("this employee is paused"));
        }
        if employee.role == RoleId::ComplianceChecker {
            return self.run_compliance_employee(&employee, subject, lang);
        }
        let input = build_input(&workspace, employee.role, subject, lang)?;
        let resource = subject_resource(&subject);
        self.crew_gate(employee.role.as_str(), &resource)?;
        let zh = input.lang == "zh";

        let (mut change, mut summary) = deterministic_change(&input)?;
        let mut model_backed = false;

        // Consult the device's providers. A real model may replace the
        // template only when its output validates; stand-ins (which echo
        // the prompt) never do. Either way the disclosure is recorded.
        let gateway = ModelGateway::new(providers_for(&self.root)?);
        let request = ModelRequest {
            task: format!("crew.{}", employee.role.as_str()),
            prompt: prompt_for(&input),
            data_class: DataClass::Amber,
            max_output_chars: MODEL_OUTPUT_CEILING_CHARS,
        };
        let consulted = gateway.complete(&request).ok();
        let (provider_id, provider_trust, output_chars, failover_from) = match &consulted {
            Some((response, disclosure)) => {
                if response.provider_id.starts_with("ollama:") {
                    match parse_model_change(&input, &response.text) {
                        Some(parsed) => {
                            summary = summarize_change(&parsed, zh);
                            change = parsed;
                            model_backed = true;
                        }
                        None => summary.push_str(if zh {
                            " 模型输出未通过校验,改用模板草稿。"
                        } else {
                            " The model's output failed validation, so the template draft is used."
                        }),
                    }
                }
                (
                    response.provider_id.clone(),
                    format!("{:?}", response.provider_trust).to_lowercase(),
                    disclosure.output_chars,
                    disclosure
                        .skipped
                        .iter()
                        .map(SkippedProvider::from)
                        .collect::<Vec<_>>(),
                )
            }
            None => ("none".to_owned(), "none".to_owned(), 0, Vec::new()),
        };

        let mut persisted = self.load()?;
        if persisted.decisions.len() >= MAX_DECISIONS {
            return Err(invalid("decision limit reached"));
        }
        let at = now();
        let customer_id = input
            .customer
            .as_ref()
            .map(|customer| customer.id)
            .unwrap_or(Uuid::nil());
        let mut events = Vec::new();
        if consulted.is_some() {
            persisted.disclosures.push(ModelDisclosure {
                id: Uuid::new_v4(),
                at,
                customer_id,
                task: request.task.clone(),
                provider_id: provider_id.clone(),
                provider_trust: provider_trust.clone(),
                stayed_local: provider_trust == "local",
                data_class: "amber".into(),
                output_chars,
                failover_from: failover_from.clone(),
            });
            events.push(AuditEntry {
                action: "model.drafted".into(),
                resource: format!("customer:{customer_id}"),
                payload: serde_json::json!({
                    "task": request.task,
                    "provider": provider_id,
                    "provider_trust": provider_trust,
                    "data_class": "amber",
                    "output_chars": output_chars,
                    "failover_from": failover_from,
                }),
            });
        }
        let card = role_card(employee.role);
        let subject_name = input
            .document
            .as_ref()
            .map(|document| document.title.clone())
            .or_else(|| input.project.as_ref().map(|project| project.name.clone()))
            .or_else(|| {
                input
                    .customer
                    .as_ref()
                    .map(|customer| customer.name.clone())
            })
            .unwrap_or_else(|| input.venture_name.clone());
        let decision = Decision {
            id: Uuid::new_v4(),
            employee_id,
            role: employee.role,
            title: format!(
                "{} · {}",
                if zh { card.title_zh } else { card.title_en },
                subject_name
            ),
            summary,
            change,
            evidence: input.evidence.clone(),
            provider_id,
            provider_trust,
            model_backed,
            status: DecisionStatus::Pending,
            created_at: at,
            decided_at: None,
            outcome: None,
        };
        let staffed = persisted
            .employees
            .iter_mut()
            .find(|employee| employee.id == employee_id)
            .ok_or_else(|| WorkspaceError::NotFound("employee".into()))?;
        staffed.runs += 1;
        staffed.last_run_at = Some(at);
        events.push(AuditEntry {
            action: "employee.ran".into(),
            resource: format!("employee:{employee_id}"),
            payload: serde_json::json!({
                "role": employee.role.as_str(),
                "task": request.task,
                "provider": decision.provider_id,
                "model_backed": model_backed,
            }),
        });
        events.push(AuditEntry {
            action: "decision.proposed".into(),
            resource: format!("decision:{}", decision.id),
            payload: serde_json::json!({ "role": employee.role.as_str(), "kind": decision.change.kind() }),
        });
        persisted.decisions.push(decision.clone());
        self.commit(&persisted, events)?;
        Ok(decision)
    }

    /// The compliance checker runs the rule pack (which stores and audits
    /// its own report) and wraps the report in a decision so it lands in the
    /// same inbox as every other employee's work.
    fn run_compliance_employee(
        &self,
        employee: &Employee,
        subject: RunSubject,
        lang: &str,
    ) -> Result<Decision, WorkspaceError> {
        let zh = lang.starts_with("zh");
        let report = self.run_compliance_check(
            ComplianceSubject {
                document_id: subject.document_id,
            },
            lang,
        )?;
        let mut persisted = self.load()?;
        if persisted.decisions.len() >= MAX_DECISIONS {
            return Err(invalid("decision limit reached"));
        }
        let at = now();
        let attention = report
            .findings
            .iter()
            .filter(|finding| finding.status == FindingStatus::Attention)
            .count();
        let review = report
            .findings
            .iter()
            .filter(|finding| finding.status == FindingStatus::NeedsReview)
            .count();
        let card = role_card(employee.role);
        let subject_name = persisted
            .venture
            .as_ref()
            .map(|venture| venture.name.clone())
            .unwrap_or_default();
        let summary = match &report.model_summary {
            Some(text) => text.clone(),
            None => {
                if zh {
                    format!(
                        "覆盖:{};{} 项需要处理,{} 项需要复核。这是未经专业审阅的演示规则包,不是法律或税务意见。",
                        report.coverage, attention, review
                    )
                } else {
                    format!(
                        "Coverage: {}; {} finding(s) need action, {} need review. Unreviewed demo pack; not legal or tax advice.",
                        report.coverage, attention, review
                    )
                }
            }
        };
        let decision = Decision {
            id: Uuid::new_v4(),
            employee_id: employee.id,
            role: employee.role,
            title: format!(
                "{} · {}",
                if zh { card.title_zh } else { card.title_en },
                subject_name
            ),
            summary,
            change: ProposedChange::ComplianceReport {
                report_id: report.id,
            },
            evidence: vec![
                "venture.jurisdiction".into(),
                "venture.registration".into(),
                "customers.jurisdiction".into(),
                "customers.personal_data_consent".into(),
                "documents.sent".into(),
            ],
            provider_id: report.provider_id.clone(),
            provider_trust: if report.provider_id == "none" {
                "none".into()
            } else {
                "local".into()
            },
            model_backed: report.model_backed,
            status: DecisionStatus::Pending,
            created_at: at,
            decided_at: None,
            outcome: None,
        };
        let staffed = persisted
            .employees
            .iter_mut()
            .find(|candidate| candidate.id == employee.id)
            .ok_or_else(|| WorkspaceError::NotFound("employee".into()))?;
        staffed.runs += 1;
        staffed.last_run_at = Some(at);
        let events = vec![
            AuditEntry {
                action: "employee.ran".into(),
                resource: format!("employee:{}", employee.id),
                payload: serde_json::json!({
                    "role": employee.role.as_str(),
                    "task": "crew.compliance_checker",
                    "provider": decision.provider_id,
                    "model_backed": decision.model_backed,
                    "report_id": report.id,
                }),
            },
            AuditEntry {
                action: "decision.proposed".into(),
                resource: format!("decision:{}", decision.id),
                payload: serde_json::json!({ "role": employee.role.as_str(), "kind": decision.change.kind() }),
            },
        ];
        persisted.decisions.push(decision.clone());
        self.commit(&persisted, events)?;
        Ok(decision)
    }

    /// The founder decides. Approving applies exactly the recorded change in
    /// one audit-first commit together with the decision itself; rejecting
    /// records the decision and changes nothing else.
    pub fn decide_proposal(
        &self,
        decision_id: Uuid,
        approve: bool,
    ) -> Result<Workspace, WorkspaceError> {
        let resource = format!("decision:{decision_id}");
        self.crew_gate(if approve { "approve" } else { "reject" }, &resource)?;
        let mut workspace = self.load()?;
        let index = workspace
            .decisions
            .iter()
            .position(|decision| decision.id == decision_id)
            .ok_or_else(|| WorkspaceError::NotFound("decision".into()))?;
        if workspace.decisions[index].status != DecisionStatus::Pending {
            return Err(invalid("decision already made"));
        }
        let at = now();
        if !approve {
            let decision = &mut workspace.decisions[index];
            decision.status = DecisionStatus::Rejected;
            decision.decided_at = Some(at);
            let kind = decision.change.kind();
            self.commit(
                &workspace,
                vec![AuditEntry {
                    action: "decision.rejected".into(),
                    resource,
                    payload: serde_json::json!({ "kind": kind }),
                }],
            )?;
            return Ok(workspace);
        }
        let change = workspace.decisions[index].change.clone();
        let role = workspace.decisions[index].role;
        let (mut events, outcome) = apply_change(&mut workspace, &change, role, at)?;
        let decision = &mut workspace.decisions[index];
        decision.status = DecisionStatus::Approved;
        decision.decided_at = Some(at);
        decision.outcome = Some(outcome.clone());
        events.insert(
            0,
            AuditEntry {
                action: "decision.approved".into(),
                resource,
                payload: serde_json::json!({ "kind": change.kind(), "outcome": outcome }),
            },
        );
        self.commit(&workspace, events)?;
        Ok(workspace)
    }

    /// What each hired employee could do right now, from state alone. The
    /// Today view lists these; nothing runs until the founder asks.
    pub fn work_suggestions(&self) -> Result<Vec<WorkSuggestion>, WorkspaceError> {
        let workspace = self.load()?;
        let mut out = Vec::new();
        let approved = |role: RoleId, matches: &dyn Fn(&ProposedChange) -> bool| {
            workspace.decisions.iter().any(|decision| {
                decision.role == role
                    && decision.status != DecisionStatus::Rejected
                    && matches(&decision.change)
            })
        };
        for employee in workspace
            .employees
            .iter()
            .filter(|employee| employee.status == EmployeeStatus::Hired)
        {
            match employee.role {
                RoleId::Analyst => {
                    for customer in workspace.customers.iter().filter(|customer| {
                        !customer.discovery_notes.trim().is_empty()
                            && !approved(RoleId::Analyst, &|change| {
                                matches!(change, ProposedChange::DiscoverySummary { customer_id, .. } if *customer_id == customer.id)
                            })
                    }) {
                        out.push(suggestion(employee, customer.id, None, None, &customer.name, "analyse_discovery"));
                    }
                }
                RoleId::ProposalWriter => {
                    for customer in workspace.customers.iter().filter(|customer| {
                        (!customer.discovery_notes.trim().is_empty() || !customer.notes.trim().is_empty())
                            && !workspace.documents.iter().any(|document| {
                                document.customer_id == customer.id && document.kind == DocumentKind::Offer
                            })
                    }) {
                        out.push(suggestion(employee, customer.id, None, None, &customer.name, "draft_proposal"));
                    }
                }
                RoleId::DeliveryPlanner => {
                    for offer in workspace.documents.iter().filter(|document| {
                        document.kind == DocumentKind::Offer
                            && document.accepted_at.is_some()
                            && !workspace.projects.iter().any(|project| project.offer_id == Some(document.id))
                    }) {
                        out.push(suggestion(employee, offer.customer_id, Some(offer.id), None, &offer.title, "plan_delivery"));
                    }
                }
                RoleId::InvoiceClerk => {
                    for project in workspace.projects.iter().filter(|project| {
                        project.status == ProjectStatus::Done
                            && !workspace.documents.iter().any(|document| {
                                document.kind == DocumentKind::Invoice && document.project_id == Some(project.id)
                            })
                    }) {
                        out.push(suggestion(employee, project.customer_id, None, Some(project.id), &project.name, "raise_invoice"));
                    }
                }
                RoleId::QualityChecker => {
                    for document in workspace.documents.iter().filter(|document| {
                        document.status == DocumentStatus::Draft
                            && !approved(RoleId::QualityChecker, &|change| {
                                matches!(change, ProposedChange::ReviewFindings { document_id, .. } if *document_id == document.id)
                            })
                    }) {
                        out.push(suggestion(employee, document.customer_id, Some(document.id), None, &document.title, "check_draft"));
                    }
                }
                RoleId::ComplianceChecker => {
                    let recent = workspace.compliance_reports.last().is_some_and(|report| {
                        report.at >= chrono::Utc::now().timestamp() - 30 * DAY_SECONDS
                    });
                    if let Some(venture) = workspace
                        .venture
                        .as_ref()
                        .filter(|venture| !venture.jurisdiction.is_empty())
                    {
                        if !recent {
                            out.push(WorkSuggestion {
                                employee_id: employee.id,
                                role: employee.role,
                                subject: RunSubjectRef {
                                    customer_id: None,
                                    document_id: None,
                                    project_id: None,
                                },
                                subject_name: venture.name.clone(),
                                reason: "run_compliance",
                            });
                        }
                    }
                }
            }
        }
        out.truncate(20);
        Ok(out)
    }
}

fn suggestion(
    employee: &Employee,
    customer_id: Uuid,
    document_id: Option<Uuid>,
    project_id: Option<Uuid>,
    subject_name: &str,
    reason: &'static str,
) -> WorkSuggestion {
    WorkSuggestion {
        employee_id: employee.id,
        role: employee.role,
        subject: RunSubjectRef {
            customer_id: Some(customer_id),
            document_id,
            project_id,
        },
        subject_name: subject_name.to_owned(),
        reason,
    }
}

fn subject_resource(subject: &RunSubject) -> String {
    if let Some(id) = subject.document_id {
        format!("document:{id}")
    } else if let Some(id) = subject.project_id {
        format!("project:{id}")
    } else if let Some(id) = subject.customer_id {
        format!("customer:{id}")
    } else {
        "venture:profile".to_owned()
    }
}

/// Apply an approved change in memory and return the events that describe
/// it. The caller commits them with the decision in one audit-first write.
fn apply_change(
    workspace: &mut Workspace,
    change: &ProposedChange,
    role: RoleId,
    at: i64,
) -> Result<(Vec<AuditEntry>, String), WorkspaceError> {
    let origin = format!("employee:{}", role.as_str());
    match change {
        ProposedChange::DiscoverySummary {
            customer_id,
            problems,
            constraints,
            budget,
            open_questions,
            assumptions,
        } => {
            let date = chrono::DateTime::from_timestamp(at, 0)
                .map(|stamp| stamp.format("%Y-%m-%d").to_string())
                .unwrap_or_default();
            let mut section = format!("\n\n--- Discovery summary (AI Analyst, {date}) ---\n");
            for (heading, items) in [
                ("Problems", problems),
                ("Constraints", constraints),
                ("Open questions", open_questions),
                ("Assumptions", assumptions),
            ] {
                section.push_str(heading);
                section.push_str(":\n");
                if items.is_empty() {
                    section.push_str("- (none)\n");
                }
                for item in items {
                    section.push_str("- ");
                    section.push_str(item);
                    section.push('\n');
                }
            }
            section.push_str("Budget: ");
            section.push_str(budget);
            section.push('\n');
            let customer = workspace.customer_mut(*customer_id)?;
            let combined = format!("{}{}", customer.discovery_notes.trim_end(), section);
            if combined.len() > MAX_BODY_BYTES {
                return Err(invalid("discovery notes would exceed the size limit"));
            }
            customer.discovery_notes = combined;
            customer.updated_at = at;
            Ok((
                vec![AuditEntry {
                    action: "customer.update".into(),
                    resource: format!("customer:{customer_id}"),
                    payload: serde_json::json!({ "changed": ["discovery_notes"], "origin": origin }),
                }],
                format!("customer:{customer_id}"),
            ))
        }
        ProposedChange::OfferDraft {
            customer_id,
            title,
            body,
            amount_cents,
            assumptions,
        } => {
            let customer_name = workspace.customer(*customer_id)?.name.clone();
            if workspace.documents.len() >= MAX_DOCUMENTS {
                return Err(invalid("document limit reached"));
            }
            let mut full_body = body.clone();
            if !assumptions.is_empty() && !body.contains("Assumptions") && !body.contains("假设")
            {
                full_body.push_str("\n\nAssumptions\n");
                for assumption in assumptions {
                    full_body.push_str("- ");
                    full_body.push_str(assumption);
                    full_body.push('\n');
                }
            }
            let document = new_document(
                DocumentKind::Offer,
                *customer_id,
                title.clone(),
                full_body,
                *amount_cents,
                None,
                None,
                at,
            );
            let id = document.id;
            workspace.documents.push(document);
            Ok((
                vec![AuditEntry {
                    action: "document.draft".into(),
                    resource: format!("document:{id}"),
                    payload: serde_json::json!({
                        "kind": "offer", "title": title, "customer": customer_name,
                        "amount_cents": amount_cents, "origin": origin,
                    }),
                }],
                format!("document:{id}"),
            ))
        }
        ProposedChange::DeliveryPlan {
            customer_id,
            offer_id,
            project_id,
            project_name,
            tasks,
            acceptance_criteria,
        } => {
            workspace.customer(*customer_id)?;
            let mut events = Vec::new();
            let project_id = match project_id {
                Some(id) => {
                    let project = workspace.project_mut(*id)?;
                    if project.status == ProjectStatus::Done {
                        return Err(invalid("the project is closed"));
                    }
                    let mut added = false;
                    for criterion in acceptance_criteria {
                        if !project.acceptance_criteria.contains(criterion) {
                            project.acceptance_criteria.push(criterion.clone());
                            added = true;
                        }
                    }
                    if added {
                        project.updated_at = at;
                        events.push(AuditEntry {
                            action: "project.update".into(),
                            resource: format!("project:{id}"),
                            payload: serde_json::json!({ "changed": ["acceptance_criteria"], "origin": origin }),
                        });
                    }
                    *id
                }
                None => {
                    if workspace.projects.len() >= MAX_PROJECTS {
                        return Err(invalid("project limit reached"));
                    }
                    let budget_cents = offer_id
                        .and_then(|id| workspace.document(id).ok())
                        .and_then(|offer| offer.amount_cents);
                    let project = Project {
                        id: Uuid::new_v4(),
                        customer_id: *customer_id,
                        offer_id: *offer_id,
                        name: project_name.clone(),
                        status: ProjectStatus::Active,
                        budget_cents,
                        created_at: at,
                        updated_at: at,
                        done_at: None,
                        acceptance_criteria: acceptance_criteria.clone(),
                    };
                    let id = project.id;
                    workspace.projects.push(project);
                    events.push(AuditEntry {
                        action: "project.create".into(),
                        resource: format!("project:{id}"),
                        payload: serde_json::json!({ "name": project_name, "customer_id": customer_id, "origin": origin }),
                    });
                    id
                }
            };
            if workspace.tasks.len() + tasks.len() > MAX_TASKS {
                return Err(invalid("task limit reached"));
            }
            for planned in tasks {
                let task = Task {
                    id: Uuid::new_v4(),
                    project_id,
                    title: planned.title.clone(),
                    due_at: Some(at + i64::from(planned.due_in_days) * DAY_SECONDS),
                    done_at: None,
                    created_at: at,
                    origin: origin.clone(),
                };
                events.push(AuditEntry {
                    action: "task.create".into(),
                    resource: format!("task:{}", task.id),
                    payload: serde_json::json!({ "title": task.title, "project_id": project_id, "origin": origin }),
                });
                workspace.tasks.push(task);
            }
            Ok((events, format!("project:{project_id}")))
        }
        ProposedChange::InvoiceDraft {
            customer_id,
            project_id,
            title,
            body,
            amount_cents,
            due_in_days,
        } => {
            let customer_name = workspace.customer(*customer_id)?.name.clone();
            if let Some(project_id) = project_id {
                if workspace.project(*project_id)?.customer_id != *customer_id {
                    return Err(invalid("project does not belong to this customer"));
                }
            }
            if workspace.documents.len() >= MAX_DOCUMENTS {
                return Err(invalid("document limit reached"));
            }
            let document = new_document(
                DocumentKind::Invoice,
                *customer_id,
                title.clone(),
                body.clone(),
                Some(*amount_cents),
                Some(at + i64::from(*due_in_days) * DAY_SECONDS),
                *project_id,
                at,
            );
            let id = document.id;
            workspace.documents.push(document);
            Ok((
                vec![AuditEntry {
                    action: "document.draft".into(),
                    resource: format!("document:{id}"),
                    payload: serde_json::json!({
                        "kind": "invoice", "title": title, "customer": customer_name,
                        "amount_cents": amount_cents, "origin": origin,
                    }),
                }],
                format!("document:{id}"),
            ))
        }
        ProposedChange::ReviewFindings { document_id, .. } => {
            workspace.document(*document_id)?;
            Ok((Vec::new(), format!("acknowledged:document:{document_id}")))
        }
        ProposedChange::ComplianceReport { report_id } => Ok((
            Vec::new(),
            format!("acknowledged:compliance_report:{report_id}"),
        )),
    }
}

#[allow(clippy::too_many_arguments)]
fn new_document(
    kind: DocumentKind,
    customer_id: Uuid,
    title: String,
    body: String,
    amount_cents: Option<u64>,
    due_at: Option<i64>,
    project_id: Option<Uuid>,
    at: i64,
) -> Document {
    Document {
        id: Uuid::new_v4(),
        kind,
        customer_id,
        title,
        body,
        amount_cents,
        status: DocumentStatus::Draft,
        created_at: at,
        updated_at: at,
        revision: 1,
        due_at,
        project_id,
        accepted_at: None,
    }
}

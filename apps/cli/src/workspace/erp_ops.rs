//! Operations on the business graph: company profile, customer and draft
//! editing, projects and tasks, follow-ups, payments, and the read models
//! built from them. Same discipline as every other mutation: validate, ask
//! the deterministic policy engine, load, mutate, commit audit-first.

use super::store::AuditEntry;
use super::util::{clean_email, clean_optional_text, clean_text, now, storage};
use super::*;

use sovereign_audit_ledger::AuditLedger;
use sovereign_contracts::{AutomationLevel, DataClass};
use uuid::Uuid;

/// Origin tag for records the founder created directly.
pub(super) const ORIGIN_FOUNDER: &str = "founder";

fn invalid(message: &str) -> WorkspaceError {
    WorkspaceError::Invalid(message.to_owned())
}

/// Jurisdiction codes are founder-entered facts: empty (unknown) or a short
/// ISO-style alphabetic code, upper-cased. No inference from language or IP.
pub(super) fn clean_jurisdiction(value: &str) -> Result<String, WorkspaceError> {
    let value = value.trim().to_ascii_uppercase();
    if value.is_empty() {
        return Ok(value);
    }
    if !(2..=3).contains(&value.len()) || !value.bytes().all(|byte| byte.is_ascii_alphabetic()) {
        return Err(invalid("jurisdiction must be a 2–3 letter code"));
    }
    Ok(value)
}

fn clean_currency(value: &str) -> Result<String, WorkspaceError> {
    let value = value.trim().to_ascii_uppercase();
    if value.is_empty() {
        return Ok("SGD".to_owned());
    }
    if value.len() != 3 || !value.bytes().all(|byte| byte.is_ascii_alphabetic()) {
        return Err(invalid("currency must be a 3-letter code"));
    }
    Ok(value)
}

/// Document bodies may be long (a proposal), so they get their own ceiling;
/// control characters other than newline and tab are still refused.
pub(super) fn clean_body(value: &str) -> Result<String, WorkspaceError> {
    let value = value.trim();
    if value.len() > MAX_BODY_BYTES {
        return Err(invalid("body is too long"));
    }
    if value
        .chars()
        .any(|ch| ch.is_control() && ch != '\n' && ch != '\t')
    {
        return Err(invalid("body contains control characters"));
    }
    Ok(value.to_owned())
}

impl Store {
    /// Policy gate shared by every business-graph mutation: L1 drafts on
    /// Amber data, allowed by default policy but always evaluated so a
    /// future rule (or a hostile resource string) fails closed here.
    fn gate(&self, operation: &str, resource: &str) -> Result<(), WorkspaceError> {
        let (allowed, _, reason) = self.check_policy(
            "workspace",
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

    /// Replace the company profile. Registration facts are founder-entered
    /// inputs for the compliance checks; nothing here is verified against a
    /// registry.
    pub fn update_venture_profile(
        &self,
        input: VentureProfileInput,
    ) -> Result<Workspace, WorkspaceError> {
        let name = clean_text("name", &input.name)?;
        let service = clean_text("service", &input.service)?;
        let jurisdiction = clean_jurisdiction(&input.jurisdiction)?;
        let currency = clean_currency(&input.currency)?;
        let uen = clean_optional_text("uen", &input.uen)?;
        if uen.len() > 32 {
            return Err(invalid("uen is too long"));
        }
        let fiscal_year_end_month = input.fiscal_year_end_month.unwrap_or(12);
        if !(1..=12).contains(&fiscal_year_end_month) {
            return Err(invalid("fiscal_year_end_month must be 1-12"));
        }
        self.gate("update", "venture:profile")?;
        let mut workspace = self.load()?;
        workspace.venture = Some(Venture {
            name: name.clone(),
            service,
            updated_at: now(),
            jurisdiction: jurisdiction.clone(),
            currency,
            uen,
            gst_registered: input.gst_registered,
            incorporated_at: input.incorporated_at,
            fiscal_year_end_month,
            revenue_estimate_cents: input.revenue_estimate_cents,
        });
        self.commit(
            &workspace,
            vec![AuditEntry {
                action: "venture.update".into(),
                resource: "venture:profile".into(),
                payload: serde_json::json!({
                    "name": name,
                    "jurisdiction": jurisdiction,
                    "gst_registered": input.gst_registered,
                }),
            }],
        )?;
        Ok(workspace)
    }

    /// Edit a customer. A save is a complete statement of the record; a save
    /// that changes nothing commits nothing.
    pub fn update_customer(
        &self,
        customer_id: Uuid,
        input: CustomerInput,
    ) -> Result<Workspace, WorkspaceError> {
        let name = clean_text("name", &input.name)?;
        let email = clean_email(&input.email)?;
        let notes = clean_optional_text("notes", &input.notes)?;
        let discovery_notes = clean_body(&input.discovery_notes)?;
        let jurisdiction = clean_jurisdiction(&input.jurisdiction)?;
        let resource = format!("customer:{customer_id}");
        self.gate("update", &resource)?;
        let mut workspace = self.load()?;
        let customer = workspace.customer_mut(customer_id)?;
        let mut changed = Vec::new();
        if customer.name != name {
            customer.name = name;
            changed.push("name");
        }
        if customer.email != email {
            customer.email = email;
            changed.push("email");
        }
        if customer.notes != notes {
            customer.notes = notes;
            changed.push("notes");
        }
        if customer.discovery_notes != discovery_notes {
            customer.discovery_notes = discovery_notes;
            changed.push("discovery_notes");
        }
        if let Some(stage) = input.stage {
            if customer.stage != stage {
                customer.stage = stage;
                changed.push("stage");
            }
        }
        if customer.jurisdiction != jurisdiction {
            customer.jurisdiction = jurisdiction;
            changed.push("jurisdiction");
        }
        if customer.personal_data_consent != input.personal_data_consent {
            customer.personal_data_consent = input.personal_data_consent;
            changed.push("personal_data_consent");
        }
        if changed.is_empty() {
            return Ok(workspace);
        }
        customer.updated_at = now();
        self.commit(
            &workspace,
            vec![AuditEntry {
                action: "customer.update".into(),
                resource,
                payload: serde_json::json!({ "changed": changed }),
            }],
        )?;
        Ok(workspace)
    }

    /// Edit a draft. Anything past `Draft` is immutable: an approval or a
    /// send binds exact content, and that content must not drift afterwards.
    pub fn update_document(
        &self,
        document_id: Uuid,
        title: &str,
        body: &str,
        amount_cents: Option<u64>,
    ) -> Result<Workspace, WorkspaceError> {
        let title = clean_text("title", title)?;
        let body = clean_body(body)?;
        let resource = format!("document:{document_id}");
        self.gate("update", &resource)?;
        let mut workspace = self.load()?;
        let document = workspace.document_mut(document_id)?;
        if document.status != DocumentStatus::Draft {
            return Err(invalid("only drafts can be edited"));
        }
        if document.kind == DocumentKind::Invoice && amount_cents.is_none() {
            return Err(invalid("invoice needs an amount"));
        }
        let mut changed = Vec::new();
        if document.title != title {
            document.title = title;
            changed.push("title");
        }
        if document.body != body {
            document.body = body;
            changed.push("body");
        }
        if document.amount_cents != amount_cents {
            document.amount_cents = amount_cents;
            changed.push("amount_cents");
        }
        if changed.is_empty() {
            return Ok(workspace);
        }
        document.revision += 1;
        document.updated_at = now();
        let revision = document.revision;
        self.commit(
            &workspace,
            vec![AuditEntry {
                action: "document.update".into(),
                resource,
                payload: serde_json::json!({ "changed": changed, "revision": revision }),
            }],
        )?;
        Ok(workspace)
    }

    /// Record that the customer accepted a sent offer. This is the founder's
    /// attestation of a business fact; it promotes the contact to a customer
    /// and is the usual reason to open a project.
    pub fn record_offer_accepted(&self, document_id: Uuid) -> Result<Workspace, WorkspaceError> {
        let resource = format!("document:{document_id}");
        self.gate("update", &resource)?;
        let mut workspace = self.load()?;
        let document = workspace.document_mut(document_id)?;
        if document.kind != DocumentKind::Offer {
            return Err(invalid("only offers can be accepted"));
        }
        if !matches!(
            document.status,
            DocumentStatus::ApprovedPendingDelivery | DocumentStatus::Delivered
        ) {
            return Err(invalid("only a sent offer can be recorded as accepted"));
        }
        if document.accepted_at.is_some() {
            return Err(invalid("offer already accepted"));
        }
        let at = now();
        document.accepted_at = Some(at);
        let customer_id = document.customer_id;
        let customer = workspace.customer_mut(customer_id)?;
        let promoted = customer.stage != CustomerStage::Customer;
        if promoted {
            customer.stage = CustomerStage::Customer;
            customer.updated_at = at;
        }
        let mut events = vec![AuditEntry {
            action: "offer.accepted".into(),
            resource,
            payload: serde_json::json!({ "attested_by": "founder" }),
        }];
        if promoted {
            events.push(AuditEntry {
                action: "customer.update".into(),
                resource: format!("customer:{customer_id}"),
                payload: serde_json::json!({ "changed": ["stage"] }),
            });
        }
        self.commit(&workspace, events)?;
        Ok(workspace)
    }

    pub fn add_project(
        &self,
        customer_id: Uuid,
        name: &str,
        offer_id: Option<Uuid>,
        budget_cents: Option<u64>,
    ) -> Result<Workspace, WorkspaceError> {
        let name = clean_text("name", name)?;
        self.gate("create", &format!("customer:{customer_id}"))?;
        let mut workspace = self.load()?;
        workspace.customer(customer_id)?;
        if let Some(offer_id) = offer_id {
            let offer = workspace.document(offer_id)?;
            if offer.kind != DocumentKind::Offer || offer.customer_id != customer_id {
                return Err(invalid("offer does not belong to this customer"));
            }
        }
        if workspace.projects.len() >= MAX_PROJECTS {
            return Err(invalid("project limit reached"));
        }
        let at = now();
        let project = Project {
            id: Uuid::new_v4(),
            customer_id,
            offer_id,
            name: name.clone(),
            status: ProjectStatus::Proposed,
            budget_cents,
            created_at: at,
            updated_at: at,
            done_at: None,
        };
        let resource = format!("project:{}", project.id);
        workspace.projects.push(project);
        self.commit(
            &workspace,
            vec![AuditEntry {
                action: "project.create".into(),
                resource,
                payload: serde_json::json!({ "name": name, "customer_id": customer_id }),
            }],
        )?;
        Ok(workspace)
    }

    /// Move a project forward. Closing requires every task to be done: a
    /// completion claim must have something checkable behind it.
    pub fn set_project_status(
        &self,
        project_id: Uuid,
        status: ProjectStatus,
    ) -> Result<Workspace, WorkspaceError> {
        let resource = format!("project:{project_id}");
        self.gate("update", &resource)?;
        let mut workspace = self.load()?;
        let open_tasks = workspace
            .tasks
            .iter()
            .filter(|task| task.project_id == project_id && task.done_at.is_none())
            .count();
        let project = workspace.project_mut(project_id)?;
        if project.status == status {
            return Ok(workspace);
        }
        if project.status == ProjectStatus::Done {
            return Err(invalid("a closed project cannot change status"));
        }
        if status == ProjectStatus::Done && open_tasks > 0 {
            return Err(invalid("finish every task before closing the project"));
        }
        let at = now();
        project.status = status;
        project.updated_at = at;
        if status == ProjectStatus::Done {
            project.done_at = Some(at);
        }
        self.commit(
            &workspace,
            vec![AuditEntry {
                action: "project.status".into(),
                resource,
                payload: serde_json::json!({ "status": status }),
            }],
        )?;
        Ok(workspace)
    }

    pub fn add_task(
        &self,
        project_id: Uuid,
        title: &str,
        due_at: Option<i64>,
    ) -> Result<Workspace, WorkspaceError> {
        self.add_task_with_origin(project_id, title, due_at, ORIGIN_FOUNDER)
    }

    pub(super) fn add_task_with_origin(
        &self,
        project_id: Uuid,
        title: &str,
        due_at: Option<i64>,
        origin: &str,
    ) -> Result<Workspace, WorkspaceError> {
        let title = clean_text("title", title)?;
        let resource = format!("project:{project_id}");
        self.gate("create", &resource)?;
        let mut workspace = self.load()?;
        if workspace.project(project_id)?.status == ProjectStatus::Done {
            return Err(invalid("the project is closed"));
        }
        if workspace.tasks.len() >= MAX_TASKS {
            return Err(invalid("task limit reached"));
        }
        let task = Task {
            id: Uuid::new_v4(),
            project_id,
            title: title.clone(),
            due_at,
            done_at: None,
            created_at: now(),
            origin: origin.to_owned(),
        };
        let task_resource = format!("task:{}", task.id);
        workspace.tasks.push(task);
        self.commit(
            &workspace,
            vec![AuditEntry {
                action: "task.create".into(),
                resource: task_resource,
                payload: serde_json::json!({ "title": title, "project_id": project_id, "origin": origin }),
            }],
        )?;
        Ok(workspace)
    }

    pub fn complete_task(&self, task_id: Uuid) -> Result<Workspace, WorkspaceError> {
        let resource = format!("task:{task_id}");
        self.gate("update", &resource)?;
        let mut workspace = self.load()?;
        let task = workspace.task_mut(task_id)?;
        if task.done_at.is_some() {
            return Ok(workspace);
        }
        task.done_at = Some(now());
        self.commit(
            &workspace,
            vec![AuditEntry {
                action: "task.done".into(),
                resource,
                payload: serde_json::json!({ "attested_by": "founder" }),
            }],
        )?;
        Ok(workspace)
    }

    pub fn add_follow_up(
        &self,
        customer_id: Uuid,
        due_at: i64,
        note: &str,
    ) -> Result<Workspace, WorkspaceError> {
        self.add_follow_up_with_origin(customer_id, due_at, note, ORIGIN_FOUNDER)
    }

    pub(super) fn add_follow_up_with_origin(
        &self,
        customer_id: Uuid,
        due_at: i64,
        note: &str,
        origin: &str,
    ) -> Result<Workspace, WorkspaceError> {
        let note = clean_text("note", note)?;
        if due_at <= 0 {
            return Err(invalid("due_at must be a unix timestamp"));
        }
        let resource = format!("customer:{customer_id}");
        self.gate("create", &resource)?;
        let mut workspace = self.load()?;
        workspace.customer(customer_id)?;
        if workspace.follow_ups.len() >= MAX_FOLLOW_UPS {
            return Err(invalid("follow-up limit reached"));
        }
        let follow_up = FollowUp {
            id: Uuid::new_v4(),
            customer_id,
            due_at,
            note: note.clone(),
            done_at: None,
            created_at: now(),
            origin: origin.to_owned(),
        };
        let follow_resource = format!("follow_up:{}", follow_up.id);
        workspace.follow_ups.push(follow_up);
        self.commit(
            &workspace,
            vec![AuditEntry {
                action: "follow_up.create".into(),
                resource: follow_resource,
                payload: serde_json::json!({ "customer_id": customer_id, "due_at": due_at, "origin": origin }),
            }],
        )?;
        Ok(workspace)
    }

    pub fn complete_follow_up(&self, follow_up_id: Uuid) -> Result<Workspace, WorkspaceError> {
        let resource = format!("follow_up:{follow_up_id}");
        self.gate("update", &resource)?;
        let mut workspace = self.load()?;
        let follow_up = workspace.follow_up_mut(follow_up_id)?;
        if follow_up.done_at.is_some() {
            return Ok(workspace);
        }
        follow_up.done_at = Some(now());
        self.commit(
            &workspace,
            vec![AuditEntry {
                action: "follow_up.done".into(),
                resource,
                payload: serde_json::json!({ "attested_by": "founder" }),
            }],
        )?;
        Ok(workspace)
    }

    /// Record money received against an issued invoice. Founder attestation
    /// only; receipts can never exceed the invoice and never create revenue
    /// from an unsent draft.
    pub fn record_payment(
        &self,
        invoice_id: Uuid,
        amount_cents: u64,
        received_at: i64,
        note: &str,
    ) -> Result<Workspace, WorkspaceError> {
        let note = clean_optional_text("note", note)?;
        if amount_cents == 0 {
            return Err(invalid("payment must be positive"));
        }
        if received_at <= 0 {
            return Err(invalid("received_at must be a unix timestamp"));
        }
        let resource = format!("document:{invoice_id}");
        self.gate("update", &resource)?;
        let mut workspace = self.load()?;
        let invoice = workspace.document(invoice_id)?;
        if invoice.kind != DocumentKind::Invoice
            || !matches!(
                invoice.status,
                DocumentStatus::ApprovedPendingDelivery | DocumentStatus::Delivered
            )
        {
            return Err(invalid("only an issued invoice can receive payments"));
        }
        let total = invoice.amount_cents.unwrap_or(0);
        let paid: u64 = workspace
            .payments
            .iter()
            .filter(|payment| payment.invoice_id == invoice_id)
            .map(|payment| payment.amount_cents)
            .sum();
        let paid_total = paid
            .checked_add(amount_cents)
            .filter(|sum| *sum <= total)
            .ok_or_else(|| invalid("payment exceeds the invoice amount"))?;
        if workspace.payments.len() >= MAX_PAYMENTS {
            return Err(invalid("payment limit reached"));
        }
        workspace.payments.push(Payment {
            id: Uuid::new_v4(),
            invoice_id,
            amount_cents,
            received_at,
            note,
            created_at: now(),
        });
        self.commit(
            &workspace,
            vec![AuditEntry {
                action: "payment.record".into(),
                resource,
                payload: serde_json::json!({ "amount_cents": amount_cents, "paid_total_cents": paid_total }),
            }],
        )?;
        Ok(workspace)
    }

    /// Outstanding money per issued invoice.
    pub fn receivables(&self) -> Result<Vec<Receivable>, WorkspaceError> {
        Ok(receivable_rows(&self.load()?, now()))
    }

    /// Every signed event that touched a customer or their records, oldest
    /// first. Read straight from the ledger: the timeline can only contain
    /// what was actually signed.
    pub fn customer_timeline(
        &self,
        customer_id: Uuid,
    ) -> Result<Vec<TimelineEntry>, WorkspaceError> {
        let workspace = self.load()?;
        let customer = workspace.customer(customer_id)?;
        let mut resources = vec![format!("customer:{customer_id}")];
        let mut subjects = vec![(resources[0].clone(), customer.name.clone())];
        for document in workspace
            .documents
            .iter()
            .filter(|document| document.customer_id == customer_id)
        {
            let resource = format!("document:{}", document.id);
            subjects.push((resource.clone(), document.title.clone()));
            resources.push(resource);
        }
        for project in workspace
            .projects
            .iter()
            .filter(|project| project.customer_id == customer_id)
        {
            let resource = format!("project:{}", project.id);
            subjects.push((resource.clone(), project.name.clone()));
            resources.push(resource);
            for task in workspace
                .tasks
                .iter()
                .filter(|task| task.project_id == project.id)
            {
                let resource = format!("task:{}", task.id);
                subjects.push((resource.clone(), task.title.clone()));
                resources.push(resource);
            }
        }
        for follow_up in workspace
            .follow_ups
            .iter()
            .filter(|follow_up| follow_up.customer_id == customer_id)
        {
            let resource = format!("follow_up:{}", follow_up.id);
            subjects.push((resource.clone(), follow_up.note.clone()));
            resources.push(resource);
        }
        let ledger_path = self.root.join("ledger.json");
        if !ledger_path.exists() {
            return Ok(Vec::new());
        }
        let ledger =
            AuditLedger::load(&ledger_path, self.device.public_key_b64()).map_err(storage)?;
        Ok(ledger
            .events()
            .iter()
            .filter(|event| resources.contains(&event.resource))
            .map(|event| TimelineEntry {
                at: event.timestamp.timestamp(),
                action: event.action.clone(),
                resource: event.resource.clone(),
                subject: subjects
                    .iter()
                    .find(|(resource, _)| *resource == event.resource)
                    .map(|(_, subject)| subject.clone())
                    .unwrap_or_default(),
            })
            .collect())
    }
}

/// Pure derivation of receivables from state; shared with the Command Center.
pub(super) fn receivable_rows(workspace: &Workspace, now_unix: i64) -> Vec<Receivable> {
    workspace
        .documents
        .iter()
        .filter(|document| {
            document.kind == DocumentKind::Invoice
                && matches!(
                    document.status,
                    DocumentStatus::ApprovedPendingDelivery | DocumentStatus::Delivered
                )
        })
        .map(|invoice| {
            let amount_cents = invoice.amount_cents.unwrap_or(0);
            let paid_cents: u64 = workspace
                .payments
                .iter()
                .filter(|payment| payment.invoice_id == invoice.id)
                .map(|payment| payment.amount_cents)
                .sum();
            let outstanding_cents = amount_cents.saturating_sub(paid_cents);
            let status = if outstanding_cents == 0 {
                "paid"
            } else if invoice.due_at.is_some_and(|due| due < now_unix) {
                "overdue"
            } else if paid_cents > 0 {
                "partial"
            } else {
                "open"
            };
            Receivable {
                invoice_id: invoice.id,
                customer_id: invoice.customer_id,
                customer_name: workspace
                    .customer(invoice.customer_id)
                    .map(|customer| customer.name.clone())
                    .unwrap_or_default(),
                title: invoice.title.clone(),
                amount_cents,
                paid_cents,
                outstanding_cents,
                status: status.to_owned(),
                due_at: invoice.due_at,
            }
        })
        .collect()
}

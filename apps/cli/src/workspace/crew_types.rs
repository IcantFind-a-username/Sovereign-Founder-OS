//! AI employees: roles a founder can hire, the bounded proposals they make,
//! and the founder's decisions on those proposals. An employee never
//! changes business state itself. Every run ends in a `Decision` the founder
//! approves or rejects; only an approval applies the change, and it applies
//! exactly the deterministic change that was shown, nothing more.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The first roster. One model can serve every role; roles differ in what
/// they may read, what they must produce, and what they cannot decide.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoleId {
    /// Turns discovery notes into problems, constraints, budget, questions.
    Analyst,
    /// Drafts a versioned proposal with scope, amount, and assumptions.
    ProposalWriter,
    /// Turns an accepted offer into a project with tasks and acceptance criteria.
    DeliveryPlanner,
    /// Raises an invoice draft for finished work.
    InvoiceClerk,
    /// Finds gaps, contradictions, and placeholders in a draft.
    QualityChecker,
    /// Runs the jurisdiction rule pack and cites its sources.
    ComplianceChecker,
}

impl RoleId {
    pub const ALL: [RoleId; 6] = [
        RoleId::Analyst,
        RoleId::ProposalWriter,
        RoleId::DeliveryPlanner,
        RoleId::InvoiceClerk,
        RoleId::QualityChecker,
        RoleId::ComplianceChecker,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            RoleId::Analyst => "analyst",
            RoleId::ProposalWriter => "proposal_writer",
            RoleId::DeliveryPlanner => "delivery_planner",
            RoleId::InvoiceClerk => "invoice_clerk",
            RoleId::QualityChecker => "quality_checker",
            RoleId::ComplianceChecker => "compliance_checker",
        }
    }

    pub fn parse(text: &str) -> Option<RoleId> {
        RoleId::ALL.into_iter().find(|role| role.as_str() == text)
    }
}

/// The role card shown when hiring: what the employee reads, delivers, and
/// can never decide on its own. Static product copy, bilingual.
#[derive(Debug, Clone, Serialize)]
pub struct RoleCard {
    pub id: RoleId,
    pub title_en: &'static str,
    pub title_zh: &'static str,
    pub description_en: &'static str,
    pub description_zh: &'static str,
    pub reads: &'static str,
    pub delivers: &'static str,
    pub cannot: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmployeeStatus {
    Hired,
    Paused,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Employee {
    pub id: Uuid,
    pub role: RoleId,
    pub name: String,
    pub status: EmployeeStatus,
    pub hired_at: i64,
    #[serde(default)]
    pub runs: u32,
    #[serde(default)]
    pub last_run_at: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionStatus {
    Pending,
    Approved,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedTask {
    pub title: String,
    pub due_in_days: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewFinding {
    /// `gap`, `contradiction`, `placeholder`, or `risk`.
    pub kind: String,
    pub detail: String,
}

/// The exact, fully specified change an approval applies. Nothing is
/// re-derived at approval time: what the founder read is what happens.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProposedChange {
    /// Appends a dated, structured discovery summary to the customer's notes.
    DiscoverySummary {
        customer_id: Uuid,
        problems: Vec<String>,
        constraints: Vec<String>,
        budget: String,
        open_questions: Vec<String>,
        assumptions: Vec<String>,
    },
    /// Creates a draft offer; the founder edits and sends it as usual.
    OfferDraft {
        customer_id: Uuid,
        title: String,
        body: String,
        amount_cents: Option<u64>,
        assumptions: Vec<String>,
    },
    /// Creates a project (unless `project_id` names an existing one) and its tasks.
    DeliveryPlan {
        customer_id: Uuid,
        offer_id: Option<Uuid>,
        project_id: Option<Uuid>,
        project_name: String,
        tasks: Vec<PlannedTask>,
        acceptance_criteria: Vec<String>,
    },
    /// Creates a draft invoice linked to a project.
    InvoiceDraft {
        customer_id: Uuid,
        project_id: Option<Uuid>,
        title: String,
        body: String,
        amount_cents: u64,
        due_in_days: u32,
    },
    /// Records review findings against a document; approving acknowledges
    /// them (no business state changes).
    ReviewFindings {
        document_id: Uuid,
        findings: Vec<ReviewFinding>,
    },
    /// Records a compliance check report; approving acknowledges it.
    ComplianceReport { report_id: Uuid },
}

impl ProposedChange {
    pub fn kind(&self) -> &'static str {
        match self {
            ProposedChange::DiscoverySummary { .. } => "discovery_summary",
            ProposedChange::OfferDraft { .. } => "offer_draft",
            ProposedChange::DeliveryPlan { .. } => "delivery_plan",
            ProposedChange::InvoiceDraft { .. } => "invoice_draft",
            ProposedChange::ReviewFindings { .. } => "review_findings",
            ProposedChange::ComplianceReport { .. } => "compliance_report",
        }
    }
}

/// One proposal by one employee, waiting for (or decided by) the founder.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    pub id: Uuid,
    pub employee_id: Uuid,
    pub role: RoleId,
    pub title: String,
    /// Plain-language summary the founder reads before deciding.
    pub summary: String,
    pub change: ProposedChange,
    /// Which stored facts the employee was given (field paths, never values).
    pub evidence: Vec<String>,
    pub provider_id: String,
    pub provider_trust: String,
    /// True when a real model produced the proposal; false when the
    /// deterministic template did (the model was absent or its output failed
    /// validation). Shown to the founder, never hidden.
    pub model_backed: bool,
    pub status: DecisionStatus,
    pub created_at: i64,
    #[serde(default)]
    pub decided_at: Option<i64>,
    /// What the approval created, e.g. `document:<id>`.
    #[serde(default)]
    pub outcome: Option<String>,
}

/// Which record an employee should look at when it runs.
#[derive(Debug, Clone, Copy, Default, Deserialize)]
pub struct RunSubject {
    #[serde(default)]
    pub customer_id: Option<Uuid>,
    #[serde(default)]
    pub document_id: Option<Uuid>,
    #[serde(default)]
    pub project_id: Option<Uuid>,
}

/// Read model for the Today view: something a hired employee could do now,
/// derived from state alone.
#[derive(Debug, Clone, Serialize)]
pub struct WorkSuggestion {
    pub employee_id: Uuid,
    pub role: RoleId,
    pub subject: RunSubjectRef,
    pub subject_name: String,
    /// Stable machine code the UI localizes.
    pub reason: &'static str,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct RunSubjectRef {
    pub customer_id: Option<Uuid>,
    pub document_id: Option<Uuid>,
    pub project_id: Option<Uuid>,
}

//! Business-graph entities for the consultant loop (Experimental product
//! slice): funnel stage and discovery on customers, projects and tasks,
//! follow-ups, payments, and the read models derived from them. Every
//! mutation of these types goes through the same policy check and signed
//! audit commit as the rest of the workspace; nothing here is authoritative
//! until it is stored in the vault with its event on the chain.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Where a contact sits in the consulting funnel. Promotion to `Customer`
/// is a founder-recorded classification (an accepted offer), never a model
/// inference and never proof of a signed contract or received money.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CustomerStage {
    Lead,
    Customer,
}

impl Default for CustomerStage {
    /// Records written before stages existed were called customers.
    fn default() -> Self {
        CustomerStage::Customer
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectStatus {
    Proposed,
    Active,
    Done,
}

/// An engagement: the delivery side of an accepted offer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: Uuid,
    pub customer_id: Uuid,
    /// The offer this project delivers, when it came from one.
    #[serde(default)]
    pub offer_id: Option<Uuid>,
    pub name: String,
    pub status: ProjectStatus,
    #[serde(default)]
    pub budget_cents: Option<u64>,
    pub created_at: i64,
    pub updated_at: i64,
    #[serde(default)]
    pub done_at: Option<i64>,
}

/// One unit of delivery work. `origin` records who proposed it: `founder`,
/// or `employee:<role>` when an AI employee's approved proposal created it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub project_id: Uuid,
    pub title: String,
    #[serde(default)]
    pub due_at: Option<i64>,
    #[serde(default)]
    pub done_at: Option<i64>,
    pub created_at: i64,
    pub origin: String,
}

/// A dated reminder to contact a customer. Completing it is the founder's
/// own attestation; the system contacts nobody.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FollowUp {
    pub id: Uuid,
    pub customer_id: Uuid,
    pub due_at: i64,
    pub note: String,
    #[serde(default)]
    pub done_at: Option<i64>,
    pub created_at: i64,
    pub origin: String,
}

/// Money the founder records as received against an issued invoice. Receipts
/// are founder attestations — no bank, gateway, or payment provider is
/// involved — and they can never exceed the invoice amount.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payment {
    pub id: Uuid,
    pub invoice_id: Uuid,
    pub amount_cents: u64,
    pub received_at: i64,
    pub note: String,
    pub created_at: i64,
}

/// Read model: what is still owed on one issued invoice.
#[derive(Debug, Clone, Serialize)]
pub struct Receivable {
    pub invoice_id: Uuid,
    pub customer_id: Uuid,
    pub customer_name: String,
    pub title: String,
    pub amount_cents: u64,
    pub paid_cents: u64,
    pub outstanding_cents: u64,
    /// `open`, `partial`, `paid`, or `overdue` (open or partial past `due_at`).
    pub status: String,
    pub due_at: Option<i64>,
}

/// Read model: one signed event on the audit chain that touched a customer
/// or something belonging to them, newest last. Derived from the ledger, so
/// it can only show what was actually signed.
#[derive(Debug, Clone, Serialize)]
pub struct TimelineEntry {
    pub at: i64,
    pub action: String,
    pub resource: String,
    pub subject: String,
}

/// Input for the company profile. Jurisdiction and registration facts feed
/// the compliance checks; they are founder-entered facts, not inferred.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct VentureProfileInput {
    pub name: String,
    pub service: String,
    #[serde(default)]
    pub jurisdiction: String,
    #[serde(default)]
    pub currency: String,
    #[serde(default)]
    pub uen: String,
    #[serde(default)]
    pub gst_registered: bool,
    #[serde(default)]
    pub incorporated_at: Option<i64>,
    #[serde(default)]
    pub fiscal_year_end_month: Option<u8>,
    #[serde(default)]
    pub revenue_estimate_cents: Option<u64>,
}

/// Input for editing a customer. Every field is replaced (not patched) so a
/// save is a complete statement of the record.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct CustomerInput {
    pub name: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub discovery_notes: String,
    #[serde(default)]
    pub stage: Option<CustomerStage>,
    #[serde(default)]
    pub jurisdiction: String,
    #[serde(default)]
    pub personal_data_consent: bool,
}

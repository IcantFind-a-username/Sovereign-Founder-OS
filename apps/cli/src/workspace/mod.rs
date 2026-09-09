//! Minimal Founder Workspace: the first *usable* slice of the product.
//!
//! This is a deliberately small, deterministic prototype of the future
//! Sovereign Enterprise Graph and Approval Center. It already honors the
//! architecture's non-negotiables where they exist today:
//!
//! - authoritative state lives in the local encrypted vault, never in a cloud
//!   or a chat history;
//! - every mutation is evaluated by the deterministic policy engine first and
//!   leaves a signed, hash-chained audit event;
//! - actions the policy classifies as high-risk (sending a document) are not
//!   executed on the model's say-so: they enter a pending-approval queue for
//!   the human owner, and only the owner's signed approval unlocks the effect;
//! - the founder can export every byte of their business state at any time.
//!
//! Honest labels: documents are template-generated and the graph schema is a
//! prototype. A local drafting assistant (deterministic, not an LLM) can
//! suggest outreach text through the resilient model gateway, but its output
//! is untrusted, is never written to authoritative state, and holds no keys —
//! only the disclosure is audited. Approving a send runs the real RFC
//! 0003 chain — owner-signed approval evidence, a Capability V2 token issued
//! from it, a pure-compute preparation step in the verified sandbox, and then
//! the first real host effect: the approved document is written to the local
//! `outbox/` directory through an audited, path-safe broker. That local file
//! write is genuine and revocable; delivering the file to the customer
//! remains the founder's own action, and no network effect exists. Owner keys
//! live in the prototype vault.

mod compliance;
mod compliance_pack;
mod compose;
mod crew_ops;
mod crew_roles;
mod crew_types;
mod erp_ops;
mod erp_types;
mod kernel_exec;
mod model_config;
mod ops;
mod reporting;
mod send_workflow;
mod store;
mod types;
mod util;
mod verify;

#[cfg(test)]
mod compliance_tests;
#[cfg(test)]
mod crew_tests;
#[cfg(test)]
mod erp_tests;
#[cfg(test)]
mod stage1_suite;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;

pub use compliance::ComplianceSubject;
pub use compliance_pack::packs;
pub use crew_roles::role_cards;
pub use crew_types::*;
pub use erp_types::*;
pub use model_config::{provider_status, MODEL_CONFIG_FILE};
pub use types::*;
pub use util::parse_amount_cents;
pub use verify::verify_export;

use std::path::PathBuf;

use sovereign_identity::DeviceIdentity;
use sovereign_policy::PolicyEngine;

pub const WORKSPACE_VAULT_ENTRY: &str = "workspace_graph";
/// Version 2 adds the business graph (stages, projects, tasks, follow-ups,
/// payments). Older vaults load through serde defaults and are stamped with
/// the current version on the next commit; nothing is rewritten on read.
const WORKSPACE_VERSION: u32 = 2;
/// Stable identifier stamped into every export and required on verification.
pub const EXPORT_FORMAT: &str = "sovereign-founder-os-export";
const MAX_TEXT_FIELD_BYTES: usize = 4 * 1024;
const MAX_CUSTOMERS: usize = 500;
const MAX_DOCUMENTS: usize = 2_000;
const MAX_BODY_BYTES: usize = 32 * 1024;
const MAX_PROJECTS: usize = 500;
const MAX_TASKS: usize = 5_000;
const MAX_FOLLOW_UPS: usize = 2_000;
const MAX_PAYMENTS: usize = 5_000;
const MAX_EMPLOYEES: usize = 12;
const MAX_DECISIONS: usize = 5_000;
const MAX_COMPLIANCE_REPORTS: usize = 500;

// The built-in delivery-preparation tool is authored by the application
// itself; its publisher key is a build constant, not a secret. Owner keys
// (approval, authority, admission) are generated per installation and kept
// in the encrypted vault — prototype key management, honestly labelled.
const BUILTIN_PUBLISHER_SECRET: [u8; 32] = *b"sovereign-builtin-publisher-01!!";
const BUILTIN_PUBLISHER_ISSUER: &str = "builtin.sovereign-founder-os";
const RUNTIME_AUTHORITY_ISSUER: &str = "workspace-runtime.local";
const OWNER_APPROVAL_ISSUER: &str = "founder-owner.local";
const OWNER_ADMISSION_ISSUER: &str = "founder-device.workspace";
const WORKSPACE_AUDIENCE: &str = "sovereign-runtime";
const APPROVAL_TTL_SECONDS: i64 = 300;

/// Storage + evidence context for one workspace operation.
pub struct Store {
    root: PathBuf,
    device: DeviceIdentity,
    policy: PolicyEngine,
}

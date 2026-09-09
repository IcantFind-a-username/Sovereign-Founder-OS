//! The data-sovereignty boundary, surfaced in the product.
//!
//! `crates/privacy` decides what may leave this device; this module is the
//! adapter between the workspace's own records and that crate, plus the
//! device policy the founder selects. It performs no dispatch: no public
//! provider exists yet, so a preview here answers "what exactly would leave
//! if one were enabled", which is a question worth being able to answer
//! before the answer matters.

use serde::Serialize;
use sovereign_privacy::{
    compile, CompileError, Placement, PlacementDecision, PolicySnapshot, Preset, Provenance,
    Purpose, SourceRecord, TrustedValue,
};
use uuid::Uuid;

use super::util::{now, storage};
use super::*;

/// Where the founder's chosen preset lives: device policy, stored beside the
/// provider configuration rather than in the business vault, because it
/// describes this machine rather than the company.
pub const PRESET_FILE: &str = "privacy.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StoredPreset {
    AutoProtect,
    LocalOnly,
}

impl From<StoredPreset> for Preset {
    fn from(value: StoredPreset) -> Self {
        match value {
            StoredPreset::AutoProtect => Preset::AutoProtect,
            StoredPreset::LocalOnly => Preset::LocalOnly,
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
struct PrivacyFile {
    preset: StoredPreset,
}

/// One field's fate, in the founder's terms rather than the protocol's.
#[derive(Debug, Clone, Serialize)]
pub struct PreviewField {
    pub field: String,
    pub outcome: String,
    pub note: String,
    pub placeholder: Option<String>,
    pub warning: Option<String>,
}

/// A local, on-screen answer to "what would leave this device". It carries
/// exact values because that is the point; it is returned to the owner's own
/// loopback page and is never persisted or logged.
#[derive(Debug, Clone, Serialize)]
pub struct ExposureView {
    pub purpose: String,
    pub transform_id: String,
    pub fields: Vec<PreviewField>,
    pub outbound_text: String,
    pub outbound_bytes: usize,
    pub outbound_digest: String,
    pub expires_at_unix: i64,
    /// The placement the current preset would choose, and why.
    pub placement: String,
    pub placement_reason: Option<String>,
    /// True while no public provider is configured, which is the case today.
    pub dispatch_available: bool,
}

fn purpose_from(text: &str) -> Result<Purpose, WorkspaceError> {
    match text {
        "draft_discovery_summary" => Ok(Purpose::DraftDiscoverySummary),
        "draft_proposal" => Ok(Purpose::DraftProposal),
        "review_draft" => Ok(Purpose::ReviewDraft),
        other => Err(WorkspaceError::Invalid(format!("unknown purpose {other}"))),
    }
}

impl Store {
    /// The device's selected preset; `AutoProtect` when nothing is stored.
    pub fn privacy_preset(&self) -> StoredPreset {
        let path = self.root.join(PRESET_FILE);
        std::fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<PrivacyFile>(&bytes).ok())
            .map(|file| file.preset)
            .unwrap_or(StoredPreset::AutoProtect)
    }

    /// Change the preset and audit the change: which route a founder's data
    /// may take is exactly the kind of decision that should leave evidence.
    pub fn set_privacy_preset(&self, preset: StoredPreset) -> Result<StoredPreset, WorkspaceError> {
        let previous = self.privacy_preset();
        if previous == preset {
            return Ok(preset);
        }
        let body = serde_json::json!({ "preset": preset });
        std::fs::write(
            self.root.join(PRESET_FILE),
            serde_json::to_vec_pretty(&body).map_err(storage)?,
        )
        .map_err(storage)?;
        self.record(
            "privacy.preset",
            "device:policy",
            serde_json::json!({ "from": previous, "to": preset }),
        )?;
        Ok(preset)
    }

    /// Compile what a public model would receive for this task, and show it.
    pub fn exposure_preview(
        &self,
        purpose: &str,
        customer_id: Option<Uuid>,
        document_id: Option<Uuid>,
    ) -> Result<ExposureView, WorkspaceError> {
        let purpose = purpose_from(purpose)?;
        let workspace = self.load()?;
        let venture = workspace
            .venture
            .as_ref()
            .ok_or_else(|| WorkspaceError::Invalid("set up the company profile first".into()))?;

        // Only fields the workspace actually holds are offered. Offering is
        // not permission: the transform's allowlist still decides, and a
        // field it does not name is omitted whatever we pass here.
        let mut record = SourceRecord::new()
            .with(
                "venture.service",
                TrustedValue::public_content(venture.service.clone()),
            )
            .with(
                "venture.currency",
                TrustedValue::public_content(venture.currency.clone()),
            );
        if let Some(id) = customer_id {
            let customer = workspace.customer(id)?;
            record = record
                .with(
                    "customer.name",
                    TrustedValue::protected(customer.name.clone(), Provenance::OwnerEntered),
                )
                .with(
                    "customer.email",
                    TrustedValue::protected(customer.email.clone(), Provenance::OwnerEntered),
                )
                .with(
                    "customer.discovery_notes",
                    TrustedValue::protected(
                        customer.discovery_notes.clone(),
                        Provenance::OwnerEntered,
                    ),
                )
                .with(
                    "customer.notes",
                    TrustedValue::protected(customer.notes.clone(), Provenance::OwnerEntered),
                );
        }
        if let Some(id) = document_id {
            let document = workspace.document(id)?;
            record = record
                .with(
                    "document.body",
                    TrustedValue::protected(document.body.clone(), Provenance::OwnerEntered),
                )
                .with(
                    "document.kind",
                    TrustedValue::public_content(match document.kind {
                        DocumentKind::Offer => "offer",
                        DocumentKind::Invoice => "invoice",
                    }),
                );
            if let Some(amount) = document.amount_cents {
                record = record.with(
                    "proposal.amount",
                    TrustedValue::public_content(format!("{}.{:02}", amount / 100, amount % 100)),
                );
            }
        }

        let stored = self.privacy_preset();
        let now_unix = now();
        let snapshot = PolicySnapshot::new(stored.into(), now_unix);

        // What the preset would decide. Local compute is available whenever a
        // provider answers, so a preview under Local Only still explains the
        // route rather than pretending the question does not arise.
        let local_available = super::model_config::provider_status(&self.root)
            .map(|providers| providers.iter().any(|p| p.health == "healthy"))
            .unwrap_or(false);
        let (placement, placement_reason) = match snapshot.placement_for(local_available) {
            PlacementDecision::Run(Placement::Local) => ("local", None),
            PlacementDecision::Run(Placement::PublicProjection) => ("public_projection", None),
            PlacementDecision::Run(Placement::OwnedNode) => ("owned_node", None),
            PlacementDecision::Queued { reason } => ("queued", Some(reason.to_owned())),
            PlacementDecision::Unavailable { reason } => ("unavailable", Some(reason.to_owned())),
        };

        // The compiler is asked regardless of placement: the founder is
        // entitled to see what a public route would carry before choosing it.
        // Under Local Only it refuses, which is itself the honest answer.
        let auto = PolicySnapshot::new(Preset::AutoProtect, now_unix);
        let (job, preview) = compile(&record, purpose, auto, now_unix).map_err(map_compile)?;

        Ok(ExposureView {
            purpose: purpose.as_str().to_owned(),
            transform_id: preview.transform_id.clone(),
            fields: preview
                .rows
                .iter()
                .map(|row| PreviewField {
                    field: row.field.clone(),
                    outcome: row.outcome.to_owned(),
                    note: row.note.to_owned(),
                    placeholder: row.placeholder.clone(),
                    warning: row.warning.map(str::to_owned),
                })
                .collect(),
            outbound_text: preview.outbound_text.clone(),
            outbound_bytes: preview.outbound_bytes,
            outbound_digest: job.outbound_digest(),
            expires_at_unix: job.expires_at_unix(),
            placement: placement.to_owned(),
            placement_reason,
            // No public adapter exists. Saying so here keeps the page from
            // implying a button that does not exist.
            dispatch_available: false,
        })
    }
}

fn map_compile(error: CompileError) -> WorkspaceError {
    match error {
        CompileError::MissingField(field) => {
            WorkspaceError::Invalid(format!("{field} is needed for this preview"))
        }
        other => WorkspaceError::Invalid(other.to_string()),
    }
}

/// Metadata for the page: the presets, and the transforms that exist.
pub fn transforms_json() -> serde_json::Value {
    serde_json::json!({
        "transforms": sovereign_privacy::REGISTRY.iter().map(|transform| {
            serde_json::json!({
                "id": transform.id(),
                "purpose": transform.purpose.as_str(),
                "fields": transform.fields.iter().map(|rule| serde_json::json!({
                    "field": rule.field,
                    "note": rule.note,
                })).collect::<Vec<_>>(),
            })
        }).collect::<Vec<_>>(),
    })
}

//! Process-local deterministic stand-in (RFC 0004 first implementation).
//!
//! This is **on-device deterministic processing**, not an LLM, not a sandboxed
//! local model, and not “cloud-assisted” inference. It exists so a fully
//! local workflow can finish without constructing a public-compute job.

use std::fmt;

use crate::types::Purpose;
use crate::value::{Provenance, SourceRecord, TrustedValue};

/// Stable kind token written into value-free evidence. Closed identifier,
/// not a caller string.
pub const STAND_IN_KIND: &str = "on_device_deterministic_demonstration";

/// Beginner compact copy (RFC 0004 beginner preview).
pub const STAND_IN_LABEL: &str =
    "On-device processing. No task data leaves this device for this task.";

/// Honest explanation. Must not claim cloud assistance or real AI.
pub const STAND_IN_EXPLANATION: &str = "A local deterministic function processed the customer data shown for this task. No LLM or external service was contacted, and this is not real AI. This is an on-device demonstration, not cloud-assisted inference.";

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LocalError {
    #[error("required field `{0}` is missing or empty")]
    MissingField(String),
}

/// Result of the on-device stand-in. `Debug` is value-free; the owner-facing
/// view is the only way to read the processed text.
#[derive(Clone)]
pub struct LocalResult {
    purpose: Purpose,
    kind: &'static str,
    label: &'static str,
    explanation: &'static str,
    text: TrustedValue,
    owner_view: String,
}

impl LocalResult {
    pub fn purpose(&self) -> Purpose {
        self.purpose
    }

    pub fn kind(&self) -> &'static str {
        self.kind
    }

    pub fn label(&self) -> &'static str {
        self.label
    }

    pub fn explanation(&self) -> &'static str {
        self.explanation
    }

    pub fn text(&self) -> &TrustedValue {
        &self.text
    }

    /// On-screen view for the authorized owner. Not for logs or evidence.
    pub fn owner_view(&self) -> &str {
        &self.owner_view
    }

    pub(crate) fn from_projection_bytes(purpose: Purpose, outbound: &str) -> Self {
        let owner_view = format!("{STAND_IN_EXPLANATION}\n\n{outbound}");
        Self {
            purpose,
            kind: STAND_IN_KIND,
            label: STAND_IN_LABEL,
            explanation: STAND_IN_EXPLANATION,
            text: TrustedValue::restricted_derived(owner_view.clone()),
            owner_view,
        }
    }
}

impl fmt::Debug for LocalResult {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LocalResult")
            .field("purpose", &self.purpose)
            .field("kind", &self.kind)
            .field("chars", &self.owner_view.chars().count())
            .finish()
    }
}

fn required_field(purpose: Purpose) -> &'static str {
    match purpose {
        Purpose::DraftDiscoverySummary | Purpose::DraftProposal => "customer.discovery_notes",
        Purpose::ReviewDraft => "document.body",
    }
}

/// Run the fixed no-network stand-in over a protected record.
pub fn process(record: &SourceRecord, purpose: Purpose) -> Result<LocalResult, LocalError> {
    let field = required_field(purpose);
    let value = record
        .get(field)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| LocalError::MissingField(field.to_owned()))?;

    let owner_view = format!(
        "{STAND_IN_EXPLANATION}\n\nPurpose: {}\n\n{}",
        purpose.as_str(),
        value.expose()
    );
    Ok(LocalResult {
        purpose,
        kind: STAND_IN_KIND,
        label: STAND_IN_LABEL,
        explanation: STAND_IN_EXPLANATION,
        text: TrustedValue::protected(owner_view.clone(), Provenance::LocallyDerived),
        owner_view,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stand_in_copy_is_on_device_and_not_cloud_assisted() {
        let lower_kind = STAND_IN_KIND.to_lowercase();
        assert!(lower_kind.contains("on_device") || lower_kind.contains("on-device"));
        assert!(STAND_IN_LABEL.to_lowercase().contains("on-device"));
        let explanation = STAND_IN_EXPLANATION.to_lowercase();
        assert!(explanation.contains("not cloud-assisted"));
        assert!(explanation.contains("not real ai"));
        assert!(!STAND_IN_LABEL.to_lowercase().contains("cloud-assisted"));
    }

    #[test]
    fn missing_notes_fail_closed() {
        let record =
            SourceRecord::new().with("venture.service", TrustedValue::public_content("sprints"));
        assert!(matches!(
            process(&record, Purpose::DraftDiscoverySummary),
            Err(LocalError::MissingField(field)) if field == "customer.discovery_notes"
        ));
    }

    #[test]
    fn debug_is_value_free() {
        let record = SourceRecord::new().with(
            "customer.discovery_notes",
            TrustedValue::protected("CANARY-local-debug", Provenance::OwnerEntered),
        );
        let result = process(&record, Purpose::DraftDiscoverySummary).unwrap();
        assert!(result.owner_view().contains("CANARY-local-debug"));
        assert!(!format!("{result:?}").contains("CANARY-local-debug"));
    }
}

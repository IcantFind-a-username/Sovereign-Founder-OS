//! The transform registry: the closed, versioned set of ways a protected
//! record may become public-compute bytes.
//!
//! Two rules give the registry its safety, and both are structural rather
//! than a matter of care:
//!
//! - **Allowlist.** A transform names every field it may read and how each is
//!   dispositioned. A field it does not name is omitted, so adding a field to
//!   the caller's data model can never silently start disclosing it.
//! - **Fixed slots.** A pseudonymised field always renders as the same
//!   fixed-width label, whatever the underlying value is, so two records that
//!   differ only in protected values compile to identical bytes. Neither the
//!   value nor its length is inferable from the payload.
//!
//! There is no general-purpose anonymiser here on purpose: safety is specific
//! to a purpose, a dataset, and a recipient, so it lives in a named transform
//! that a person reviewed.

use serde::Serialize;

use crate::types::Purpose;

/// What happens to one named field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Disposition {
    /// Sent verbatim. Only for values the transform's author established are
    /// structural, never for anything a customer typed.
    Public,
    /// Replaced by a fixed label (`[ORG_1]`), and the value is also removed
    /// from every scrubbed text field in the same record.
    Pseudonym { label: &'static str },
    /// Free text that carries the substance of the task. Known protected
    /// values are replaced by their labels first. What remains is shown to
    /// the founder in the preview, which is why preview is mandatory: a
    /// person is the last check on free text, not a pattern matcher.
    ScrubbedText { max_chars: usize },
    /// Never sent, and its absence is not signalled.
    Omitted { reason: &'static str },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct FieldRule {
    pub field: &'static str,
    pub disposition: Disposition,
    /// Plain-language reason shown in the preview.
    pub note: &'static str,
}

/// A registered, versioned transform. Identity includes the version, so
/// changing behaviour means a new identity and a new review.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Transform {
    pub name: &'static str,
    pub version: u32,
    pub purpose: Purpose,
    /// The instruction sent with the projection. Fixed text, reviewed with
    /// the transform; never assembled from caller input.
    pub instruction: &'static str,
    pub fields: &'static [FieldRule],
    /// Hard ceiling on the compiled payload.
    pub max_bytes: usize,
}

impl Transform {
    pub fn id(&self) -> String {
        format!("{}@{}", self.name, self.version)
    }

    pub fn rule_for(&self, field: &str) -> Option<&FieldRule> {
        self.fields.iter().find(|rule| rule.field == field)
    }
}

const DISCOVERY_SUMMARY: Transform = Transform {
    name: "consulting.discovery-summary",
    version: 1,
    purpose: Purpose::DraftDiscoverySummary,
    instruction: "You are helping an independent consultant read a prospect's discovery notes. From the notes below, list the concrete problems, the constraints, the stated budget, the open questions to ask, and the assumptions you are making. Never present a guess as a confirmed fact. The bracketed labels are placeholders for names withheld from you; reuse them exactly and never invent a real name.",
    fields: &[
        FieldRule {
            field: "customer.name",
            disposition: Disposition::Pseudonym { label: "ORG" },
            note: "the organisation's name is replaced by a placeholder",
        },
        FieldRule {
            field: "customer.contact_name",
            disposition: Disposition::Pseudonym { label: "PERSON" },
            note: "the contact's name is replaced by a placeholder",
        },
        FieldRule {
            field: "customer.email",
            disposition: Disposition::Omitted {
                reason: "an address is never needed to analyse a brief",
            },
            note: "not sent",
        },
        FieldRule {
            field: "customer.discovery_notes",
            disposition: Disposition::ScrubbedText { max_chars: 6_000 },
            note: "your notes, with the names above replaced — read them before sending",
        },
        FieldRule {
            field: "venture.service",
            disposition: Disposition::Public,
            note: "your own service description",
        },
        FieldRule {
            field: "venture.currency",
            disposition: Disposition::Public,
            note: "currency code",
        },
    ],
    max_bytes: 16 * 1024,
};

const PROPOSAL_DRAFT: Transform = Transform {
    name: "consulting.proposal-draft",
    version: 1,
    purpose: Purpose::DraftProposal,
    instruction: "You are drafting a consulting proposal for an independent consultant. Using the brief below, write a proposal with the sections Context, Scope, Deliverables, Timeline, Investment, Assumptions, and Next step. Promise nothing the brief does not support. The bracketed labels are placeholders for names withheld from you; reuse them exactly and never invent a real name.",
    fields: &[
        FieldRule {
            field: "customer.name",
            disposition: Disposition::Pseudonym { label: "ORG" },
            note: "the organisation's name is replaced by a placeholder",
        },
        FieldRule {
            field: "customer.contact_name",
            disposition: Disposition::Pseudonym { label: "PERSON" },
            note: "the contact's name is replaced by a placeholder",
        },
        FieldRule {
            field: "customer.email",
            disposition: Disposition::Omitted {
                reason: "an address is never needed to draft a proposal",
            },
            note: "not sent",
        },
        FieldRule {
            field: "customer.discovery_notes",
            disposition: Disposition::ScrubbedText { max_chars: 6_000 },
            note: "the brief, with the names above replaced — read it before sending",
        },
        FieldRule {
            field: "venture.service",
            disposition: Disposition::Public,
            note: "your own service description",
        },
        FieldRule {
            field: "venture.currency",
            disposition: Disposition::Public,
            note: "currency code",
        },
        FieldRule {
            field: "proposal.amount",
            disposition: Disposition::Public,
            note: "the amount you already decided",
        },
    ],
    max_bytes: 16 * 1024,
};

const DRAFT_REVIEW: Transform = Transform {
    name: "consulting.draft-review",
    version: 1,
    purpose: Purpose::ReviewDraft,
    instruction: "You are reviewing a consultant's draft before it is sent. List gaps, contradictions, leftover placeholders, and risks, quoting the passage for each. Return an empty list only if you found nothing. The bracketed labels are placeholders for names withheld from you.",
    fields: &[
        FieldRule {
            field: "customer.name",
            disposition: Disposition::Pseudonym { label: "ORG" },
            note: "the organisation's name is replaced by a placeholder",
        },
        FieldRule {
            field: "document.body",
            disposition: Disposition::ScrubbedText { max_chars: 12_000 },
            note: "the draft, with the name above replaced — read it before sending",
        },
        FieldRule {
            field: "document.kind",
            disposition: Disposition::Public,
            note: "whether it is an offer or an invoice",
        },
    ],
    max_bytes: 32 * 1024,
};

/// Every transform the product may use. Closed: a caller cannot register one.
pub const REGISTRY: &[Transform] = &[DISCOVERY_SUMMARY, PROPOSAL_DRAFT, DRAFT_REVIEW];

/// The transform registered for a purpose, if any.
pub fn for_purpose(purpose: Purpose) -> Option<&'static Transform> {
    REGISTRY
        .iter()
        .find(|transform| transform.purpose == purpose)
}

pub fn by_id(id: &str) -> Option<&'static Transform> {
    REGISTRY.iter().find(|transform| transform.id() == id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn every_purpose_has_exactly_one_transform_and_ids_are_unique() {
        let ids: BTreeSet<String> = REGISTRY.iter().map(Transform::id).collect();
        assert_eq!(ids.len(), REGISTRY.len(), "transform ids must be unique");
        for purpose in [
            Purpose::DraftDiscoverySummary,
            Purpose::DraftProposal,
            Purpose::ReviewDraft,
        ] {
            let matches = REGISTRY
                .iter()
                .filter(|transform| transform.purpose == purpose)
                .count();
            assert_eq!(matches, 1, "{purpose:?} must map to exactly one transform");
        }
    }

    #[test]
    fn no_transform_sends_an_address_or_names_a_field_twice() {
        for transform in REGISTRY {
            let mut seen = BTreeSet::new();
            for rule in transform.fields {
                assert!(
                    seen.insert(rule.field),
                    "{} repeats {}",
                    transform.id(),
                    rule.field
                );
                if rule.field.contains("email") || rule.field.contains("phone") {
                    assert!(
                        matches!(rule.disposition, Disposition::Omitted { .. }),
                        "{} must not disclose {}",
                        transform.id(),
                        rule.field
                    );
                }
            }
        }
    }

    #[test]
    fn instructions_are_fixed_text_with_no_interpolation_holes() {
        for transform in REGISTRY {
            assert!(!transform.instruction.contains('{'), "{}", transform.id());
            assert!(transform.instruction.len() > 60, "{}", transform.id());
        }
    }
}

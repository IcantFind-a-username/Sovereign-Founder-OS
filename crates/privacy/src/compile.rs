//! The privacy compiler: the only way to build a public-compute job.
//!
//! Nothing else in the product may assemble bytes for a public model. The
//! compiler reads a caller's record through a registered transform, applies
//! each field's disposition, and returns two things: an opaque job carrying
//! the exact outbound bytes, and a local preview that shows a person exactly
//! what those bytes are before anything is dispatched.

use std::collections::BTreeMap;

use serde::Serialize;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::transform::Disposition;
use crate::types::{Grant, PolicySnapshot, Preset, Purpose, Recipient, Sensitivity};
use crate::value::SourceRecord;

/// How long a compiled job stays dispatchable. Short on purpose: the preview
/// a person approved describes this policy snapshot and these values.
pub const JOB_TTL_SECONDS: i64 = 600;
/// Values shorter than this cannot be removed from free text reliably — a
/// two-letter name matches inside ordinary words. The compiler still replaces
/// the field itself, and the preview says the text was not scrubbed for it.
const MIN_SCRUBBABLE_CHARS: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CompileError {
    #[error("no transform is registered for this purpose")]
    NoTransform,
    #[error("the selected preset does not permit public compute")]
    PresetForbidsPublicCompute,
    #[error("required field `{0}` is missing or empty")]
    MissingField(String),
    #[error("field `{0}` is longer than the transform permits")]
    FieldTooLong(String),
    #[error("the compiled payload is {actual} bytes; the transform allows {maximum}")]
    PayloadTooLarge { actual: usize, maximum: usize },
    #[error("field `{0}` holds externally produced content, which cannot be re-sent as a brief")]
    UntrustedInput(String),
}

/// A compiled, dispatchable job. It is deliberately not `Serialize`, and its
/// `Debug` is value-free: the outbound bytes leave only through a dispatch
/// that binds this job, and the placeholder map never leaves the device at
/// all.
#[derive(Clone)]
pub struct PublicJob {
    id: Uuid,
    transform_id: String,
    purpose: Purpose,
    instruction: &'static str,
    payload: String,
    created_at_unix: i64,
    expires_at_unix: i64,
    policy: PolicySnapshot,
    grant: Grant,
    /// label → the local value it stands for. Local only: this is what makes
    /// a response rehydratable, and it is exactly what must never be sent.
    placeholders: BTreeMap<String, String>,
}

impl PublicJob {
    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn transform_id(&self) -> &str {
        &self.transform_id
    }

    pub fn purpose(&self) -> Purpose {
        self.purpose
    }

    pub fn expires_at_unix(&self) -> i64 {
        self.expires_at_unix
    }

    pub fn policy(&self) -> PolicySnapshot {
        self.policy
    }

    pub fn grant(&self) -> &Grant {
        &self.grant
    }

    /// Exactly what would be sent — instruction and payload, nothing else.
    pub fn outbound_text(&self) -> String {
        format!("{}\n\n{}", self.instruction, self.payload)
    }

    pub fn outbound_digest(&self) -> String {
        hex::encode(Sha256::digest(self.outbound_text().as_bytes()))
    }

    /// Still within its window. Revalidated before dispatch, not just at
    /// compile time, so a job cannot be held and replayed later.
    pub fn is_live_at(&self, now_unix: i64) -> bool {
        now_unix >= self.created_at_unix && now_unix < self.expires_at_unix
    }

    pub(crate) fn placeholders(&self) -> &BTreeMap<String, String> {
        &self.placeholders
    }

    /// The value-free record of this job, safe for logs and evidence: what
    /// transform ran, for what purpose, how big the payload was, and its
    /// digest. No field values, no placeholder map, no lengths of protected
    /// inputs.
    pub fn manifest(&self) -> ExposureManifest {
        ExposureManifest {
            job_id: self.id,
            transform_id: self.transform_id.clone(),
            purpose: self.purpose,
            recipient: self.grant.recipient,
            outbound_bytes: self.outbound_text().len(),
            outbound_digest: self.outbound_digest(),
            placeholder_count: self.placeholders.len(),
            policy_epoch: self.policy.revocation_epoch,
            created_at_unix: self.created_at_unix,
            expires_at_unix: self.expires_at_unix,
        }
    }
}

/// Value-free evidence. Everything here is a closed identifier, a count, or a
/// digest of bytes the owner already previewed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExposureManifest {
    pub job_id: Uuid,
    pub transform_id: String,
    pub purpose: Purpose,
    pub recipient: Recipient,
    pub outbound_bytes: usize,
    pub outbound_digest: String,
    pub placeholder_count: usize,
    pub policy_epoch: u64,
    pub created_at_unix: i64,
    pub expires_at_unix: i64,
}

/// What happened to one field, in the founder's terms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewRow {
    pub field: String,
    /// `sent`, `replaced`, `omitted`, or `absent`.
    pub outcome: &'static str,
    pub note: &'static str,
    /// The placeholder a replaced value became, if any.
    pub placeholder: Option<String>,
    /// Set when a value was too short to remove from free text reliably.
    pub warning: Option<&'static str>,
}

/// A local, transient view shown before dispatch. It may show exact values to
/// the authorized person — that is its whole purpose — so it is not
/// `Serialize`, its `Debug` is value-free, and it must never be persisted or
/// logged. The UI reads its fields deliberately; nothing prints it by
/// accident.
#[derive(Clone)]
pub struct ExposurePreview {
    pub transform_id: String,
    pub purpose: Purpose,
    pub rows: Vec<PreviewRow>,
    /// Exactly the bytes that would leave this device.
    pub outbound_text: String,
    pub outbound_bytes: usize,
}

/// Identity and shape only. The payload and the placeholder map are the two
/// things this type exists to protect, so neither is printable.
impl std::fmt::Debug for PublicJob {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PublicJob")
            .field("id", &self.id)
            .field("transform_id", &self.transform_id)
            .field("purpose", &self.purpose)
            .field("outbound_bytes", &self.outbound_text().len())
            .field("outbound_digest", &self.outbound_digest())
            .field("placeholders", &self.placeholders.len())
            .field("expires_at_unix", &self.expires_at_unix)
            .finish()
    }
}

/// Shape only, for the same reason: a preview exists to be read on screen by
/// the owner, never to be dumped into a log line.
impl std::fmt::Debug for ExposurePreview {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ExposurePreview")
            .field("transform_id", &self.transform_id)
            .field("purpose", &self.purpose)
            .field("rows", &self.rows.len())
            .field("outbound_bytes", &self.outbound_bytes)
            .finish()
    }
}

/// Compile a record into a public-compute job plus its preview.
///
/// Fails closed on every unclear case: an unregistered purpose, a preset that
/// forbids public compute, a missing required field, an oversized payload, or
/// externally produced content being passed off as a brief.
pub fn compile(
    record: &SourceRecord,
    purpose: Purpose,
    policy: PolicySnapshot,
    now_unix: i64,
) -> Result<(PublicJob, ExposurePreview), CompileError> {
    if policy.preset == Preset::LocalOnly {
        return Err(CompileError::PresetForbidsPublicCompute);
    }
    let transform = crate::transform::for_purpose(purpose).ok_or(CompileError::NoTransform)?;

    // Pass 1: assign a fixed label to each pseudonymised field that is
    // present. Order follows the transform, so the same shape of record
    // always produces the same labels.
    let mut placeholders: BTreeMap<String, String> = BTreeMap::new();
    let mut labels: BTreeMap<&str, String> = BTreeMap::new();
    let mut counters: BTreeMap<&str, usize> = BTreeMap::new();
    for rule in transform.fields {
        let Disposition::Pseudonym { label } = rule.disposition else {
            continue;
        };
        let Some(value) = record.get(rule.field).filter(|value| !value.is_empty()) else {
            continue;
        };
        let index = counters.entry(label).or_insert(0);
        *index += 1;
        let token = format!("[{label}_{index}]");
        placeholders.insert(token.clone(), value.expose().to_owned());
        labels.insert(rule.field, token);
    }

    // Longest first, so a value containing another is replaced whole.
    let mut scrub: Vec<(&str, &str)> = placeholders
        .iter()
        .map(|(token, value)| (value.as_str(), token.as_str()))
        .filter(|(value, _)| value.chars().count() >= MIN_SCRUBBABLE_CHARS)
        .collect();
    scrub.sort_by_key(|(value, _)| std::cmp::Reverse(value.len()));
    let short_value_present = placeholders
        .values()
        .any(|value| value.chars().count() < MIN_SCRUBBABLE_CHARS);

    // Pass 2: render each field and record what the founder will see.
    let mut rows = Vec::new();
    let mut lines = Vec::new();
    for rule in transform.fields {
        let present = record.get(rule.field).filter(|value| !value.is_empty());
        match (rule.disposition, present) {
            (Disposition::Omitted { .. }, _) => rows.push(PreviewRow {
                field: rule.field.to_owned(),
                outcome: "omitted",
                note: rule.note,
                placeholder: None,
                warning: None,
            }),
            (_, None) => rows.push(PreviewRow {
                field: rule.field.to_owned(),
                outcome: "absent",
                note: rule.note,
                placeholder: None,
                warning: None,
            }),
            (Disposition::Pseudonym { .. }, Some(_)) => {
                let token = labels.get(rule.field).cloned().unwrap_or_default();
                lines.push(format!("{}: {token}", field_label(rule.field)));
                rows.push(PreviewRow {
                    field: rule.field.to_owned(),
                    outcome: "replaced",
                    note: rule.note,
                    placeholder: Some(token),
                    warning: None,
                });
            }
            (Disposition::Public, Some(value)) => {
                if value.sensitivity() == Sensitivity::RestrictedDerived {
                    return Err(CompileError::UntrustedInput(rule.field.to_owned()));
                }
                lines.push(format!("{}: {}", field_label(rule.field), value.expose()));
                rows.push(PreviewRow {
                    field: rule.field.to_owned(),
                    outcome: "sent",
                    note: rule.note,
                    placeholder: None,
                    warning: None,
                });
            }
            (Disposition::ScrubbedText { max_chars }, Some(value)) => {
                if value.len() > max_chars {
                    return Err(CompileError::FieldTooLong(rule.field.to_owned()));
                }
                let mut text = value.expose().to_owned();
                for (needle, token) in &scrub {
                    text = replace_ignoring_case(&text, needle, token);
                }
                lines.push(format!("{}:\n{text}", field_label(rule.field)));
                rows.push(PreviewRow {
                    field: rule.field.to_owned(),
                    outcome: "sent",
                    note: rule.note,
                    placeholder: None,
                    warning: short_value_present.then_some(
                        "a name shorter than three characters could not be removed from this text \
                         reliably — read it before sending",
                    ),
                });
            }
        }
    }

    let payload = lines.join("\n\n");
    let job = PublicJob {
        id: Uuid::new_v4(),
        transform_id: transform.id(),
        purpose,
        instruction: transform.instruction,
        payload,
        created_at_unix: now_unix,
        expires_at_unix: now_unix + JOB_TTL_SECONDS,
        policy,
        grant: Grant {
            recipient: Recipient::PublicComputeProjection,
            purpose,
            operation: "model.complete",
            expires_at_unix: now_unix + JOB_TTL_SECONDS,
        },
        placeholders,
    };

    let outbound_text = job.outbound_text();
    let outbound_bytes = outbound_text.len();
    if outbound_bytes > transform.max_bytes {
        return Err(CompileError::PayloadTooLarge {
            actual: outbound_bytes,
            maximum: transform.max_bytes,
        });
    }
    let preview = ExposurePreview {
        transform_id: job.transform_id.clone(),
        purpose,
        rows,
        outbound_text,
        outbound_bytes,
    };
    Ok((job, preview))
}

/// Require a field before compiling, so a caller gets a clear error rather
/// than a projection that quietly says nothing.
pub fn require(record: &SourceRecord, field: &str) -> Result<(), CompileError> {
    match record.get(field) {
        Some(value) if !value.is_empty() => Ok(()),
        _ => Err(CompileError::MissingField(field.to_owned())),
    }
}

fn field_label(field: &str) -> &str {
    field.rsplit('.').next().unwrap_or(field)
}

/// Case-insensitive replacement over char boundaries. Small and explicit
/// rather than a regex dependency: the haystacks here are bounded by the
/// transform's own limits.
fn replace_ignoring_case(haystack: &str, needle: &str, replacement: &str) -> String {
    if needle.is_empty() {
        return haystack.to_owned();
    }
    let lower_haystack = haystack.to_lowercase();
    let lower_needle = needle.to_lowercase();
    let mut out = String::with_capacity(haystack.len());
    let mut cursor = 0;
    while let Some(found) = lower_haystack[cursor..].find(&lower_needle) {
        let start = cursor + found;
        let end = start + lower_needle.len();
        // Lowercasing can change byte lengths; skip the match if it does not
        // land on a boundary of the original rather than slicing mid-character.
        if !haystack.is_char_boundary(start) || !haystack.is_char_boundary(end) {
            out.push_str(&haystack[cursor..start.min(haystack.len())]);
            cursor = start;
            break;
        }
        out.push_str(&haystack[cursor..start]);
        out.push_str(replacement);
        cursor = end;
    }
    out.push_str(&haystack[cursor.min(haystack.len())..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replacement_is_case_insensitive_and_leaves_the_rest_alone() {
        assert_eq!(
            replace_ignoring_case("Acme and ACME and acme", "acme", "[ORG_1]"),
            "[ORG_1] and [ORG_1] and [ORG_1]"
        );
        assert_eq!(
            replace_ignoring_case("nothing here", "acme", "[X]"),
            "nothing here"
        );
        assert_eq!(replace_ignoring_case("abc", "", "[X]"), "abc");
    }
}

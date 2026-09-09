//! Opaque trusted values and the record a caller hands the compiler.
//!
//! A `TrustedValue` deliberately does not implement `Serialize`, `Display`, or
//! a value-bearing `Debug`. Printing one shows its provenance, never its
//! contents, so a value cannot leak through a log line, an error, a metric, or
//! a `{:?}` in a hurry. The only ways out are inside this crate: the compiler
//! reads it to build a projection, and the local preview shows it to the
//! authorized person before anything is dispatched.

use std::collections::BTreeMap;
use std::fmt;

use crate::types::Sensitivity;

/// Where a value came from, kept for evidence. Closed set; never free text
/// from a caller, so it cannot smuggle a value into a manifest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Provenance {
    /// Entered by the founder in this workspace.
    OwnerEntered,
    /// Derived locally by deterministic code from owner-entered values.
    LocallyDerived,
    /// Produced by a model or another external party.
    ExternallyProduced,
}

/// A value the crate refuses to print.
#[derive(Clone, PartialEq, Eq)]
pub struct TrustedValue {
    value: String,
    sensitivity: Sensitivity,
    provenance: Provenance,
}

impl TrustedValue {
    /// Anything whose sensitivity is not established enters as `Protected`.
    /// That is the safe default the RFC requires for unknown dynamic values.
    pub fn protected(value: impl Into<String>, provenance: Provenance) -> Self {
        Self {
            value: value.into(),
            sensitivity: Sensitivity::Protected,
            provenance,
        }
    }

    /// Fixed or authoritative content with trusted provenance — a currency
    /// code, a template heading. Never a customer-supplied string.
    pub fn public_content(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            sensitivity: Sensitivity::PublicContent,
            provenance: Provenance::LocallyDerived,
        }
    }

    /// Model output and anything else externally produced.
    pub fn restricted_derived(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            sensitivity: Sensitivity::RestrictedDerived,
            provenance: Provenance::ExternallyProduced,
        }
    }

    pub fn sensitivity(&self) -> Sensitivity {
        self.sensitivity
    }

    pub fn provenance(&self) -> Provenance {
        self.provenance
    }

    /// Length in characters. Metadata about the value, never the value; used
    /// for previews and bounds, never written to evidence.
    pub fn len(&self) -> usize {
        self.value.chars().count()
    }

    pub fn is_empty(&self) -> bool {
        self.value.trim().is_empty()
    }

    /// The bytes themselves. Crate-private on purpose: only the compiler and
    /// the local preview may read a protected value, and both are here.
    pub(crate) fn expose(&self) -> &str {
        &self.value
    }
}

/// Provenance and sensitivity only — never the value. This is what a stray
/// `{:?}` prints.
impl fmt::Debug for TrustedValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TrustedValue")
            .field("sensitivity", &self.sensitivity)
            .field("provenance", &self.provenance)
            .field("chars", &self.len())
            .finish()
    }
}

/// The fields a caller offers for one compilation.
///
/// Offering a field is not permission to send it: the transform's allowlist
/// decides. A field the transform does not name is omitted, so adding a field
/// to the caller's own data model can never silently start disclosing it.
#[derive(Debug, Default, Clone)]
pub struct SourceRecord {
    fields: BTreeMap<String, TrustedValue>,
}

impl SourceRecord {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with(mut self, field: impl Into<String>, value: TrustedValue) -> Self {
        self.fields.insert(field.into(), value);
        self
    }

    pub fn get(&self, field: &str) -> Option<&TrustedValue> {
        self.fields.get(field)
    }

    pub fn field_names(&self) -> impl Iterator<Item = &str> {
        self.fields.keys().map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_trusted_value_never_prints_its_contents() {
        let value = TrustedValue::protected("alex.chen@acme.test", Provenance::OwnerEntered);
        let printed = format!("{value:?}");
        assert!(!printed.contains("alex"), "{printed}");
        assert!(!printed.contains("acme"), "{printed}");
        assert!(printed.contains("Protected"));
        assert!(printed.contains("OwnerEntered"));
        assert!(
            printed.contains("19"),
            "the length is metadata, not the value"
        );
    }

    #[test]
    fn unknown_values_enter_as_protected() {
        let value = TrustedValue::protected("anything", Provenance::OwnerEntered);
        assert_eq!(value.sensitivity(), Sensitivity::Protected);
        assert_eq!(
            TrustedValue::public_content("SGD").sensitivity(),
            Sensitivity::PublicContent
        );
        assert_eq!(
            TrustedValue::restricted_derived("model text").sensitivity(),
            Sensitivity::RestrictedDerived
        );
    }

    #[test]
    fn a_record_reports_only_the_fields_it_was_given() {
        let record = SourceRecord::new()
            .with(
                "customer.name",
                TrustedValue::protected("Acme", Provenance::OwnerEntered),
            )
            .with("venture.currency", TrustedValue::public_content("SGD"));
        assert_eq!(
            record.field_names().collect::<Vec<_>>(),
            ["customer.name", "venture.currency"]
        );
        assert!(record.get("customer.email").is_none());
    }
}

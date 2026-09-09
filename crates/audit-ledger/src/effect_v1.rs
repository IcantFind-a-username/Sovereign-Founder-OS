//! What the fixture's evidence chain is allowed to remember about an effect.
//!
//! An audit record exists to answer "did this happen, in what order, and was
//! it tampered with". It does not exist to answer "what was it". Those look
//! similar and are not, and the difference is the whole design here: the
//! evidence is a *projection*, and everything not on a short allow-list is
//! absent rather than redacted.
//!
//! Absent rather than redacted, because redaction leaks. A field replaced by
//! its hash still answers questions: with a small dictionary of candidate
//! recipients — and the set of people a founder emails is small — anyone
//! holding the record can hash each one and learn which it was. Length leaks
//! the same way. So does a timestamp, against someone who knows roughly when
//! something happened.
//!
//! The allow-list is therefore short and its members are chosen because they
//! carry no information about content: an opaque identifier the caller
//! already knows, a closed outcome with a handful of values, a sequence
//! number, the previous record's hash, and a signer tag that says this is a
//! fixture. Nothing derived from what the effect was about appears at all.
//!
//! The coordinator's own state remains the only place that knows what
//! happened. Evidence describes; it never decides.

use sha2::{Digest, Sha256};

/// A closed set. An outcome carried as free text would be a place to put
/// anything, and "anything" eventually includes a subject line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectOutcome {
    Dispatched,
    Refused,
    /// The effect's result is unknown and must stay unknown. Nothing may
    /// relabel it later on the strength of a guess.
    Indeterminate,
}

impl EffectOutcome {
    pub const fn as_str(self) -> &'static str {
        match self {
            EffectOutcome::Dispatched => "dispatched",
            EffectOutcome::Refused => "refused",
            EffectOutcome::Indeterminate => "indeterminate",
        }
    }
}

/// The signer tag. Every fixture record carries it, so no record from this
/// chain can be mistaken for product evidence — including by a reader who
/// found one on its own with no context.
pub const FIXTURE_SIGNER: &str = "synthetic-owner-effect-fixture-signer";

/// One evidence record. Every field is on the allow-list; there is no
/// `extra`, no `detail`, and no `reason`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectEvidenceV1 {
    /// Opaque, and already known to whoever can read this chain. It carries
    /// nothing about the effect's content.
    pub intent_id: uuid::Uuid,
    pub outcome: EffectOutcome,
    pub sequence: u64,
    pub previous_hash: [u8; 32],
    pub signer: &'static str,
}

impl EffectEvidenceV1 {
    pub fn new(
        intent_id: uuid::Uuid,
        outcome: EffectOutcome,
        sequence: u64,
        previous_hash: [u8; 32],
    ) -> Self {
        Self {
            intent_id,
            outcome,
            sequence,
            previous_hash,
            signer: FIXTURE_SIGNER,
        }
    }

    /// This record's hash, which the next record chains from.
    pub fn hash(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(b"sfo-effect-evidence-v1\0");
        hasher.update(self.intent_id.as_bytes());
        hasher.update(self.outcome.as_str().as_bytes());
        hasher.update(b"\0");
        hasher.update(self.sequence.to_le_bytes());
        hasher.update(self.previous_hash);
        hasher.update(self.signer.as_bytes());
        hasher.finalize().into()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceError {
    /// The same intent already has a record with a different outcome.
    OutcomeConflict,
    /// The chain does not verify.
    ChainBroken,
}

/// The fixture's evidence chain, separate from the product's ledger.
#[derive(Debug, Default)]
pub struct EffectEvidenceChain {
    records: Vec<EffectEvidenceV1>,
}

impl EffectEvidenceChain {
    pub fn new() -> Self {
        Self::default()
    }

    /// Append one outcome for one intent.
    ///
    /// Idempotent for the same outcome, because a crash between deciding an
    /// effect's fate and recording it is ordinary and the recovery is to
    /// record it again. Conflicting for a different one, because two answers
    /// to the same question is not something to average — it is a defect, and
    /// the second write is refused rather than allowed to overwrite.
    pub fn append(
        &mut self,
        intent_id: uuid::Uuid,
        outcome: EffectOutcome,
    ) -> Result<&EffectEvidenceV1, EvidenceError> {
        if let Some(index) = self
            .records
            .iter()
            .position(|record| record.intent_id == intent_id)
        {
            if self.records[index].outcome != outcome {
                return Err(EvidenceError::OutcomeConflict);
            }
            return Ok(&self.records[index]);
        }

        let previous_hash = self.records.last().map(|r| r.hash()).unwrap_or([0; 32]);
        let record =
            EffectEvidenceV1::new(intent_id, outcome, self.records.len() as u64, previous_hash);
        self.records.push(record);
        Ok(self.records.last().expect("just pushed"))
    }

    /// Verify the chain end to end.
    pub fn verify(&self) -> Result<(), EvidenceError> {
        let mut expected_previous = [0u8; 32];
        for (index, record) in self.records.iter().enumerate() {
            if record.sequence != index as u64 || record.previous_hash != expected_previous {
                return Err(EvidenceError::ChainBroken);
            }
            if record.signer != FIXTURE_SIGNER {
                return Err(EvidenceError::ChainBroken);
            }
            expected_previous = record.hash();
        }
        Ok(())
    }

    pub fn records(&self) -> &[EffectEvidenceV1] {
        &self.records
    }
}

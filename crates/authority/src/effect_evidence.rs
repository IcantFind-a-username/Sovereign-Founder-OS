//! After a terminal coordinator commit, append value-free synthetic evidence.
//!
//! The coordinator's own state remains the only place that knows what
//! happened. This module records a value-free cursor of terminal intent
//! IDs and closed outcomes, then projects that cursor onto the fixture
//! evidence chain. The two writes are separate commits: a crash between
//! them leaves the effect terminal and the evidence missing. Recovery
//! appends the missing projection and never retries the effect.
//!
//! Fixture-only. Not product RP1-05, not Exact Effect invert.

use crate::broker::exact_fixture::EffectIntentId;
use crate::broker::store::{OwnedStore, StoreError};
use redb::{ReadableTable, TableDefinition};
use sovereign_audit_ledger::effect_v1::{
    EffectEvidenceChain, EffectOutcome, EvidenceError as ChainError,
};

/// Coordinator truth: intent id → closed outcome. Written first.
const TERMINALS: TableDefinition<&[u8], &str> = TableDefinition::new("terminal-outcomes-v1");
/// Append order for the cursor, so heal replays in commit order.
const TERMINAL_ORDER: TableDefinition<u64, &[u8]> = TableDefinition::new("terminal-order-v1");
/// Evidence projection: intent id → closed outcome. Written second.
const EVIDENCE: TableDefinition<&[u8], &str> = TableDefinition::new("effect-evidence-v1");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceError {
    /// The same intent already has a different terminal or evidence outcome.
    OutcomeConflict,
    /// Terminal is durable; appending the projection failed. The effect
    /// must not be retried.
    TerminalEvidencePending,
    /// The store could not answer. Never treated as permission.
    Unavailable,
}

impl From<StoreError> for EvidenceError {
    fn from(_: StoreError) -> Self {
        EvidenceError::Unavailable
    }
}

impl From<ChainError> for EvidenceError {
    fn from(error: ChainError) -> Self {
        match error {
            ChainError::OutcomeConflict => EvidenceError::OutcomeConflict,
            ChainError::ChainBroken => EvidenceError::Unavailable,
        }
    }
}

/// Record a terminal coordinator outcome. Idempotent for the same
/// outcome; conflicting for a different one. Does not append evidence
/// and does not dispatch.
pub fn commit_terminal(
    store: &OwnedStore<'_>,
    intent_id: EffectIntentId,
    outcome: EffectOutcome,
) -> Result<(), EvidenceError> {
    let key = intent_id.as_uuid();
    let key_bytes = key.as_bytes();
    let wanted = outcome.as_str();
    store.write(|transaction| {
        let terminals = transaction
            .open_table(TERMINALS)
            .map_err(|_| EvidenceError::Unavailable)?;
        if let Some(existing) = terminals
            .get(key_bytes.as_slice())
            .map_err(|_| EvidenceError::Unavailable)?
        {
            return if existing.value() == wanted {
                Ok(())
            } else {
                Err(EvidenceError::OutcomeConflict)
            };
        }
        drop(terminals);

        let mut order = transaction
            .open_table(TERMINAL_ORDER)
            .map_err(|_| EvidenceError::Unavailable)?;
        let next = next_sequence(&order)?;
        order
            .insert(next, key_bytes.as_slice())
            .map_err(|_| EvidenceError::Unavailable)?;
        drop(order);

        transaction
            .open_table(TERMINALS)
            .map_err(|_| EvidenceError::Unavailable)?
            .insert(key_bytes.as_slice(), wanted)
            .map_err(|_| EvidenceError::Unavailable)?;
        Ok(())
    })
}

/// Append evidence for every committed terminal that has none.
/// Never dispatches, never mutates a terminal, never relabels.
pub fn heal_evidence(store: &OwnedStore<'_>) -> Result<HealReport, EvidenceError> {
    append_missing(store)
}

/// Commit the terminal, then append missing evidence. A crash between
/// those two commits is the heal window. An append failure after a
/// successful terminal commit is `TerminalEvidencePending`.
pub fn record_terminal_and_append(
    store: &OwnedStore<'_>,
    intent_id: EffectIntentId,
    outcome: EffectOutcome,
) -> Result<HealReport, EvidenceError> {
    commit_terminal(store, intent_id, outcome)?;
    #[cfg(feature = "fault-injection")]
    crate::fault_injection::reach(
        crate::fault_injection::Barrier::AfterTerminalCommitBeforeEvidenceAppend,
    );
    append_missing(store).map_err(|error| match error {
        EvidenceError::Unavailable | EvidenceError::TerminalEvidencePending => {
            EvidenceError::TerminalEvidencePending
        }
        other => other,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HealReport {
    pub appended: usize,
}

/// Rebuild the in-memory projection from durable evidence only.
pub fn load_chain(store: &OwnedStore<'_>) -> Result<EffectEvidenceChain, EvidenceError> {
    let mut chain = EffectEvidenceChain::new();
    for (intent_id, outcome) in listed_evidence(store)? {
        chain.append(intent_id.as_uuid(), outcome)?;
    }
    Ok(chain)
}

/// Value-free cursor: intent and closed outcome, in commit order.
pub fn listed_terminals(
    store: &OwnedStore<'_>,
) -> Result<Vec<(EffectIntentId, EffectOutcome)>, EvidenceError> {
    list_ordered(store, TERMINALS)
}

fn listed_evidence(
    store: &OwnedStore<'_>,
) -> Result<Vec<(EffectIntentId, EffectOutcome)>, EvidenceError> {
    list_ordered(store, EVIDENCE)
}

fn append_missing(store: &OwnedStore<'_>) -> Result<HealReport, EvidenceError> {
    let pending = {
        let terminals = listed_terminals(store)?;
        let evidence = listed_evidence(store)?;
        terminals
            .into_iter()
            .filter(|(intent, _)| !evidence.iter().any(|(recorded, _)| recorded == intent))
            .collect::<Vec<_>>()
    };

    if pending.is_empty() {
        return Ok(HealReport { appended: 0 });
    }

    store.write(|transaction| {
        let mut table = transaction
            .open_table(EVIDENCE)
            .map_err(|_| EvidenceError::Unavailable)?;
        for (intent_id, outcome) in &pending {
            let key = intent_id.as_uuid();
            table
                .insert(key.as_bytes().as_slice(), outcome.as_str())
                .map_err(|_| EvidenceError::Unavailable)?;
        }
        Ok(HealReport {
            appended: pending.len(),
        })
    })
}

fn list_ordered(
    store: &OwnedStore<'_>,
    values: TableDefinition<&[u8], &str>,
) -> Result<Vec<(EffectIntentId, EffectOutcome)>, EvidenceError> {
    store.read(|transaction| {
        let order = match transaction.open_table(TERMINAL_ORDER) {
            Ok(order) => order,
            Err(_) => return Ok(Vec::new()),
        };
        let table = match transaction.open_table(values) {
            Ok(table) => table,
            Err(_) => return Ok(Vec::new()),
        };
        let mut listed = Vec::new();
        let mut seq = 0u64;
        loop {
            let Some(key) = order
                .get(&seq)
                .map_err(|_| EvidenceError::Unavailable)?
                .map(|value| value.value().to_vec())
            else {
                break;
            };
            if let Some(outcome) = table
                .get(key.as_slice())
                .map_err(|_| EvidenceError::Unavailable)?
            {
                let intent =
                    uuid::Uuid::from_slice(&key).map_err(|_| EvidenceError::Unavailable)?;
                listed.push((
                    EffectIntentId::from_uuid(intent),
                    parse_outcome(outcome.value())?,
                ));
            }
            seq += 1;
        }
        Ok(listed)
    })
}

fn next_sequence(order: &redb::Table<'_, u64, &[u8]>) -> Result<u64, EvidenceError> {
    let mut seq = 0u64;
    while order
        .get(&seq)
        .map_err(|_| EvidenceError::Unavailable)?
        .is_some()
    {
        seq += 1;
    }
    Ok(seq)
}

fn parse_outcome(value: &str) -> Result<EffectOutcome, EvidenceError> {
    match value {
        "dispatched" => Ok(EffectOutcome::Dispatched),
        "refused" => Ok(EffectOutcome::Refused),
        "indeterminate" => Ok(EffectOutcome::Indeterminate),
        _ => Err(EvidenceError::Unavailable),
    }
}

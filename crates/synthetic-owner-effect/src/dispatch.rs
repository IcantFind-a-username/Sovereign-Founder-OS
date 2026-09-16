//! Consume a reserved handle, publish once, reconcile without writing.
//!
//! Design Accept ≠ product Current. Not 1C0, Exact Effect, ActiveV2, or RP1.

use std::path::Path;

use sovereign_authority::broker::store::OwnedStore;
use uuid::Uuid;

use crate::evidence::append_evidence;
use crate::outcome::{hit_publish, ClosedOutcome, PublishError, PublishFailpoint};
use crate::publish::{
    expected_bytes, observe_publication, publish_exact, writer_io_observed, PublicationObservation,
};
use crate::reserve::{
    load_intent_record, store_intent_record, IntentRecord, IntentState, ReserveError,
};
use crate::reserved::AuthorityReservedEffect;
use crate::sealed::EffectIntentId;
use crate::wasm_step::run_fixed_core_wasm;

/// Live facts rechecked immediately before committing `Dispatching`.
pub struct DispatchLiveContext {
    pub now_unix: i64,
    pub session_id: Uuid,
    pub logout_epoch: u64,
    pub signer_epoch: [u8; 16],
    pub fixture_generation: u64,
}

/// Sole writer entry: consumes [`AuthorityReservedEffect`] by value.
pub fn publish_once(
    store: &OwnedStore<'_>,
    fixture_root: &Path,
    reserved: AuthorityReservedEffect,
    live: &DispatchLiveContext,
) -> Result<ClosedOutcome, PublishError> {
    let intent_id = reserved.intent_id();
    let _consumed: AuthorityReservedEffect = reserved;
    dispatch_reserved(store, fixture_root, intent_id, live)
}

fn dispatch_reserved(
    store: &OwnedStore<'_>,
    fixture_root: &Path,
    intent_id: EffectIntentId,
    live: &DispatchLiveContext,
) -> Result<ClosedOutcome, PublishError> {
    let record = load_intent_record(store, intent_id).map_err(map_reserve)?;
    if record.state.is_terminal() {
        return Err(PublishError::AlreadyTerminal);
    }
    if record.state == IntentState::Dispatching {
        return reconcile_without_writing(store, fixture_root, intent_id, live.signer_epoch);
    }
    if record.state != IntentState::AuthorityReserved {
        return Err(PublishError::NotReserved);
    }
    if writer_io_observed(fixture_root, intent_id) {
        return Err(PublishError::WriterIoObserved);
    }
    if let Err(error) = recheck_live(&record, live) {
        return fail_closed(store, fixture_root, intent_id, record, error);
    }
    if let Err(error) = run_fixed_core_wasm(intent_id, record.fixture_generation) {
        return fail_closed(store, fixture_root, intent_id, record, error);
    }
    hit_publish(PublishFailpoint::BeforeDispatchingCommit)?;
    let mut dispatching = record;
    dispatching.state = IntentState::Dispatching;
    store_intent_record(store, intent_id, &dispatching).map_err(map_reserve)?;
    hit_publish(PublishFailpoint::AfterDispatchingCommit)?;

    let bytes = expected_bytes(intent_id);
    match publish_exact(fixture_root, intent_id, &bytes) {
        Ok(()) => close(store, intent_id, dispatching, ClosedOutcome::Succeeded),
        Err(PublishError::AlreadyPublished) => {
            reconcile_without_writing(store, fixture_root, intent_id, live.signer_epoch)
        }
        Err(_) => close(store, intent_id, dispatching, ClosedOutcome::Indeterminate),
    }
}

/// Restart / crash recovery. Never writes `.eml`, never re-signs.
pub fn reconcile_without_writing(
    store: &OwnedStore<'_>,
    fixture_root: &Path,
    intent_id: EffectIntentId,
    live_signer_epoch: [u8; 16],
) -> Result<ClosedOutcome, PublishError> {
    let record = load_intent_record(store, intent_id).map_err(map_reserve)?;
    match record.state {
        IntentState::Succeeded => Ok(ClosedOutcome::Succeeded),
        IntentState::FailedBeforeDispatch => Ok(ClosedOutcome::FailedBeforeDispatch),
        IntentState::Indeterminate => Ok(ClosedOutcome::Indeterminate),
        IntentState::Prepared | IntentState::AuthorityReserved => {
            if record.signer_epoch != hex::encode(live_signer_epoch) {
                if writer_io_observed(fixture_root, intent_id) {
                    return Err(PublishError::WriterIoObserved);
                }
                close(
                    store,
                    intent_id,
                    record,
                    ClosedOutcome::FailedBeforeDispatch,
                )
            } else {
                Err(PublishError::NotReserved)
            }
        }
        IntentState::Dispatching => {
            let expected = expected_bytes(intent_id);
            let outcome = match observe_publication(fixture_root, intent_id, &expected) {
                PublicationObservation::Identical => ClosedOutcome::Succeeded,
                PublicationObservation::Absent
                | PublicationObservation::Different
                | PublicationObservation::WrongType
                | PublicationObservation::Unreadable
                | PublicationObservation::UncertainDurability => ClosedOutcome::Indeterminate,
            };
            close(store, intent_id, record, outcome)
        }
    }
}

fn recheck_live(record: &IntentRecord, live: &DispatchLiveContext) -> Result<(), PublishError> {
    let _ = live.now_unix;
    if record.session_id != live.session_id.to_string() {
        return Err(PublishError::SessionMismatch);
    }
    if record.logout_epoch != live.logout_epoch {
        return Err(PublishError::LogoutMismatch);
    }
    if record.signer_epoch != hex::encode(live.signer_epoch) {
        return Err(PublishError::EpochMismatch);
    }
    if record.fixture_generation != live.fixture_generation {
        return Err(PublishError::GenerationMismatch);
    }
    Ok(())
}

fn fail_closed(
    store: &OwnedStore<'_>,
    fixture_root: &Path,
    intent_id: EffectIntentId,
    record: IntentRecord,
    error: PublishError,
) -> Result<ClosedOutcome, PublishError> {
    if writer_io_observed(fixture_root, intent_id) {
        return Err(PublishError::WriterIoObserved);
    }
    close(
        store,
        intent_id,
        record,
        ClosedOutcome::FailedBeforeDispatch,
    )?;
    Err(error)
}

fn close(
    store: &OwnedStore<'_>,
    intent_id: EffectIntentId,
    mut record: IntentRecord,
    outcome: ClosedOutcome,
) -> Result<ClosedOutcome, PublishError> {
    record.state = match outcome {
        ClosedOutcome::Succeeded => IntentState::Succeeded,
        ClosedOutcome::FailedBeforeDispatch => IntentState::FailedBeforeDispatch,
        ClosedOutcome::Indeterminate => IntentState::Indeterminate,
    };
    store_intent_record(store, intent_id, &record).map_err(map_reserve)?;
    let _ = append_evidence(store, intent_id, outcome);
    Ok(outcome)
}

fn map_reserve(error: ReserveError) -> PublishError {
    match error {
        ReserveError::UnknownIntent => PublishError::UnknownIntent,
        _ => PublishError::Unavailable,
    }
}

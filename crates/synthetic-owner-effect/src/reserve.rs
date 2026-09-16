//! Sole coordinator write-transaction for v01-D05.
//!
//! Consumes opaque `VerifiedCapabilityV2` and `VerifiedApprovalV1` by value.
//! Rechecks verified bindings against durable state, then commits every
//! reservation fact in one redb write or none of them:
//!
//! 1. approval claim, retained through the already-Current signed approval expiry
//! 2. capability/token claim
//! 3. idempotency binding to the random `effect_intent_id`
//! 4. synthetic authority-node use/decrement
//! 5. intent transition `Prepared -> AuthorityReserved`
//!
//! Never calls the legacy consuming validator and never duplicates COSE,
//! canonicalization, trust, policy, approval, or context checks.
//!
//! **Maturity:** Developer Preview fixture. Design Accept ≠ product Current.
//! Not 1C0, Exact Effect, ActiveV2, or RP1.

use std::cell::Cell;
use std::io::Write;
use std::time::Duration;

use redb::{ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize};
use sovereign_artifact::{
    RiskClass, CLOSED_FIXTURE_OPERATION_ID, CLOSED_FIXTURE_TOOL_ID, CLOSED_FIXTURE_TOOL_VERSION,
};
use sovereign_authority::broker::store::{OwnedStore, StoreError};
use sovereign_capability::v2::{VerifiedApprovalV1, VerifiedCapabilityV2};
use uuid::Uuid;

use crate::reserved::AuthorityReservedEffect;
use crate::sealed::EffectIntentId;

const APPROVALS: TableDefinition<&[u8], &[u8]> =
    TableDefinition::new("fixture-reserved-approvals-v1");
const TOKENS: TableDefinition<&[u8], &[u8]> = TableDefinition::new("fixture-reserved-tokens-v1");
const IDEMPOTENCY: TableDefinition<&[u8], &[u8]> =
    TableDefinition::new("fixture-reserved-idempotency-v1");
const AUTHORITY_NODE: TableDefinition<&[u8], u64> =
    TableDefinition::new("fixture-synthetic-authority-node-v1");
pub(crate) const INTENTS: TableDefinition<&[u8], &[u8]> =
    TableDefinition::new("fixture-effect-intents-v1");
const REVOKED_TOKENS: TableDefinition<&[u8], u8> =
    TableDefinition::new("fixture-revoked-tokens-v1");
const REVOKED_APPROVALS: TableDefinition<&[u8], u8> =
    TableDefinition::new("fixture-revoked-approvals-v1");

/// Starting use budget for one synthetic authority node. Fixture-only.
pub const SYNTHETIC_NODE_INITIAL_USES: u64 = 8;

/// Environment variable naming a kill barrier for real-process crash tests.
pub const KILL_BARRIER_ENV: &str = "SOVEREIGN_FIXTURE_RESERVE_BARRIER";

/// Printed when a kill barrier is reached, so the parent does not guess.
pub const KILL_REACHED_PREFIX: &str = "fixture-reserve-barrier-reached: ";

pub const BARRIER_BEFORE_COMMIT: &str = "BeforeCommit";
pub const BARRIER_AFTER_COMMIT: &str = "AfterCommit";

thread_local! {
    // Per-thread so a failpoint test cannot abort a parallel reservation
    // on another thread. Process-wide injection is what CI's default
    // `--test-threads` caught as `Failpoint(AfterApprovalClaim)` on reopen.
    static INJECT: Cell<u8> = const { Cell::new(0) };
}

/// In-process failpoint between logical mutations. Returning `Err` aborts
/// the write transaction, so nothing partial is visible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ReservationFailpoint {
    AfterApprovalClaim = 1,
    AfterTokenClaim = 2,
    AfterIdempotencyBind = 3,
    AfterAuthorityDecrement = 4,
    AfterIntentTransition = 5,
}

impl ReservationFailpoint {
    pub const ALL_MUTATIONS: &'static [Self] = &[
        Self::AfterApprovalClaim,
        Self::AfterTokenClaim,
        Self::AfterIdempotencyBind,
        Self::AfterAuthorityDecrement,
        Self::AfterIntentTransition,
    ];
}

/// Run `body` with one in-process failpoint armed on **this thread**.
/// The injection is cleared even if `body` panics. Other threads are
/// unaffected, so parallel cargo tests and same-process reservation races
/// cannot observe a sibling failpoint.
pub fn with_failpoint<R>(stage: ReservationFailpoint, body: impl FnOnce() -> R) -> R {
    INJECT.with(|cell| cell.set(stage as u8));
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            INJECT.with(|cell| cell.set(0));
        }
    }
    let _reset = Reset;
    body()
}

fn hit(stage: ReservationFailpoint) -> Result<(), ReserveError> {
    if INJECT.with(Cell::get) == stage as u8 {
        return Err(ReserveError::Failpoint(stage));
    }
    Ok(())
}

pub(crate) fn kill_barrier(name: &'static str) {
    let Ok(requested) = std::env::var(KILL_BARRIER_ENV) else {
        return;
    };
    if requested != name {
        return;
    }
    let mut stdout = std::io::stdout();
    let _ = writeln!(stdout, "{KILL_REACHED_PREFIX}{name}");
    let _ = stdout.flush();
    std::thread::sleep(Duration::from_secs(30));
}

/// Live bindings rechecked inside the reservation transaction.
///
/// This is not a reservation request of raw claim UUIDs. The coordinator
/// still consumes opaque proofs by value; these fields are the current
/// durable/session facts those proofs must match.
pub struct ReservationContext {
    pub now_unix: i64,
    pub session_id: Uuid,
    pub logout_epoch: u64,
    pub signer_epoch: [u8; 16],
    pub fixture_generation: u64,
    pub intent_id: EffectIntentId,
}

/// Durable `Prepared` snapshot written before reservation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparedSnapshot {
    pub intent_id: EffectIntentId,
    pub session_id: Uuid,
    pub logout_epoch: u64,
    pub signer_epoch: [u8; 16],
    pub fixture_generation: u64,
    pub policy_decision_id: Uuid,
}

/// Observable intent state, including D06 terminal outcomes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntentState {
    Prepared,
    AuthorityReserved,
    Dispatching,
    Succeeded,
    FailedBeforeDispatch,
    Indeterminate,
}

impl IntentState {
    pub const fn is_pre_dispatch(self) -> bool {
        matches!(self, Self::Prepared | Self::AuthorityReserved)
    }

    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Succeeded | Self::FailedBeforeDispatch | Self::Indeterminate
        )
    }
}

/// Value-free inspection of whether a reservation committed. Cannot
/// reconstruct [`AuthorityReservedEffect`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReservationView {
    pub intent_state: Option<IntentState>,
    pub approval_claimed: bool,
    pub approval_expires_at_unix: Option<i64>,
    pub token_claimed: bool,
    pub idempotency_bound_to: Option<Uuid>,
    pub authority_uses_remaining: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReserveError {
    Unavailable,
    UnknownIntent,
    IntentNotPrepared,
    EffectAlreadyReserved,
    ApprovalAlreadySpent,
    TokenAlreadyConsumed,
    IdempotencyReplay,
    IdempotencyConflict,
    AuthorityExhausted,
    Expired,
    Revoked,
    SessionMismatch,
    EpochMismatch,
    LogoutMismatch,
    GenerationMismatch,
    ProfileMismatch,
    BindingMismatch,
    Failpoint(ReservationFailpoint),
}

impl ReserveError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Unavailable => "E-RESERVE-UNAVAILABLE",
            Self::UnknownIntent => "E-UNKNOWN-INTENT",
            Self::IntentNotPrepared => "E-INTENT-NOT-PREPARED",
            Self::EffectAlreadyReserved => "E-EFFECT-ALREADY-RESERVED",
            Self::ApprovalAlreadySpent => "E-APPROVAL-ALREADY-SPENT",
            Self::TokenAlreadyConsumed => "E-TOKEN-ALREADY-CONSUMED",
            Self::IdempotencyReplay => "E-IDEMPOTENCY-REPLAY",
            Self::IdempotencyConflict => "E-IDEMPOTENCY-CONFLICT",
            Self::AuthorityExhausted => "E-AUTHORITY-EXHAUSTED",
            Self::Expired => "E-EXPIRED",
            Self::Revoked => "E-REVOKED",
            Self::SessionMismatch => "E-SESSION-MISMATCH",
            Self::EpochMismatch => "E-EPOCH-MISMATCH",
            Self::LogoutMismatch => "E-LOGOUT-MISMATCH",
            Self::GenerationMismatch => "E-GENERATION-MISMATCH",
            Self::ProfileMismatch => "E-PROFILE-MISMATCH",
            Self::BindingMismatch => "E-BINDING-MISMATCH",
            Self::Failpoint(_) => "E-RESERVE-FAILPOINT",
        }
    }
}

impl From<StoreError> for ReserveError {
    fn from(_: StoreError) -> Self {
        Self::Unavailable
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct IntentRecord {
    pub(crate) state: IntentState,
    pub(crate) session_id: String,
    pub(crate) logout_epoch: u64,
    pub(crate) signer_epoch: String,
    pub(crate) fixture_generation: u64,
    pub(crate) policy_decision_id: String,
}

fn require_prepared(state: IntentState) -> Result<(), ReserveError> {
    match state {
        IntentState::Prepared => Ok(()),
        IntentState::AuthorityReserved
        | IntentState::Dispatching
        | IntentState::Succeeded
        | IntentState::FailedBeforeDispatch
        | IntentState::Indeterminate => Err(ReserveError::EffectAlreadyReserved),
    }
}

#[derive(Serialize, Deserialize)]
struct ExpiryClaim {
    expires_at_unix: i64,
}

/// Persist `Prepared` and seed the synthetic authority node if absent.
pub fn persist_prepared(
    store: &OwnedStore<'_>,
    snapshot: &PreparedSnapshot,
) -> Result<(), ReserveError> {
    let record = IntentRecord {
        state: IntentState::Prepared,
        session_id: snapshot.session_id.to_string(),
        logout_epoch: snapshot.logout_epoch,
        signer_epoch: hex::encode(snapshot.signer_epoch),
        fixture_generation: snapshot.fixture_generation,
        policy_decision_id: snapshot.policy_decision_id.to_string(),
    };
    let bytes = serde_json::to_vec(&record).map_err(|_| ReserveError::Unavailable)?;
    let intent_key = snapshot.intent_id.as_uuid();
    let generation_key = snapshot.fixture_generation.to_be_bytes();
    store.write(|transaction| {
        {
            let mut intents = transaction
                .open_table(INTENTS)
                .map_err(|_| ReserveError::Unavailable)?;
            if let Some(existing) = intents
                .get(intent_key.as_bytes().as_slice())
                .map_err(|_| ReserveError::Unavailable)?
            {
                let current: IntentRecord = serde_json::from_slice(existing.value())
                    .map_err(|_| ReserveError::Unavailable)?;
                require_prepared(current.state)?;
                return Ok(());
            }
            intents
                .insert(intent_key.as_bytes().as_slice(), bytes.as_slice())
                .map_err(|_| ReserveError::Unavailable)?;
        }
        {
            let mut nodes = transaction
                .open_table(AUTHORITY_NODE)
                .map_err(|_| ReserveError::Unavailable)?;
            if nodes
                .get(generation_key.as_slice())
                .map_err(|_| ReserveError::Unavailable)?
                .is_none()
            {
                nodes
                    .insert(generation_key.as_slice(), SYNTHETIC_NODE_INITIAL_USES)
                    .map_err(|_| ReserveError::Unavailable)?;
            }
        }
        Ok(())
    })
}

/// Mark a token id revoked. Reservation rechecks this durable fact.
pub fn revoke_token(store: &OwnedStore<'_>, token_id: Uuid) -> Result<(), ReserveError> {
    store.write(|transaction| {
        transaction
            .open_table(REVOKED_TOKENS)
            .map_err(|_| ReserveError::Unavailable)?
            .insert(token_id.as_bytes().as_slice(), 1)
            .map_err(|_| ReserveError::Unavailable)?;
        Ok(())
    })
}

/// Mark an approval id revoked. Reservation rechecks this durable fact.
pub fn revoke_approval(store: &OwnedStore<'_>, approval_id: Uuid) -> Result<(), ReserveError> {
    store.write(|transaction| {
        transaction
            .open_table(REVOKED_APPROVALS)
            .map_err(|_| ReserveError::Unavailable)?
            .insert(approval_id.as_bytes().as_slice(), 1)
            .map_err(|_| ReserveError::Unavailable)?;
        Ok(())
    })
}

/// Consume verified proofs by value and atomically reserve every fact.
pub fn reserve_exact_authority(
    store: &OwnedStore<'_>,
    context: &ReservationContext,
    capability: VerifiedCapabilityV2,
    approval: VerifiedApprovalV1,
) -> Result<AuthorityReservedEffect, ReserveError> {
    let intent_id = context.intent_id;
    let committed = store.write(|transaction| {
        let idempotency_key = capability.idempotency_key();
        let intent_uuid = intent_id.as_uuid();
        {
            let table = transaction
                .open_table(IDEMPOTENCY)
                .map_err(|_| ReserveError::Unavailable)?;
            let bound = table
                .get(idempotency_key.as_bytes().as_slice())
                .map_err(|_| ReserveError::Unavailable)?
                .map(|value| value.value().to_vec());
            if let Some(bound) = bound {
                let bound = uuid_from_bytes(&bound)?;
                return Err(if bound == intent_uuid {
                    ReserveError::IdempotencyReplay
                } else {
                    ReserveError::IdempotencyConflict
                });
            }
        }

        recheck_bindings(transaction, context, &capability, &approval)?;

        let approval_claim = serde_json::to_vec(&ExpiryClaim {
            expires_at_unix: approval.expires_at_unix(),
        })
        .map_err(|_| ReserveError::Unavailable)?;
        {
            let mut table = transaction
                .open_table(APPROVALS)
                .map_err(|_| ReserveError::Unavailable)?;
            if table
                .get(approval.approval_id().as_bytes().as_slice())
                .map_err(|_| ReserveError::Unavailable)?
                .is_some()
            {
                return Err(ReserveError::ApprovalAlreadySpent);
            }
            table
                .insert(
                    approval.approval_id().as_bytes().as_slice(),
                    approval_claim.as_slice(),
                )
                .map_err(|_| ReserveError::Unavailable)?;
        }
        hit(ReservationFailpoint::AfterApprovalClaim)?;

        let token_claim = serde_json::to_vec(&ExpiryClaim {
            expires_at_unix: capability.expires_at_unix(),
        })
        .map_err(|_| ReserveError::Unavailable)?;
        {
            let mut table = transaction
                .open_table(TOKENS)
                .map_err(|_| ReserveError::Unavailable)?;
            if table
                .get(capability.token_id().as_bytes().as_slice())
                .map_err(|_| ReserveError::Unavailable)?
                .is_some()
            {
                return Err(ReserveError::TokenAlreadyConsumed);
            }
            table
                .insert(
                    capability.token_id().as_bytes().as_slice(),
                    token_claim.as_slice(),
                )
                .map_err(|_| ReserveError::Unavailable)?;
        }
        hit(ReservationFailpoint::AfterTokenClaim)?;

        {
            let mut table = transaction
                .open_table(IDEMPOTENCY)
                .map_err(|_| ReserveError::Unavailable)?;
            table
                .insert(
                    idempotency_key.as_bytes().as_slice(),
                    intent_uuid.as_bytes().as_slice(),
                )
                .map_err(|_| ReserveError::Unavailable)?;
        }
        hit(ReservationFailpoint::AfterIdempotencyBind)?;

        {
            let generation_key = context.fixture_generation.to_be_bytes();
            let mut table = transaction
                .open_table(AUTHORITY_NODE)
                .map_err(|_| ReserveError::Unavailable)?;
            let remaining = table
                .get(generation_key.as_slice())
                .map_err(|_| ReserveError::Unavailable)?
                .map(|value| value.value())
                .unwrap_or(0);
            if remaining == 0 {
                return Err(ReserveError::AuthorityExhausted);
            }
            table
                .insert(generation_key.as_slice(), remaining - 1)
                .map_err(|_| ReserveError::Unavailable)?;
        }
        hit(ReservationFailpoint::AfterAuthorityDecrement)?;

        {
            let mut table = transaction
                .open_table(INTENTS)
                .map_err(|_| ReserveError::Unavailable)?;
            let mut record: IntentRecord = {
                let existing = table
                    .get(intent_uuid.as_bytes().as_slice())
                    .map_err(|_| ReserveError::Unavailable)?
                    .ok_or(ReserveError::UnknownIntent)?;
                serde_json::from_slice(existing.value()).map_err(|_| ReserveError::Unavailable)?
            };
            require_prepared(record.state)?;
            record.state = IntentState::AuthorityReserved;
            let bytes = serde_json::to_vec(&record).map_err(|_| ReserveError::Unavailable)?;
            table
                .insert(intent_uuid.as_bytes().as_slice(), bytes.as_slice())
                .map_err(|_| ReserveError::Unavailable)?;
        }
        hit(ReservationFailpoint::AfterIntentTransition)?;
        kill_barrier(BARRIER_BEFORE_COMMIT);
        Ok(intent_id)
    })?;
    kill_barrier(BARRIER_AFTER_COMMIT);
    Ok(AuthorityReservedEffect::from_committed(committed))
}

fn recheck_bindings(
    transaction: &redb::WriteTransaction,
    context: &ReservationContext,
    capability: &VerifiedCapabilityV2,
    approval: &VerifiedApprovalV1,
) -> Result<(), ReserveError> {
    if context.now_unix >= approval.expires_at_unix()
        || context.now_unix >= capability.expires_at_unix()
    {
        return Err(ReserveError::Expired);
    }

    let claims = capability.claims();
    if claims.risk_class != RiskClass::LowRiskEffectful
        || claims.backend != sovereign_artifact::ArtifactBackend::CoreWasm
        || claims.tool.tool_id != CLOSED_FIXTURE_TOOL_ID
        || claims.tool.tool_version != CLOSED_FIXTURE_TOOL_VERSION
        || claims.tool.operation != CLOSED_FIXTURE_OPERATION_ID
    {
        return Err(ReserveError::ProfileMismatch);
    }

    let intent_stem = context.intent_id.file_stem();
    if claims.primary_resource != intent_stem
        || approval.claims().primary_resource != intent_stem
        || claims.session_id != context.session_id
        || approval.claims().session_id != context.session_id
        || capability.approval_id() != Some(approval.approval_id())
    {
        return Err(ReserveError::BindingMismatch);
    }

    if revoked(transaction, REVOKED_TOKENS, capability.token_id())?
        || revoked(transaction, REVOKED_APPROVALS, approval.approval_id())?
    {
        return Err(ReserveError::Revoked);
    }

    let intents = transaction
        .open_table(INTENTS)
        .map_err(|_| ReserveError::Unavailable)?;
    let Some(existing) = intents
        .get(context.intent_id.as_uuid().as_bytes().as_slice())
        .map_err(|_| ReserveError::Unavailable)?
    else {
        return Err(ReserveError::UnknownIntent);
    };
    let record: IntentRecord =
        serde_json::from_slice(existing.value()).map_err(|_| ReserveError::Unavailable)?;
    drop(existing);
    drop(intents);
    require_prepared(record.state)?;
    if record.session_id != context.session_id.to_string() {
        return Err(ReserveError::SessionMismatch);
    }
    if record.logout_epoch != context.logout_epoch {
        return Err(ReserveError::LogoutMismatch);
    }
    if record.signer_epoch != hex::encode(context.signer_epoch) {
        return Err(ReserveError::EpochMismatch);
    }
    if record.fixture_generation != context.fixture_generation {
        return Err(ReserveError::GenerationMismatch);
    }
    if record.policy_decision_id != claims.policy_decision_id.to_string()
        || record.policy_decision_id != approval.claims().policy_decision_id.to_string()
    {
        return Err(ReserveError::BindingMismatch);
    }
    Ok(())
}

fn revoked(
    transaction: &redb::WriteTransaction,
    table: TableDefinition<&[u8], u8>,
    id: Uuid,
) -> Result<bool, ReserveError> {
    let open = match transaction.open_table(table) {
        Ok(open) => open,
        Err(redb::TableError::TableDoesNotExist(_)) => return Ok(false),
        Err(_) => return Err(ReserveError::Unavailable),
    };
    let found = open
        .get(id.as_bytes().as_slice())
        .map_err(|_| ReserveError::Unavailable)?
        .is_some();
    Ok(found)
}

fn uuid_from_bytes(bytes: &[u8]) -> Result<Uuid, ReserveError> {
    let array: [u8; 16] = bytes.try_into().map_err(|_| ReserveError::Unavailable)?;
    Ok(Uuid::from_bytes(array))
}

/// Inspect durable reservation facts. Cannot construct the reserved handle.
pub fn inspect_reservation(
    store: &OwnedStore<'_>,
    intent_id: EffectIntentId,
    approval_id: Uuid,
    token_id: Uuid,
    idempotency_key: Uuid,
    fixture_generation: u64,
) -> Result<ReservationView, ReserveError> {
    store.read(|transaction| {
        let intent_state = match transaction.open_table(INTENTS) {
            Ok(table) => table
                .get(intent_id.as_uuid().as_bytes().as_slice())
                .map_err(|_| ReserveError::Unavailable)?
                .map(|value| {
                    serde_json::from_slice::<IntentRecord>(value.value())
                        .map(|record| record.state)
                        .map_err(|_| ReserveError::Unavailable)
                })
                .transpose()?,
            Err(redb::TableError::TableDoesNotExist(_)) => None,
            Err(_) => return Err(ReserveError::Unavailable),
        };
        let (approval_claimed, approval_expires_at_unix) =
            expiry_claim(transaction, APPROVALS, approval_id)?;
        let (token_claimed, _) = expiry_claim(transaction, TOKENS, token_id)?;
        let idempotency_bound_to = match transaction.open_table(IDEMPOTENCY) {
            Ok(table) => table
                .get(idempotency_key.as_bytes().as_slice())
                .map_err(|_| ReserveError::Unavailable)?
                .map(|value| uuid_from_bytes(value.value()))
                .transpose()?,
            Err(redb::TableError::TableDoesNotExist(_)) => None,
            Err(_) => return Err(ReserveError::Unavailable),
        };
        let generation_key = fixture_generation.to_be_bytes();
        let authority_uses_remaining = match transaction.open_table(AUTHORITY_NODE) {
            Ok(table) => table
                .get(generation_key.as_slice())
                .map_err(|_| ReserveError::Unavailable)?
                .map(|value| value.value())
                .unwrap_or(0),
            Err(redb::TableError::TableDoesNotExist(_)) => 0,
            Err(_) => return Err(ReserveError::Unavailable),
        };
        Ok(ReservationView {
            intent_state,
            approval_claimed,
            approval_expires_at_unix,
            token_claimed,
            idempotency_bound_to,
            authority_uses_remaining,
        })
    })
}

fn expiry_claim(
    transaction: &redb::ReadTransaction,
    table: TableDefinition<&[u8], &[u8]>,
    id: Uuid,
) -> Result<(bool, Option<i64>), ReserveError> {
    let open = match transaction.open_table(table) {
        Ok(open) => open,
        Err(redb::TableError::TableDoesNotExist(_)) => return Ok((false, None)),
        Err(_) => return Err(ReserveError::Unavailable),
    };
    match open
        .get(id.as_bytes().as_slice())
        .map_err(|_| ReserveError::Unavailable)?
    {
        None => Ok((false, None)),
        Some(value) => {
            let claim: ExpiryClaim =
                serde_json::from_slice(value.value()).map_err(|_| ReserveError::Unavailable)?;
            Ok((true, Some(claim.expires_at_unix)))
        }
    }
}

/// Value-free intent state. Cannot reconstruct a reserved handle.
pub fn inspect_intent_state(
    store: &OwnedStore<'_>,
    intent_id: EffectIntentId,
) -> Result<Option<IntentState>, ReserveError> {
    store.read(|transaction| {
        let table = match transaction.open_table(INTENTS) {
            Ok(table) => table,
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(None),
            Err(_) => return Err(ReserveError::Unavailable),
        };
        match table
            .get(intent_id.as_uuid().as_bytes().as_slice())
            .map_err(|_| ReserveError::Unavailable)?
        {
            None => Ok(None),
            Some(value) => {
                let record: IntentRecord =
                    serde_json::from_slice(value.value()).map_err(|_| ReserveError::Unavailable)?;
                Ok(Some(record.state))
            }
        }
    })
}

pub(crate) fn load_intent_record(
    store: &OwnedStore<'_>,
    intent_id: EffectIntentId,
) -> Result<IntentRecord, ReserveError> {
    store.read(|transaction| {
        let table = transaction
            .open_table(INTENTS)
            .map_err(|_| ReserveError::Unavailable)?;
        let existing = table
            .get(intent_id.as_uuid().as_bytes().as_slice())
            .map_err(|_| ReserveError::Unavailable)?
            .ok_or(ReserveError::UnknownIntent)?;
        serde_json::from_slice(existing.value()).map_err(|_| ReserveError::Unavailable)
    })
}

pub(crate) fn store_intent_record(
    store: &OwnedStore<'_>,
    intent_id: EffectIntentId,
    record: &IntentRecord,
) -> Result<(), ReserveError> {
    let bytes = serde_json::to_vec(record).map_err(|_| ReserveError::Unavailable)?;
    store.write(|transaction| {
        transaction
            .open_table(INTENTS)
            .map_err(|_| ReserveError::Unavailable)?
            .insert(intent_id.as_uuid().as_bytes().as_slice(), bytes.as_slice())
            .map_err(|_| ReserveError::Unavailable)?;
        Ok(())
    })
}

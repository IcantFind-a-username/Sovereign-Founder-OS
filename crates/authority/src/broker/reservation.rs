//! Reserving everything an effect needs, in one transaction or not at all.
//!
//! An effect needs six things to be true at once: an approval that has not
//! been spent, a one-use token that has not been consumed, an idempotency key
//! that is either unbound or bound to this exact fingerprint, the root it will
//! act on, a use-count that has not been exhausted, and the effect id itself
//! not already reserved.
//!
//! Checking them one at a time and then acting is the classic mistake, and it
//! fails in two directions at once. Between the check and the act, another
//! caller can consume the token — so the check was worthless. And if the
//! fourth reservation fails after three succeeded, three things are now spent
//! on an effect that will never happen, which a retry cannot recover because
//! they are one-use by design.
//!
//! So this is one transaction. Redb gives it atomicity: every reservation is
//! written inside a single write transaction, and a failure anywhere aborts
//! the whole thing by dropping the transaction without committing. Nothing
//! partial is ever visible, and nothing is spent on an effect that did not
//! start.
//!
//! What redb does not give is authenticity — it is ACID and unencrypted. The
//! atomicity here is about consistency, not about trust.

use super::store::{OwnedStore, StoreError};
use redb::{ReadableTable, TableDefinition};
use uuid::Uuid;

/// One table per reservation kind. Separate tables rather than one keyed by a
/// kind tag, so a bug that wrote the wrong kind would be a type error at the
/// call site rather than a silent overwrite of somebody else's key.
const APPROVALS: TableDefinition<&[u8], u64> = TableDefinition::new("reserved-approvals");
const TOKENS: TableDefinition<&[u8], u64> = TableDefinition::new("reserved-tokens");
const IDEMPOTENCY: TableDefinition<&[u8], &[u8]> = TableDefinition::new("bound-idempotency");
const EFFECTS: TableDefinition<&[u8], u64> = TableDefinition::new("reserved-effects");

/// Everything one effect needs reserved. All of it, together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReservationRequest {
    pub approval_id: Uuid,
    pub token_id: Uuid,
    pub idempotency_key: Uuid,
    /// What the idempotency key must be bound to. A key already bound to a
    /// different fingerprint is a conflict, not a replay.
    pub invocation_fingerprint: [u8; 32],
    pub effect_intent_id: Uuid,
    /// The generation of the root this acts on. A reservation made against
    /// one root must not be honoured against another.
    pub root_generation: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReservationError {
    ApprovalAlreadySpent,
    TokenAlreadyConsumed,
    /// The key is bound to a different fingerprint.
    IdempotencyConflict,
    /// The key is bound to this exact fingerprint: a genuine retry.
    IdempotencyReplay,
    EffectAlreadyReserved,
    /// The store could not answer. Never treated as permission.
    Unavailable,
}

impl From<StoreError> for ReservationError {
    fn from(_: StoreError) -> Self {
        ReservationError::Unavailable
    }
}

/// Reserve everything, or nothing.
///
/// Idempotency is checked first for readability, and *not* because the order
/// is load-bearing. Inside one transaction it is not: a replay detected after
/// the one-use claims were written still aborts, and the writes are dropped
/// with it. Reversing the order changes nothing an observer can see, which a
/// test confirmed by reversing it.
///
/// Saying so matters because the plausible-sounding version — "a retry must
/// be recognised before anything is spent" — describes a design where each
/// claim commits separately, and would be a reason to trust the order rather
/// than the transaction. The guarantee here comes from atomicity alone.
pub fn reserve(
    store: &OwnedStore<'_>,
    request: &ReservationRequest,
) -> Result<(), ReservationError> {
    store.write(
        |transaction: &redb::WriteTransaction| -> Result<(), ReservationError> {
            // Idempotency first: the cheapest check and the most common outcome
            // for a caller that is retrying. Not a correctness requirement — see
            // the note on this function.
            {
                let table = transaction
                    .open_table(IDEMPOTENCY)
                    .map_err(|_| ReservationError::Unavailable)?;
                let bound = table
                    .get(request.idempotency_key.as_bytes().as_slice())
                    .map_err(|_| ReservationError::Unavailable)?
                    .map(|value| value.value().to_vec());
                if let Some(bound) = bound {
                    return Err(if bound == request.invocation_fingerprint {
                        ReservationError::IdempotencyReplay
                    } else {
                        ReservationError::IdempotencyConflict
                    });
                }
            }

            // Then the three one-use claims. Each carries its own outcome, so a
            // caller learns which one was already spent rather than only that
            // something was.
            for (table, key, taken) in [
                (
                    APPROVALS,
                    request.approval_id,
                    ReservationError::ApprovalAlreadySpent,
                ),
                (
                    TOKENS,
                    request.token_id,
                    ReservationError::TokenAlreadyConsumed,
                ),
                (
                    EFFECTS,
                    request.effect_intent_id,
                    ReservationError::EffectAlreadyReserved,
                ),
            ] {
                let mut open = transaction
                    .open_table(table)
                    .map_err(|_| ReservationError::Unavailable)?;
                if open
                    .get(key.as_bytes().as_slice())
                    .map_err(|_| ReservationError::Unavailable)?
                    .is_some()
                {
                    return Err(taken);
                }
                open.insert(key.as_bytes().as_slice(), request.root_generation)
                    .map_err(|_| ReservationError::Unavailable)?;
            }

            // The binding is written last, so a failure above leaves the key
            // unbound and a later attempt is a first attempt rather than a replay
            // of something that never happened.
            transaction
                .open_table(IDEMPOTENCY)
                .map_err(|_| ReservationError::Unavailable)?
                .insert(
                    request.idempotency_key.as_bytes().as_slice(),
                    request.invocation_fingerprint.as_slice(),
                )
                .map_err(|_| ReservationError::Unavailable)?;
            Ok(())
        },
    )
}

/// Whether a key is reserved, for tests and for a caller reconciling state.
pub fn is_reserved(
    store: &OwnedStore<'_>,
    kind: Kind,
    key: Uuid,
) -> Result<bool, ReservationError> {
    let table = match kind {
        Kind::Approval => APPROVALS,
        Kind::Token => TOKENS,
        Kind::Effect => EFFECTS,
    };
    store.read(
        |transaction: &redb::ReadTransaction| -> Result<bool, ReservationError> {
            let open = match transaction.open_table(table) {
                Ok(open) => open,
                // A table that was never created holds nothing.
                Err(_) => return Ok(false),
            };
            Ok(open
                .get(key.as_bytes().as_slice())
                .map_err(|_| ReservationError::Unavailable)?
                .is_some())
        },
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Approval,
    Token,
    Effect,
}

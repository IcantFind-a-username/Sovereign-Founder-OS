//! Value-free signed fixture evidence. Describes; never decides.

use redb::{ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sovereign_artifact::Digest;
use sovereign_authority::broker::store::OwnedStore;
use sovereign_identity::{AuditRole, TypedSigner};
use uuid::Uuid;

use crate::outcome::{ClosedOutcome, PublishError};
use crate::sealed::EffectIntentId;

const EVIDENCE: TableDefinition<&[u8], &[u8]> =
    TableDefinition::new("fixture-value-free-evidence-v1");
const EVIDENCE_ORDER: TableDefinition<u64, &[u8]> =
    TableDefinition::new("fixture-value-free-evidence-order-v1");

pub const EVIDENCE_VERSION: u32 = 1;
pub const EVIDENCE_TYPE: &str = "synthetic-exact-local-outbox-v1";
pub const EVIDENCE_ISSUER: &str = "fixture.unqualified.evidence";

/// Public projection of one terminal event. No recipient, content, path,
/// size, time, or digest of a low-entropy value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedFixtureEvidence {
    pub version: u32,
    #[serde(rename = "type")]
    pub record_type: String,
    pub event_id: String,
    pub intent_id: String,
    pub outcome: String,
    pub previous_event_hash: String,
    pub signer_issuer: String,
    pub signer_key_id: String,
    pub event_hash: String,
    pub signature: String,
}

pub fn listed_evidence(store: &OwnedStore<'_>) -> Result<Vec<SignedFixtureEvidence>, PublishError> {
    store.read(|transaction| {
        let order = match transaction.open_table(EVIDENCE_ORDER) {
            Ok(order) => order,
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(Vec::new()),
            Err(_) => return Err(PublishError::Unavailable),
        };
        let table = match transaction.open_table(EVIDENCE) {
            Ok(table) => table,
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(Vec::new()),
            Err(_) => return Err(PublishError::Unavailable),
        };
        let mut listed = Vec::new();
        let mut seq = 0u64;
        loop {
            let Some(key) = order
                .get(&seq)
                .map_err(|_| PublishError::Unavailable)?
                .map(|value| value.value().to_vec())
            else {
                break;
            };
            if let Some(value) = table
                .get(key.as_slice())
                .map_err(|_| PublishError::Unavailable)?
            {
                let record: SignedFixtureEvidence =
                    serde_json::from_slice(value.value()).map_err(|_| PublishError::Unavailable)?;
                listed.push(record);
            }
            seq += 1;
        }
        Ok(listed)
    })
}

pub(crate) fn append_evidence(
    store: &OwnedStore<'_>,
    intent_id: EffectIntentId,
    outcome: ClosedOutcome,
) -> Result<SignedFixtureEvidence, PublishError> {
    let previous = listed_evidence(store)?
        .last()
        .map(|record| record.event_hash.clone())
        .unwrap_or_else(|| hex::encode([0u8; 32]));
    let record = sign_record(intent_id, outcome, previous)?;
    let key = intent_id.as_uuid();
    let bytes = serde_json::to_vec(&record).map_err(|_| PublishError::Unavailable)?;
    store.write(|transaction| -> Result<(), PublishError> {
        let existing = transaction
            .open_table(EVIDENCE)
            .map_err(|_| PublishError::Unavailable)?;
        if existing
            .get(key.as_bytes().as_slice())
            .map_err(|_| PublishError::Unavailable)?
            .is_some()
        {
            return Ok(());
        }
        drop(existing);
        let mut order = transaction
            .open_table(EVIDENCE_ORDER)
            .map_err(|_| PublishError::Unavailable)?;
        let mut seq = 0u64;
        while order
            .get(&seq)
            .map_err(|_| PublishError::Unavailable)?
            .is_some()
        {
            seq += 1;
        }
        order
            .insert(seq, key.as_bytes().as_slice())
            .map_err(|_| PublishError::Unavailable)?;
        drop(order);
        transaction
            .open_table(EVIDENCE)
            .map_err(|_| PublishError::Unavailable)?
            .insert(key.as_bytes().as_slice(), bytes.as_slice())
            .map_err(|_| PublishError::Unavailable)?;
        Ok(())
    })?;
    Ok(record)
}

fn sign_record(
    intent_id: EffectIntentId,
    outcome: ClosedOutcome,
    previous_event_hash: String,
) -> Result<SignedFixtureEvidence, PublishError> {
    let signer = TypedSigner::<AuditRole>::generate(EVIDENCE_ISSUER)
        .map_err(|_| PublishError::Unavailable)?;
    let event_id = Uuid::new_v4();
    let body = json!({
        "event_id": event_id.to_string(),
        "intent_id": intent_id.as_uuid().to_string(),
        "outcome": outcome.as_str(),
        "previous_event_hash": previous_event_hash,
        "signer_issuer": EVIDENCE_ISSUER,
        "signer_key_id": hex::encode(signer.key_id()),
        "type": EVIDENCE_TYPE,
        "version": EVIDENCE_VERSION,
    });
    let canonical =
        serde_json_canonicalizer::to_vec(&body).map_err(|_| PublishError::Unavailable)?;
    let event_hash = Digest::of_bytes(&canonical).as_hex();
    let mut to_sign = body;
    if let Value::Object(map) = &mut to_sign {
        map.insert("event_hash".into(), Value::String(event_hash.clone()));
    }
    let signed_canonical =
        serde_json_canonicalizer::to_vec(&to_sign).map_err(|_| PublishError::Unavailable)?;
    let signature = signer
        .sign_cose(&signed_canonical)
        .map_err(|_| PublishError::Unavailable)?;
    Ok(SignedFixtureEvidence {
        version: EVIDENCE_VERSION,
        record_type: EVIDENCE_TYPE.to_owned(),
        event_id: event_id.to_string(),
        intent_id: intent_id.as_uuid().to_string(),
        outcome: outcome.as_str().to_owned(),
        previous_event_hash,
        signer_issuer: EVIDENCE_ISSUER.to_owned(),
        signer_key_id: hex::encode(signer.key_id()),
        event_hash,
        signature: hex::encode(signature),
    })
}

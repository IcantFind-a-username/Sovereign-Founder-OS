//! What the fixture's evidence chain may remember about an effect.
//!
//! An audit record answers "did this happen, in what order, was it tampered
//! with". It does not answer "what was it". The tests here are about keeping
//! those apart, and the sharpest of them is the dictionary oracle: redaction
//! leaks, and a field replaced by its hash still answers questions when the
//! set of possible answers is small — which, for the people a founder emails,
//! it is.

#![cfg(feature = "owner-effect-fixture")]

use sovereign_audit_ledger::effect_v1::{
    EffectEvidenceChain, EffectEvidenceV1, EffectOutcome, EvidenceError, FIXTURE_SIGNER,
};
use uuid::Uuid;

#[test]
fn a_record_carries_only_the_allowlisted_fields() {
    let record = EffectEvidenceV1::new(Uuid::new_v4(), EffectOutcome::Dispatched, 0, [0; 32]);

    // Rendered in full and read back: whatever the record can say about
    // itself is exactly these five things.
    let rendered = format!("{record:?}");
    for field in [
        "intent_id",
        "outcome",
        "sequence",
        "previous_hash",
        "signer",
    ] {
        assert!(
            rendered.contains(field),
            "{field} is missing from {rendered}"
        );
    }

    // And the struct has no other field to add one to.
    let source = include_str!("../src/effect_v1.rs");
    let code: String = source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    let declaration = code
        .split("pub struct EffectEvidenceV1 {")
        .nth(1)
        .and_then(|rest| rest.split('}').next())
        .expect("the struct declaration");
    let fields: Vec<&str> = declaration
        .lines()
        .filter_map(|line| line.trim().strip_suffix(','))
        .filter_map(|line| line.split(':').next())
        .map(|name| name.trim_start_matches("pub ").trim())
        .collect();
    assert_eq!(
        fields,
        vec![
            "intent_id",
            "outcome",
            "sequence",
            "previous_hash",
            "signer"
        ],
        "the record gained a field"
    );
}

/// The names that must never appear. Each is something an audit record
/// plausibly wants and each answers "what was it" rather than "did it
/// happen".
#[test]
fn the_record_omits_everything_that_describes_the_effect() {
    let source = include_str!("../src/effect_v1.rs");
    let code: String = source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    let declaration = code
        .split("pub struct EffectEvidenceV1 {")
        .nth(1)
        .and_then(|rest| rest.split('}').next())
        .expect("the struct declaration");

    for forbidden in [
        "recipient",
        "content",
        "digest",
        "path",
        "size",
        "bytes",
        "time",
        "timestamp",
        "subject",
        "body",
        "policy",
        "reason",
        "detail",
        "extra",
    ] {
        assert!(
            !declaration.contains(forbidden),
            "the record carries {forbidden:?}, which describes the effect rather than its occurrence"
        );
    }
}

/// The sharp one. Two effects that differ only in their content must produce
/// projections that differ in no way an observer can use.
///
/// If any field were derived from the content — a hash, a length, anything —
/// then an observer holding a small dictionary of candidates could compute
/// each one and learn which it was. The set of people a founder emails is a
/// small dictionary.
#[test]
fn a_low_entropy_dictionary_gives_no_projection_oracle() {
    let dictionary = [
        "alice@example.test",
        "bob@example.test",
        "carol@example.test",
        "dave@example.test",
    ];

    // The same intent and outcome, with wildly different content behind them.
    // The projection takes no content at all, so the only way these could
    // differ is if the type had a field for it.
    let intent = Uuid::new_v4();
    let mut projections = Vec::new();
    for _recipient in dictionary {
        let record = EffectEvidenceV1::new(intent, EffectOutcome::Dispatched, 0, [0; 32]);
        projections.push((format!("{record:?}"), record.hash()));
    }

    let first = &projections[0];
    for (index, projection) in projections.iter().enumerate() {
        assert_eq!(
            projection, first,
            "projection {index} differs from the first, so the content is observable"
        );
    }
}

/// A record found on its own, with no context, must still be recognisable as
/// fixture evidence rather than product evidence.
#[test]
fn every_record_is_tagged_as_a_fixture_signer() {
    let record = EffectEvidenceV1::new(Uuid::new_v4(), EffectOutcome::Refused, 0, [0; 32]);
    assert_eq!(record.signer, FIXTURE_SIGNER);
    assert!(
        FIXTURE_SIGNER.contains("synthetic") && FIXTURE_SIGNER.contains("fixture"),
        "the tag must say what it is without needing context: {FIXTURE_SIGNER}"
    );
}

#[test]
fn the_chain_verifies_and_each_record_links_to_the_last() {
    let mut chain = EffectEvidenceChain::new();
    let first = chain
        .append(Uuid::new_v4(), EffectOutcome::Dispatched)
        .unwrap()
        .clone();
    let second = chain
        .append(Uuid::new_v4(), EffectOutcome::Refused)
        .unwrap()
        .clone();

    assert_eq!(first.sequence, 0);
    assert_eq!(first.previous_hash, [0; 32]);
    assert_eq!(second.sequence, 1);
    assert_eq!(second.previous_hash, first.hash());
    assert!(chain.verify().is_ok());
}

/// Appending the same outcome for the same intent again is the recovery from
/// a crash between deciding an effect's fate and recording it, so it must not
/// grow the chain or fail.
#[test]
fn the_same_outcome_for_one_intent_appends_once() {
    let mut chain = EffectEvidenceChain::new();
    let intent = Uuid::new_v4();

    let first = chain
        .append(intent, EffectOutcome::Dispatched)
        .unwrap()
        .hash();
    let again = chain
        .append(intent, EffectOutcome::Dispatched)
        .unwrap()
        .hash();

    assert_eq!(first, again, "a replay produced a different record");
    assert_eq!(chain.records().len(), 1, "a replay grew the chain");
    assert!(chain.verify().is_ok());
}

/// Two answers to the same question is a defect, not something to average.
/// The second write is refused rather than allowed to overwrite the first.
#[test]
fn a_different_outcome_for_one_intent_conflicts() {
    let mut chain = EffectEvidenceChain::new();
    let intent = Uuid::new_v4();
    chain.append(intent, EffectOutcome::Dispatched).unwrap();

    assert_eq!(
        chain.append(intent, EffectOutcome::Refused).err(),
        Some(EvidenceError::OutcomeConflict)
    );
    assert_eq!(chain.records()[0].outcome, EffectOutcome::Dispatched);
}

/// An unknown outcome must stay unknown. Relabelling it later on the strength
/// of a guess is how an audit chain starts recording what someone assumed
/// instead of what was observed.
#[test]
fn an_indeterminate_outcome_is_never_relabelled() {
    let mut chain = EffectEvidenceChain::new();
    let intent = Uuid::new_v4();
    chain.append(intent, EffectOutcome::Indeterminate).unwrap();

    for later in [EffectOutcome::Dispatched, EffectOutcome::Refused] {
        assert_eq!(
            chain.append(intent, later).err(),
            Some(EvidenceError::OutcomeConflict),
            "an indeterminate outcome was relabelled {later:?}"
        );
    }
    assert_eq!(chain.records()[0].outcome, EffectOutcome::Indeterminate);
}

/// Every field is covered by the hash, so tampering with any one breaks the
/// chain rather than only the ones an author remembered.
#[test]
fn tampering_with_any_field_changes_the_hash() {
    let intent = Uuid::new_v4();
    let base = EffectEvidenceV1::new(intent, EffectOutcome::Dispatched, 3, [7; 32]);

    let variants = [
        EffectEvidenceV1::new(Uuid::new_v4(), EffectOutcome::Dispatched, 3, [7; 32]),
        EffectEvidenceV1::new(intent, EffectOutcome::Refused, 3, [7; 32]),
        EffectEvidenceV1::new(intent, EffectOutcome::Dispatched, 4, [7; 32]),
        EffectEvidenceV1::new(intent, EffectOutcome::Dispatched, 3, [8; 32]),
    ];
    for (index, variant) in variants.iter().enumerate() {
        assert_ne!(
            base.hash(),
            variant.hash(),
            "field {index} is not covered by the hash"
        );
    }
}

/// The chain describes; it does not decide. There is no method that changes
/// an effect's state, so evidence cannot advance one.
#[test]
fn evidence_never_advances_an_effect() {
    let source = include_str!("../src/effect_v1.rs");
    let code: String = source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    for mutator in [
        "fn dispatch",
        "fn set_outcome",
        "fn advance",
        "fn retry",
        "fn execute",
    ] {
        assert!(
            !code.contains(mutator),
            "the evidence chain exposes {mutator:?}, so it can change what it is supposed to record"
        );
    }
    assert!(
        code.contains("pub fn append"),
        "stripping comments removed the code as well"
    );
}

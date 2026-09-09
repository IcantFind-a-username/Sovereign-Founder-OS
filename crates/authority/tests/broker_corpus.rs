//! The one synthetic corpus the fixture may hold.
//!
//! Freezing this is not test hygiene. Redb is ACID but unencrypted, so
//! whatever the fixture persists sits in a plaintext file; if a real address
//! could reach it, the fixture would have created a small unencrypted store
//! of the founder's contacts as a side effect of proving a protocol.

#![cfg(feature = "owner-effect-fixture")]

use sovereign_authority::broker::corpus::{
    contains_canary, validate_recipient, CorpusError, BODY_CANARY, CANARIES, RECIPIENT, SENDER,
    SUBJECT_CANARY,
};

#[test]
fn the_frozen_recipient_is_accepted() {
    assert_eq!(validate_recipient(RECIPIENT), Ok(RECIPIENT));
}

/// An allow-list of exactly one, compared in full. Each rejection below is a
/// shape that a weaker check would let through, which is why they are listed
/// rather than summarised.
#[test]
fn every_other_recipient_is_refused() {
    let refused = [
        // Stands for a real address — the case the constant exists for.
        // Written at a reserved domain on purpose: a repository that spells
        // out a live mailbox to prove it is rejected has still written down a
        // live mailbox.
        "a-person@their-employer.example",
        // Suffix matching would accept this.
        "evil@example.test.attacker.example",
        // A domain check would accept both of these.
        "someone-else@example.test",
        "fixture-recipient@example.invalid",
        // Case and whitespace are not normalised into a match.
        "FIXTURE-RECIPIENT@EXAMPLE.TEST",
        " fixture-recipient@example.test",
        "fixture-recipient@example.test ",
        // A prefix check would accept this.
        "fixture-recipient@example.test,someone-else@elsewhere.example",
        "",
    ];
    for candidate in refused {
        assert_eq!(
            validate_recipient(candidate),
            Err(CorpusError::RecipientNotFrozen),
            "recipient {candidate:?} was accepted"
        );
    }
}

/// Every constant sits at a domain that can never be registered — RFC 2606
/// and RFC 6761 reserve them — so none of these strings can become a real
/// address belonging to a real person.
#[test]
fn every_address_uses_a_permanently_unregistrable_domain() {
    for address in [RECIPIENT, SENDER] {
        let domain = address.split('@').nth(1).expect("an address has a domain");
        assert!(
            domain.ends_with(".test")
                || domain.ends_with(".invalid")
                || domain.starts_with("example.")
                || domain == "example.com",
            "{address} uses {domain}, which someone could register"
        );
    }
}

/// A scanner can only prove a surface is canary-free if the canaries cannot
/// occur by accident.
#[test]
fn the_canaries_are_distinctive_enough_to_search_for() {
    for canary in [SUBJECT_CANARY, BODY_CANARY] {
        assert!(
            canary.len() >= 20,
            "{canary} is too short to be distinctive"
        );
        assert!(
            canary.contains("SFO-FIXTURE"),
            "{canary} is not recognisably ours"
        );
    }
    // And they are all distinct, so a hit names which surface leaked.
    let mut seen = CANARIES.to_vec();
    seen.sort_unstable();
    seen.dedup();
    assert_eq!(
        seen.len(),
        CANARIES.len(),
        "two canaries are the same string"
    );
}

#[test]
fn contains_canary_finds_each_one_anywhere_in_a_blob() {
    for canary in CANARIES {
        let blob = format!("prefix bytes {canary} trailing bytes");
        assert!(
            contains_canary(blob.as_bytes()),
            "{canary} was not found inside a larger blob"
        );
    }
    assert!(!contains_canary(b"nothing to see here"));
    assert!(!contains_canary(b""));
}

/// The scanner must not report a hit for a near miss, or every value-free
/// assertion becomes noise a reader learns to ignore.
#[test]
fn contains_canary_does_not_fire_on_near_misses() {
    for near in [
        "SFO-FIXTURE-SUBJECT-CANARY",
        "fixture-recipient@example",
        "SFO-FIXTURE",
    ] {
        assert!(
            !contains_canary(near.as_bytes()),
            "{near:?} was reported as a canary"
        );
    }
}

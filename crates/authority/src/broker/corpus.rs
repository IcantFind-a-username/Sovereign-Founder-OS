//! The one synthetic corpus the fixture may ever hold.
//!
//! RFC 0006 freezes this: the only recipient is a compile-time constant, and
//! the sender, subject and body are canaries. It is worth being clear about
//! what that buys, because "use test data" sounds like hygiene and this is
//! not hygiene.
//!
//! Redb is ACID and crash-safe. It is not encrypted and not authenticated, so
//! whatever the fixture persists sits in a file in plaintext. If a real
//! address could reach that file, the fixture would have created a small
//! unencrypted store of the founder's contacts as a side effect of proving a
//! protocol. Freezing the corpus means the worst case is a file full of
//! strings that were already public constants in this repository.
//!
//! The canaries have a second job. Every table, log, error, IPC field and
//! export outside a narrow allowlist must be provably free of them, and a
//! scanner can only prove that if it knows exactly what to look for. Values
//! chosen at runtime cannot be searched for; these can.
//!
//! Reserved by RFC 2606 and RFC 6761: `.test`, `.invalid` and `example.*` can
//! never be registered, so none of these strings can become a real address
//! held by a real person.

/// The one and only recipient. Not a default, not a placeholder — the
/// validator below accepts this and nothing else.
pub const RECIPIENT: &str = "fixture-recipient@example.test";

/// Fixed sender, subject and body. Distinctive on purpose: a scanner looking
/// for leakage needs strings that cannot occur by accident.
pub const SENDER: &str = "fixture-sender@example.test";
pub const SUBJECT_CANARY: &str = "SFO-FIXTURE-SUBJECT-CANARY-8f2a1c";
pub const BODY_CANARY: &str = "SFO-FIXTURE-BODY-CANARY-4d7e93";

/// Every canary, so a scanner enumerates them rather than a person
/// remembering to add each new one.
pub const CANARIES: &[&str] = &[RECIPIENT, SENDER, SUBJECT_CANARY, BODY_CANARY];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CorpusError {
    /// Anything other than the frozen recipient.
    RecipientNotFrozen,
}

/// Accept the frozen recipient and nothing else.
///
/// An allow-list of exactly one, compared in full. Not a suffix check, which
/// `<attacker>@example.test.<their-domain>` passes; not a domain check, which admits
/// every address at a reserved domain; not a syntax check, which admits every
/// well-formed address in the world.
pub fn validate_recipient(candidate: &str) -> Result<&'static str, CorpusError> {
    if candidate == RECIPIENT {
        Ok(RECIPIENT)
    } else {
        Err(CorpusError::RecipientNotFrozen)
    }
}

/// Whether a blob carries any canary. Used by tests and by anything that must
/// prove a value-free surface really is one.
pub fn contains_canary(bytes: &[u8]) -> bool {
    CANARIES
        .iter()
        .any(|canary| bytes.windows(canary.len()).any(|w| w == canary.as_bytes()))
}

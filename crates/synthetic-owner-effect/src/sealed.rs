//! Allocate a random intent, then compose the one RFC 5322 message.
//!
//! The identifier is allocated before anything is composed. An id derived
//! from content would leak its subject to anyone who can see a filename, and
//! it could not name an intent that has not been composed yet.
//!
//! The payload is private. There is no public accessor that returns its
//! bytes. A caller can cause a message to exist and cannot read it.

use std::fmt;

use sovereign_authority::broker::corpus::{BODY_CANARY, RECIPIENT, SENDER, SUBJECT_CANARY};
use uuid::Uuid;

/// A fixed date, in RFC 5322 form. Synthetic on purpose: a real one would
/// make the composition nondeterministic and would record when.
pub const FIXTURE_DATE: &str = "Thu, 01 Jan 1970 00:00:00 +0000";

/// Immutable coordinator reference bound into Capability V2 canonical input.
pub const COORDINATOR_REF: &str = "synthetic-owner-effect-v2";

/// An opaque name for one intent. Allocated before composition.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct EffectIntentId(Uuid);

impl EffectIntentId {
    /// The only way to make one. Random, and unrelated to any payload.
    pub fn allocate() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn as_uuid(self) -> Uuid {
        self.0
    }

    pub(crate) fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }

    /// A stable file-safe name. Derived from the id and nothing else.
    pub fn file_stem(&self) -> String {
        self.0.simple().to_string()
    }
}

impl fmt::Debug for EffectIntentId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "EffectIntentId({})", self.0.simple())
    }
}

/// The composed message, held privately.
///
/// Not `pub` fields, no `content()`, no `Deref`, no `AsRef<[u8]>`.
pub struct SealedPayload {
    intent_id: EffectIntentId,
    bytes: Vec<u8>,
}

impl fmt::Debug for SealedPayload {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SealedPayload")
            .field("intent_id", &self.intent_id)
            .field("bytes", &"<protected>")
            .finish()
    }
}

/// What a caller may know about a payload: its shape, never its substance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixturePreview {
    pub intent_id: EffectIntentId,
    pub header_count: usize,
    pub is_synthetic: bool,
}

impl SealedPayload {
    /// Compose the one message the fixture may produce, for an id that was
    /// allocated first.
    pub fn compose(intent_id: EffectIntentId) -> Self {
        let message = format!(
            "From: {SENDER}\r\n\
             To: {RECIPIENT}\r\n\
             Subject: {SUBJECT_CANARY}\r\n\
             Date: {FIXTURE_DATE}\r\n\
             Message-ID: <{}@example.test>\r\n\
             MIME-Version: 1.0\r\n\
             Content-Type: text/plain; charset=us-ascii\r\n\
             \r\n\
             {BODY_CANARY}\r\n",
            intent_id.file_stem()
        );
        Self {
            intent_id,
            bytes: message.into_bytes(),
        }
    }

    pub fn intent_id(&self) -> EffectIntentId {
        self.intent_id
    }

    pub fn preview(&self) -> FixturePreview {
        FixturePreview {
            intent_id: self.intent_id,
            header_count: 7,
            is_synthetic: true,
        }
    }

    /// Crate-private: the one place the message leaves the type.
    #[allow(dead_code)]
    pub(crate) fn sealed_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_composition_is_byte_exact_and_rfc_5322_shaped() {
        let id = EffectIntentId::allocate();
        let payload = SealedPayload::compose(id);
        let text = std::str::from_utf8(payload.sealed_bytes()).expect("ASCII");
        assert_eq!(
            text.matches('\n').count(),
            text.matches("\r\n").count(),
            "a bare LF appears in the composed message"
        );
        assert_eq!(text.matches("\r\n\r\n").count(), 1);
        let (headers, body) = text.split_once("\r\n\r\n").unwrap();
        assert_eq!(headers.lines().count(), payload.preview().header_count);
        assert_eq!(body, format!("{BODY_CANARY}\r\n"));
        assert!(headers.contains(&format!("To: {RECIPIENT}")));
    }

    #[test]
    fn the_same_id_composes_identical_bytes() {
        let id = EffectIntentId::allocate();
        assert_eq!(
            SealedPayload::compose(id).sealed_bytes(),
            SealedPayload::compose(id).sealed_bytes()
        );
    }
}

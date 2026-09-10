//! Composing the one message the fixture may produce, and keeping it inside.
//!
//! Three decisions here are worth the words, because each is a place where
//! the obvious implementation is subtly wrong.
//!
//! **The identifier is allocated before anything is composed.** An id derived
//! from content — a hash of the message, a name built from the recipient — is
//! an identifier that leaks its subject to anyone who can see a filename, and
//! it changes when the content does, so it cannot name an intent that has not
//! been composed yet. Allocating first makes the id a name for the *intent*
//! rather than a summary of the payload.
//!
//! **The composition is deterministic, including the date.** A real timestamp
//! would make the bytes differ between runs, so nothing could assert what was
//! composed; it would also record when something happened, which is the kind
//! of fact the evidence chain deliberately omits. The fixture's date is a
//! constant for both reasons.
//!
//! **The payload is private and stays private.** Its fields are not public
//! and there is no accessor that returns its content — only `compose`, which
//! produces the exact bytes, and `preview`, which produces a fixed projection
//! that describes the shape and never the substance. A caller in another
//! crate can cause a message to exist and cannot read it.

use super::corpus::{BODY_CANARY, RECIPIENT, SENDER, SUBJECT_CANARY};
use uuid::Uuid;

/// A fixed date, in RFC 5322 form. Synthetic on purpose: a real one would
/// make the composition nondeterministic and would record when.
pub const FIXTURE_DATE: &str = "Thu, 01 Jan 1970 00:00:00 +0000";

/// An opaque name for one intent. Allocated before composition, so it says
/// nothing about what the message contains.
///
/// No accessor returns the inner value — a caller can hold one, compare it,
/// and hand it back.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct EffectIntentId(Uuid);

impl EffectIntentId {
    /// The only way to make one. Random, and unrelated to any payload,
    /// because there is no payload yet when this is called.
    pub fn allocate() -> Self {
        Self(Uuid::new_v4())
    }

    /// A stable file-safe name. Derived from the id and nothing else, so a
    /// filename on disk cannot be read for a hint about the message.
    pub fn file_stem(&self) -> String {
        self.0.simple().to_string()
    }
}

impl std::fmt::Debug for EffectIntentId {
    /// The id is not secret, and printing it is how a failure is traced. It
    /// is safe to print precisely because it carries nothing.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "EffectIntentId({})", self.0.simple())
    }
}

/// The composed message, held privately.
///
/// Not `pub` fields, no `content()`, no `Deref`, no `AsRef<[u8]>`. The type
/// exists so that a caller can hold the fact of a message without holding the
/// message.
pub struct ProtectedFixturePayload {
    intent_id: EffectIntentId,
    bytes: Vec<u8>,
}

impl std::fmt::Debug for ProtectedFixturePayload {
    /// Value-free apart from the id and the length. The length is included
    /// because it is fixed by construction — every fixture message is the
    /// same message — so it reveals nothing that is not already a constant.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProtectedFixturePayload")
            .field("intent_id", &self.intent_id)
            .field("bytes", &"<protected>")
            .finish()
    }
}

/// What a caller outside this module may know about a payload: its shape,
/// never its substance. Every field is a constant of the fixture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixturePreview {
    pub intent_id: EffectIntentId,
    pub header_count: usize,
    pub is_synthetic: bool,
}

impl ProtectedFixturePayload {
    /// Compose the one message the fixture may produce, for an id that was
    /// allocated first.
    ///
    /// Byte-exact and deterministic: the same id composes the same message
    /// every time, and two different ids compose messages that differ only
    /// where the id appears.
    pub fn compose(intent_id: EffectIntentId) -> Self {
        // CRLF line endings, as RFC 5322 requires. A composer that emitted
        // bare LF would produce something most agents accept and some reject,
        // which is the worst kind of nearly-correct.
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

    /// The shape, for anyone who needs to describe a payload without reading
    /// it.
    pub fn preview(&self) -> FixturePreview {
        FixturePreview {
            intent_id: self.intent_id,
            header_count: 7,
            is_synthetic: true,
        }
    }

    /// The exact bytes. Crate-private: this is the one place the message
    /// leaves the type, and it does so only inside the crate that composed
    /// it.
    ///
    /// Its one caller is the dispatcher, in this crate.
    pub(crate) fn sealed_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The bytes are crate-private, so the test that reads them lives here.
    /// That split is the type's design showing through rather than an
    /// inconvenience: an integration test cannot see the message, which is
    /// the property.
    #[test]
    fn the_composition_is_byte_exact_and_rfc_5322_shaped() {
        let id = EffectIntentId::allocate();
        let payload = ProtectedFixturePayload::compose(id);
        let bytes = payload.sealed_bytes();
        let text = std::str::from_utf8(bytes).expect("the fixture message is ASCII");

        // CRLF throughout. A composer emitting bare LF produces something
        // most agents accept and some reject, which is the worst kind of
        // nearly-correct.
        assert!(!text.contains('\n') || text.contains("\r\n"));
        assert_eq!(
            text.matches('\n').count(),
            text.matches("\r\n").count(),
            "a bare LF appears in the composed message"
        );

        // Exactly one blank line, separating headers from body.
        assert_eq!(text.matches("\r\n\r\n").count(), 1);

        let (headers, body) = text.split_once("\r\n\r\n").expect("a header/body split");
        assert_eq!(
            headers.lines().count(),
            payload.preview().header_count,
            "the preview's header count must match the message"
        );
        assert_eq!(body, format!("{BODY_CANARY}\r\n"));
        assert!(headers.contains(&format!("To: {RECIPIENT}")));
        assert!(headers.contains(&format!("Date: {FIXTURE_DATE}")));
    }

    /// Deterministic: the same id composes the same bytes every time. Without
    /// this nothing could assert what was composed.
    #[test]
    fn the_same_id_composes_identical_bytes() {
        let id = EffectIntentId::allocate();
        let first = ProtectedFixturePayload::compose(id);
        let second = ProtectedFixturePayload::compose(id);
        assert_eq!(first.sealed_bytes(), second.sealed_bytes());
    }

    /// Two ids differ only where the id appears. Everything else about the
    /// fixture's one message is a constant.
    #[test]
    fn two_ids_differ_only_in_the_message_id_header() {
        let first = ProtectedFixturePayload::compose(EffectIntentId::allocate());
        let second = ProtectedFixturePayload::compose(EffectIntentId::allocate());

        let strip = |payload: &ProtectedFixturePayload| {
            std::str::from_utf8(payload.sealed_bytes())
                .unwrap()
                .lines()
                .filter(|line| !line.starts_with("Message-ID:"))
                .collect::<Vec<_>>()
                .join("\r\n")
        };
        assert_ne!(first.sealed_bytes(), second.sealed_bytes());
        assert_eq!(strip(&first), strip(&second));
    }

    /// The date is a constant, not a clock. A real one would make the bytes
    /// differ between runs and would record when something happened.
    #[test]
    fn the_date_is_a_constant_and_not_a_clock() {
        let first = ProtectedFixturePayload::compose(EffectIntentId::allocate());
        std::thread::sleep(std::time::Duration::from_millis(5));
        let second = ProtectedFixturePayload::compose(EffectIntentId::allocate());

        let date_of = |payload: &ProtectedFixturePayload| {
            std::str::from_utf8(payload.sealed_bytes())
                .unwrap()
                .lines()
                .find(|line| line.starts_with("Date:"))
                .unwrap()
                .to_owned()
        };
        assert_eq!(date_of(&first), date_of(&second));
        assert!(date_of(&first).contains(FIXTURE_DATE));
    }
}

//! What a caller outside this module may know about a composed payload.
//!
//! The byte-level tests live inside the crate, because the bytes are
//! crate-private. That split is the property, not an inconvenience: these
//! tests can cause a message to exist and cannot read it, which is exactly
//! the position every other crate is in.

#![cfg(feature = "owner-effect-fixture")]

use sovereign_authority::broker::exact_fixture::{
    EffectIntentId, ProtectedFixturePayload, FIXTURE_DATE,
};

/// An id derived from content leaks its subject to anyone who can see a
/// filename, and cannot name an intent that has not been composed yet.
/// Allocating first makes it a name for the intent rather than a summary of
/// the payload.
#[test]
fn an_intent_id_is_allocated_before_anything_is_composed() {
    // No payload exists at this point, and the id already does.
    let id = EffectIntentId::allocate();
    let stem = id.file_stem();

    let payload = ProtectedFixturePayload::compose(id);
    assert_eq!(payload.intent_id(), id, "composing changed the id");
    assert_eq!(id.file_stem(), stem, "composing changed the id's file name");
}

/// Two allocations are distinct, so one intent cannot be mistaken for
/// another, and neither is predictable from the other.
#[test]
fn allocated_ids_are_distinct() {
    let mut seen = std::collections::HashSet::new();
    for _ in 0..1000 {
        assert!(
            seen.insert(EffectIntentId::allocate().file_stem()),
            "an allocation repeated"
        );
    }
}

/// A filename must not be readable for a hint about the message.
#[test]
fn the_file_stem_is_derived_from_the_id_and_nothing_else() {
    let id = EffectIntentId::allocate();
    let stem = id.file_stem();

    assert_eq!(stem.len(), 32, "expected a bare uuid, got {stem}");
    assert!(
        stem.chars().all(|c| c.is_ascii_hexdigit()),
        "the stem is not opaque: {stem}"
    );
    // Nothing from the corpus appears in it.
    for fragment in ["fixture-recipient", "example", "CANARY", "SFO"] {
        assert!(
            !stem.contains(fragment),
            "the file name carries {fragment:?}"
        );
    }
}

/// The point of the type. There is no accessor that returns the content, so a
/// caller in another crate can cause a message to exist and cannot read it.
#[test]
fn the_payload_has_no_public_way_to_read_its_content() {
    let source = include_str!("../src/broker/exact_fixture.rs");
    let code: String = source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");

    // The one accessor is crate-private, and stays that way.
    assert!(
        code.contains("pub(crate) fn sealed_bytes"),
        "sealed_bytes must stay crate-private"
    );
    for escape in [
        "pub fn bytes",
        "pub fn content",
        "pub fn sealed_bytes",
        "impl AsRef<[u8]> for ProtectedFixturePayload",
        "impl std::ops::Deref for ProtectedFixturePayload",
    ] {
        assert!(
            !code.contains(escape),
            "the payload exposes {escape:?}, so another crate can read the message"
        );
    }
    // And the fields are private, so the struct literal is unavailable too.
    assert!(
        code.contains("pub struct ProtectedFixturePayload {\n    intent_id: EffectIntentId,\n    bytes: Vec<u8>,\n}"),
        "the payload's fields must stay private"
    );
}

/// The preview describes shape, never substance — every field is a constant
/// of the fixture, so it answers nothing about this particular message.
#[test]
fn the_preview_carries_only_fixture_constants() {
    let first = ProtectedFixturePayload::compose(EffectIntentId::allocate());
    let second = ProtectedFixturePayload::compose(EffectIntentId::allocate());

    let a = first.preview();
    let b = second.preview();
    assert_ne!(a.intent_id, b.intent_id);
    assert_eq!(a.header_count, b.header_count);
    assert_eq!(a.is_synthetic, b.is_synthetic);
    assert!(a.is_synthetic);

    // Rendered, it shows the id and two constants and nothing else.
    let rendered = format!("{a:?}");
    for fragment in ["fixture-recipient", "CANARY", FIXTURE_DATE] {
        assert!(
            !rendered.contains(fragment),
            "the preview leaked {fragment:?}: {rendered}"
        );
    }
}

/// The payload's own rendering protects its bytes while staying useful for
/// tracing a failure.
#[test]
fn the_payload_never_prints_its_message() {
    let id = EffectIntentId::allocate();
    let payload = ProtectedFixturePayload::compose(id);
    let rendered = format!("{payload:?}");

    assert!(rendered.contains("<protected>"), "{rendered}");
    assert!(
        rendered.contains(&id.file_stem()),
        "the id should be traceable: {rendered}"
    );
    for fragment in ["fixture-recipient", "CANARY", "From:", "Subject:"] {
        assert!(
            !rendered.contains(fragment),
            "the rendering leaked {fragment:?}: {rendered}"
        );
    }
}

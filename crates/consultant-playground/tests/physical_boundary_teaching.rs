//! S1-G04 teaching/search grammar boundary tests.

use std::path::Path;

#[path = "support/boundary.rs"]
mod boundary;
#[path = "support/rust_lexer.rs"]
mod rust_lexer;

use boundary::{
    source_boundary, SourceBoundaryKind, CATALOG_GUIDANCE_PRODUCTION, CATALOG_PRODUCTION,
    EXPECTED_DOMAIN_PRODUCTION, READ_MODEL_DOMAIN_PRODUCTION, TEACHING_DOMAIN_PRODUCTION,
};
use rust_lexer::{RustLexer, RustToken};

fn wrapped(production: &str) -> String {
    format!("{production}\n#[cfg(test)] mod tests {{}}")
}

fn assert_rejected(path: &Path, source: &str, expected: SourceBoundaryKind) {
    let error = source_boundary(path, source).expect_err("mutation must be rejected");
    assert_eq!(error.kind, expected, "unexpected rejection: {error}");
}

#[test]
fn teaching_gate_accepts_old_and_complete_next_shapes() {
    for production in [
        EXPECTED_DOMAIN_PRODUCTION,
        READ_MODEL_DOMAIN_PRODUCTION,
        &format!("{READ_MODEL_DOMAIN_PRODUCTION}{TEACHING_DOMAIN_PRODUCTION}"),
    ] {
        source_boundary(Path::new("domain.rs"), &wrapped(production)).expect("domain shape");
    }
    assert_rejected(
        Path::new("domain.rs"),
        &wrapped(TEACHING_DOMAIN_PRODUCTION),
        SourceBoundaryKind::DomainProductionShape,
    );
    for production in [CATALOG_PRODUCTION, CATALOG_GUIDANCE_PRODUCTION] {
        source_boundary(Path::new("catalog.rs"), &wrapped(production)).expect("catalog shape");
    }
    let mut varied = wrapped(CATALOG_GUIDANCE_PRODUCTION);
    let catalog_tokens =
        RustLexer::lex(CATALOG_GUIDANCE_PRODUCTION).expect("catalog fixture lexes");
    let mut changed_slots = 0;
    for window in catalog_tokens.windows(3) {
        if let [RustToken::Ident(field), RustToken::Punct(':'), RustToken::Literal(value)] = window
        {
            if field == "en" || field == "zh" {
                assert!(varied.contains(&format!("{field}: {value}")));
                changed_slots += 1;
                varied = varied.replacen(
                    &format!("{field}: {value}"),
                    &format!("{field}: \"changed\""),
                    1,
                );
            }
        }
    }
    assert_eq!(changed_slots, 64);
    assert_ne!(varied, wrapped(CATALOG_GUIDANCE_PRODUCTION));
    source_boundary(Path::new("catalog.rs"), &varied).expect("language text is data");
}

#[test]
fn teaching_gate_rejects_guidance_and_search_mutations() {
    let complete = format!("{READ_MODEL_DOMAIN_PRODUCTION}{TEACHING_DOMAIN_PRODUCTION}");
    let source = wrapped(&complete);
    for (name, target, replacement) in [
        ("parameter", "teaching_read_model(&self)", "teaching_read_model(&self, query: &str)"),
        ("mutable receiver", "teaching_read_model(&self)", "teaching_read_model(&mut self)"),
        ("external DTO", "teaching_read_model(&self)", "teaching_read_model(&self, input: ExternalDto)"),
        (
            "graph write",
            "let guidance",
            "self.graph.offer.price_usd_cents = 1; let guidance",
        ),
        (
            "priority",
            "price_usd_cents != 350_000",
            "price_usd_cents == 350_000",
        ),
        (
            "fake call done",
            "guidance_example_changes_complete",
            "client_call_complete",
        ),
        (
            "fifth action",
            "    Reset,",
            "    Reset,\n    ScheduleClientCall,",
        ),
        ("search hit", "fact_key: \"reporting_clarity_sprint\"", "fact_key: \"other_fact\""),
        ("search count", "[ReportingHit; 2]", "[ReportingHit; 3]"),
        ("free query", "reporting_search_query", "query"),
        ("Deserialize", "serde::Serialize)]\nstruct ReportingHit", "serde::Serialize, serde::Deserialize)]\nstruct ReportingHit"),
        (
            "reverse conversion",
            "#[cfg(test)] mod tests {}",
            "impl From<PlaygroundTeachingReadModel> for PlaygroundSession { fn from(value: PlaygroundTeachingReadModel) -> Self { let _ = value; Self::new() } } #[cfg(test)] mod tests {}",
        ),
        (
            "IO",
            "let guidance =",
            "let _ = std::fs::write(\"x\", \"y\"); let guidance =",
        ),
        ("env", "let guidance =", "let _ = env!(\"SECRET\"); let guidance ="),
        (
            "process",
            "let guidance =",
            "let _ = std::process::Command::new(\"sh\"); let guidance =",
        ),
        (
            "unsafe",
            "let guidance =",
            "let _ = unsafe { 1 }; let guidance =",
        ),
        ("macro", "let guidance =", "let _ = concat!(\"x\"); let guidance ="),
        (
            "function",
            "#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]\nstruct ReportingHit",
            "fn leak() {} #[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]\nstruct ReportingHit",
        ),
    ] {
        assert!(source.contains(target), "missing mutation target {name}");
        let changed = source.replacen(target, replacement, 1);
        assert_ne!(changed, source, "mutation {name} must differ");
        assert_rejected(
            Path::new("domain.rs"),
            &changed,
            SourceBoundaryKind::DomainProductionShape,
        );
    }
}

#[test]
fn teaching_catalog_gate_rejects_unknown_keys_and_structure() {
    let source = wrapped(CATALOG_GUIDANCE_PRODUCTION);
    for (name, target, replacement) in [
        (
            "missing key",
            "    CatalogEntry { key: \"page_title\", en: \"Consultant Playground\", zh: \"顾问练习场\" },\n",
            "",
        ),
        (
            "renamed key",
            "key: \"reporting_search_query\"",
            "key: \"other\"",
        ),
        ("length", "[CatalogEntry; 32]", "[CatalogEntry; 33]"),
        (
            "expression",
            "en: \"Consultant Playground\"",
            "en: concat!(\"a\", \"b\")",
        ),
        ("executable", "];", "]; fn leak() {}"),
    ] {
        assert!(source.contains(target), "missing catalog target {name}");
        let changed = source.replacen(target, replacement, 1);
        assert_ne!(changed, source, "mutation {name} must differ");
        assert_rejected(
            Path::new("catalog.rs"),
            &changed,
            SourceBoundaryKind::CatalogProductionShape,
        );
    }
    let entries: Vec<_> = CATALOG_GUIDANCE_PRODUCTION
        .lines()
        .filter(|line| line.contains("CatalogEntry { key:"))
        .collect();
    assert_eq!(entries.len(), 32);
    let swapped = source.replacen(
        &format!("{}\n{}", entries[30], entries[31]),
        &format!("{}\n{}", entries[31], entries[30]),
        1,
    );
    assert_ne!(swapped, source);
    assert_rejected(
        Path::new("catalog.rs"),
        &swapped,
        SourceBoundaryKind::CatalogProductionShape,
    );
    let extra = source.replacen("];", &format!("{}\n];", entries[0]), 1);
    assert_ne!(extra, source);
    assert_rejected(
        Path::new("catalog.rs"),
        &extra,
        SourceBoundaryKind::CatalogProductionShape,
    );
    let tokens = RustLexer::lex(CATALOG_GUIDANCE_PRODUCTION).expect("fixture lexes");
    assert_eq!(
        tokens
            .iter()
            .filter(|token| matches!(token, RustToken::Ident(key) if key == "key"))
            .count(),
        33
    );
}

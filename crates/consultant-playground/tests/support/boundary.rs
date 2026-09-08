//! Exact-token-shape validation for `src/lib.rs` and `src/domain.rs`: the
//! production source closure must match a pinned grammar so a rename,
//! injected side effect, or path escape fails loudly instead of silently
//! widening Task 1's boundary.

use std::path::Path;

use crate::rust_lexer::{RustLexer, RustToken};

const EXPECTED_LIB_SHAPE: &str = "#[cfg_attr(not(test), allow(dead_code))]\nmod domain;";

const EXPECTED_DOMAIN_PRODUCTION: &str = r####"
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SemanticKey {
    ReportingClaritySprint,
    WeeklyReportingTakesSixHours,
    FinanceMustApprove,
    ThirtyMinuteScopingCall,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RelationshipStage {
    Lead,
    Customer,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Company {
    name: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Offer {
    name_key: SemanticKey,
    price_usd_cents: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Relationship {
    organization: &'static str,
    contact_name: &'static str,
    contact_email: &'static str,
    stage: RelationshipStage,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Discovery {
    problem_key: SemanticKey,
    budget_min_usd_cents: u32,
    budget_max_usd_cents: u32,
    constraint_key: SemanticKey,
    next_step_key: SemanticKey,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ConsultantPlaygroundGraph {
    company: Company,
    offer: Offer,
    relationship: Relationship,
    discovery: Discovery,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PlaygroundSession {
    graph: ConsultantPlaygroundGraph,
}

impl PlaygroundSession {
    fn new() -> Self {
        Self {
            graph: ConsultantPlaygroundGraph {
                company: Company {
                    name: "North Star Operations",
                },
                offer: Offer {
                    name_key: SemanticKey::ReportingClaritySprint,
                    price_usd_cents: 250_000,
                },
                relationship: Relationship {
                    organization: "Acme Ltd",
                    contact_name: "Alex Chen",
                    contact_email: "alex.chen@example.test",
                    stage: RelationshipStage::Lead,
                },
                discovery: Discovery {
                    problem_key: SemanticKey::WeeklyReportingTakesSixHours,
                    budget_min_usd_cents: 300_000,
                    budget_max_usd_cents: 500_000,
                    constraint_key: SemanticKey::FinanceMustApprove,
                    next_step_key: SemanticKey::ThirtyMinuteScopingCall,
                },
            },
        }
    }
}
"####;

const DOMAIN_TEST_HEADER: &str = "#[cfg(test)] mod tests {";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SourceBoundaryKind {
    Lexical,
    PathAttribute,
    LibCfgAttrShape,
    LibItemShape,
    DomainTestModuleShape,
    DomainProductionShape,
    UnexpectedSourceFile,
}

#[derive(Debug)]
pub(crate) struct SourceBoundaryError {
    pub(crate) kind: SourceBoundaryKind,
    detail: String,
}

impl SourceBoundaryError {
    fn new(kind: SourceBoundaryKind, detail: impl Into<String>) -> Self {
        Self {
            kind,
            detail: detail.into(),
        }
    }
}

impl std::fmt::Display for SourceBoundaryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{:?}: {}", self.kind, self.detail)
    }
}

pub(crate) fn source_boundary(path: &Path, source: &str) -> Result<(), SourceBoundaryError> {
    let tokens = RustLexer::lex(source)
        .map_err(|detail| SourceBoundaryError::new(SourceBoundaryKind::Lexical, detail))?;
    match path.file_name().and_then(|name| name.to_str()) {
        Some("lib.rs") => validate_lib_shape(&tokens),
        Some("domain.rs") => validate_domain_shape(&tokens),
        Some(name) => Err(SourceBoundaryError::new(
            SourceBoundaryKind::UnexpectedSourceFile,
            format!("unexpected Task 1 source file `{name}`"),
        )),
        None => Err(SourceBoundaryError::new(
            SourceBoundaryKind::UnexpectedSourceFile,
            "source path has no UTF-8 file name",
        )),
    }
}

fn validate_lib_shape(tokens: &[RustToken]) -> Result<(), SourceBoundaryError> {
    reject_path_attributes(tokens)?;
    let expected = RustLexer::lex(EXPECTED_LIB_SHAPE).expect("expected lib shape must lex");
    if tokens == expected {
        return Ok(());
    }
    let module = RustLexer::lex("mod domain;").expect("expected module shape must lex");
    let kind = if tokens.ends_with(&module) {
        SourceBoundaryKind::LibCfgAttrShape
    } else {
        SourceBoundaryKind::LibItemShape
    };
    Err(SourceBoundaryError::new(
        kind,
        token_mismatch("lib.rs", &expected, tokens),
    ))
}

fn validate_domain_shape(tokens: &[RustToken]) -> Result<(), SourceBoundaryError> {
    let production = strip_exact_test_module(tokens)?;
    reject_path_attributes(production)?;
    let expected =
        RustLexer::lex(EXPECTED_DOMAIN_PRODUCTION).expect("expected domain shape must lex");
    if production == expected {
        Ok(())
    } else {
        Err(SourceBoundaryError::new(
            SourceBoundaryKind::DomainProductionShape,
            token_mismatch("domain.rs production", &expected, production),
        ))
    }
}

fn strip_exact_test_module(tokens: &[RustToken]) -> Result<&[RustToken], SourceBoundaryError> {
    let header = RustLexer::lex(DOMAIN_TEST_HEADER).expect("expected test header must lex");
    let mut brace_depth = 0_usize;
    let mut start = None;
    for index in 0..tokens.len() {
        if brace_depth == 0 && tokens[index..].starts_with(&header) {
            if start.replace(index).is_some() {
                return Err(SourceBoundaryError::new(
                    SourceBoundaryKind::DomainTestModuleShape,
                    "domain has more than one exact test module",
                ));
            }
            break;
        }
        match tokens[index] {
            RustToken::Punct('{') => brace_depth += 1,
            RustToken::Punct('}') => {
                brace_depth = brace_depth.checked_sub(1).ok_or_else(|| {
                    SourceBoundaryError::new(
                        SourceBoundaryKind::DomainTestModuleShape,
                        "unbalanced closing brace before test module",
                    )
                })?;
            }
            _ => {}
        }
    }
    let start = start.ok_or_else(|| {
        SourceBoundaryError::new(
            SourceBoundaryKind::DomainTestModuleShape,
            "domain is missing exact `#[cfg(test)] mod tests { ... }` wrapper",
        )
    })?;
    let opening = start + header.len() - 1;
    let mut depth = 0_usize;
    let mut end = None;
    for (offset, token) in tokens[opening..].iter().enumerate() {
        match token {
            RustToken::Punct('{') => depth += 1,
            RustToken::Punct('}') => {
                depth = depth.checked_sub(1).ok_or_else(|| {
                    SourceBoundaryError::new(
                        SourceBoundaryKind::DomainTestModuleShape,
                        "unbalanced test-module closing brace",
                    )
                })?;
                if depth == 0 {
                    end = Some(opening + offset + 1);
                    break;
                }
            }
            _ => {}
        }
    }
    if end != Some(tokens.len()) {
        return Err(SourceBoundaryError::new(
            SourceBoundaryKind::DomainTestModuleShape,
            "test module is unbalanced or is not the terminal top-level item",
        ));
    }
    Ok(&tokens[..start])
}

fn token_mismatch(label: &str, expected: &[RustToken], actual: &[RustToken]) -> String {
    let offset = expected
        .iter()
        .zip(actual)
        .position(|(expected, actual)| expected != actual)
        .unwrap_or_else(|| expected.len().min(actual.len()));
    format!(
        "{label} differs at token {offset}: expected {:?}, found {:?}",
        expected.get(offset),
        actual.get(offset)
    )
}

fn reject_path_attributes(tokens: &[RustToken]) -> Result<(), SourceBoundaryError> {
    let mut index = 0;
    while index < tokens.len() {
        if tokens[index] != RustToken::Punct('#') {
            index += 1;
            continue;
        }
        let mut cursor = index + 1;
        if tokens.get(cursor) == Some(&RustToken::Punct('!')) {
            cursor += 1;
        }
        if tokens.get(cursor) != Some(&RustToken::Punct('[')) {
            index += 1;
            continue;
        }
        let mut depth = 1;
        cursor += 1;
        while cursor < tokens.len() && depth > 0 {
            match &tokens[cursor] {
                RustToken::Punct('[') => depth += 1,
                RustToken::Punct(']') => depth -= 1,
                RustToken::Ident(identifier) if depth > 0 && identifier == "path" => {
                    return Err(SourceBoundaryError::new(
                        SourceBoundaryKind::PathAttribute,
                        "Rust path attribute can escape the source closure",
                    ));
                }
                _ => {}
            }
            cursor += 1;
        }
        if depth != 0 {
            return Err(SourceBoundaryError::new(
                SourceBoundaryKind::Lexical,
                "unterminated Rust attribute",
            ));
        }
        index = cursor;
    }
    Ok(())
}

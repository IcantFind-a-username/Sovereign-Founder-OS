//! Exact-token-shape validation for the accepted playground source stages: the
//! production source closure must match a pinned grammar so a rename,
//! injected side effect, or path escape fails loudly instead of silently
//! widening Task 1's boundary.

use std::path::Path;

use crate::rust_lexer::{RustLexer, RustToken};

pub(crate) const EXPECTED_LIB_SHAPE: &str = "#[cfg_attr(not(test), allow(dead_code))]\nmod domain;";
pub(crate) const EXPECTED_LIB_CATALOG_SHAPE: &str = "#[cfg_attr(not(test), allow(dead_code))]\nmod domain;\n#[cfg_attr(not(test), allow(dead_code))]\nmod catalog;";

pub(crate) const EXPECTED_LIB_HTTP_SHAPE: &str = "#[cfg_attr(not(test), allow(dead_code))]\nmod domain;\n// Preserve accepted declaration order.\n#[cfg_attr(not(test), allow(dead_code))]\nmod catalog;\n// The pure handler follows domain and catalog.\n#[cfg_attr(not(test), allow(dead_code))]\nmod http;";
pub(crate) const EXPECTED_LIB_ASSETS_SHAPE: &str = "#[cfg_attr(not(test), allow(dead_code))]\nmod domain;\n// Preserve accepted declaration order.\n#[cfg_attr(not(test), allow(dead_code))]\nmod catalog;\n// The pure handler follows domain and catalog.\n#[cfg_attr(not(test), allow(dead_code))]\nmod http;\n// Compile-time assets follow the pure handler.\n#[cfg_attr(not(test), allow(dead_code))]\nmod assets;\n";
pub(crate) const EXPECTED_LIB_SERVER_SHAPE: &str = r#"#[cfg_attr(not(test), allow(dead_code))]
mod domain;
#[cfg_attr(not(test), allow(dead_code))]
mod catalog;
#[cfg_attr(not(test), allow(dead_code))]
mod http;
#[cfg_attr(not(test), allow(dead_code))]
mod assets;
mod server;
pub fn run(port: u16) -> std::io::Result<()> {
    server::run(port)
}
"#;
pub(crate) const SERVER_PRODUCTION: &str = include_str!("fixtures/server-production.rs.txt");
pub(crate) const ASSETS_PRODUCTION: &str = include_str!("fixtures/assets-production.rs.txt");
pub(crate) const HTTP_PRODUCTION: &str = include_str!("fixtures/http-production.rs.txt");

pub(crate) const EXPECTED_DOMAIN_PRODUCTION: &str = r####"
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

pub(crate) const ACTION_DOMAIN_ADDITIONS: &str =
    include_str!("fixtures/action-domain-additions.rs.txt");
pub(crate) const READ_MODEL_DOMAIN_PRODUCTION: &str =
    include_str!("fixtures/read-model-domain.rs.txt");
pub(crate) const TEACHING_DOMAIN_PRODUCTION: &str =
    include_str!("fixtures/teaching-domain-additions.rs.txt");
pub(crate) const CATALOG_PRODUCTION: &str = include_str!("fixtures/catalog-production.rs.txt");
pub(crate) const CATALOG_GUIDANCE_PRODUCTION: &str =
    include_str!("fixtures/catalog-guidance-production.rs.txt");

const DOMAIN_TEST_HEADER: &str = "#[cfg(test)] mod tests {";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SourceBoundaryKind {
    Lexical,
    PathAttribute,
    LibCfgAttrShape,
    LibItemShape,
    DomainTestModuleShape,
    DomainProductionShape,
    CatalogProductionShape,
    CatalogTestModuleShape,
    HttpProductionShape,
    HttpTestModuleShape,
    AssetsProductionShape,
    AssetsTestModuleShape,
    ServerProductionShape,
    ServerTestModuleShape,
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
        Some("catalog.rs") => validate_catalog_shape(&tokens),
        Some("http.rs") => validate_http_shape(&tokens),
        Some("assets.rs") => validate_assets_shape(&tokens),
        Some("server.rs") => validate_server_shape(&tokens),
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

fn validate_server_shape(tokens: &[RustToken]) -> Result<(), SourceBoundaryError> {
    let production =
        strip_exact_test_module(tokens, "server", SourceBoundaryKind::ServerTestModuleShape)?;
    reject_path_attributes(production)?;
    let expected = RustLexer::lex(SERVER_PRODUCTION).expect("complete server fixture must lex");
    if production == expected {
        Ok(())
    } else {
        Err(SourceBoundaryError::new(
            SourceBoundaryKind::ServerProductionShape,
            token_mismatch("server.rs production", &expected, production),
        ))
    }
}

fn validate_assets_shape(tokens: &[RustToken]) -> Result<(), SourceBoundaryError> {
    let production =
        strip_exact_test_module(tokens, "assets", SourceBoundaryKind::AssetsTestModuleShape)?;
    reject_path_attributes(production)?;
    let expected = RustLexer::lex(ASSETS_PRODUCTION).expect("complete asset fixture must lex");
    if production == expected {
        Ok(())
    } else {
        Err(SourceBoundaryError::new(
            SourceBoundaryKind::AssetsProductionShape,
            token_mismatch("assets.rs production", &expected, production),
        ))
    }
}

fn validate_http_shape(tokens: &[RustToken]) -> Result<(), SourceBoundaryError> {
    let production =
        strip_exact_test_module(tokens, "http", SourceBoundaryKind::HttpTestModuleShape)?;
    reject_path_attributes(production)?;
    let expected = RustLexer::lex(HTTP_PRODUCTION).expect("complete HTTP fixture must lex");
    if production == expected {
        Ok(())
    } else {
        Err(SourceBoundaryError::new(
            SourceBoundaryKind::HttpProductionShape,
            token_mismatch("http.rs production", &expected, production),
        ))
    }
}

fn validate_catalog_shape(tokens: &[RustToken]) -> Result<(), SourceBoundaryError> {
    let production = strip_exact_test_module(
        tokens,
        "catalog",
        SourceBoundaryKind::CatalogTestModuleShape,
    )?;
    reject_path_attributes(production)?;
    let old = RustLexer::lex(CATALOG_PRODUCTION).expect("catalog fixture must lex");
    let new = RustLexer::lex(CATALOG_GUIDANCE_PRODUCTION).expect("catalog fixture must lex");
    if production.len() == old.len() {
        return validate_catalog_tokens(production, &old, 54);
    }
    validate_catalog_tokens(production, &new, 64)
}

fn validate_catalog_tokens(
    production: &[RustToken],
    expected: &[RustToken],
    text_count: usize,
) -> Result<(), SourceBoundaryError> {
    let text_positions: Vec<_> = expected.windows(3).enumerate().filter_map(|(index, tokens)| {
        matches!(&tokens,
            &[RustToken::Ident(field), RustToken::Punct(':'), RustToken::Literal(value)]
                if (field == "en" || field == "zh") && value.starts_with('"') && value.ends_with('"')
        ).then_some(index + 2)
    }).collect();
    assert_eq!(
        text_positions.len(),
        text_count,
        "frozen catalog text positions"
    );
    if production.len() == expected.len()
        && production.iter().enumerate().all(|(index, token)| {
            if text_positions.contains(&index) {
                matches!(token, RustToken::Literal(value) if value.starts_with('"') && value.ends_with('"'))
            } else {
                token == &expected[index]
            }
        })
    {
        Ok(())
    } else {
        Err(SourceBoundaryError::new(
            SourceBoundaryKind::CatalogProductionShape,
            token_mismatch("catalog.rs", expected, production),
        ))
    }
}

fn validate_lib_shape(tokens: &[RustToken]) -> Result<(), SourceBoundaryError> {
    reject_path_attributes(tokens)?;
    let expected = RustLexer::lex(EXPECTED_LIB_SHAPE).expect("expected lib shape must lex");
    let catalog = RustLexer::lex(EXPECTED_LIB_CATALOG_SHAPE).expect("catalog lib shape must lex");
    let http = RustLexer::lex(EXPECTED_LIB_HTTP_SHAPE).expect("HTTP lib shape must lex");
    let assets = RustLexer::lex(EXPECTED_LIB_ASSETS_SHAPE).expect("asset lib shape must lex");
    let server = RustLexer::lex(EXPECTED_LIB_SERVER_SHAPE).expect("server lib shape must lex");
    if tokens == expected
        || tokens == catalog
        || tokens == http
        || tokens == assets
        || tokens == server
    {
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
    let production =
        strip_exact_test_module(tokens, "domain", SourceBoundaryKind::DomainTestModuleShape)?;
    reject_path_attributes(production)?;
    let expected =
        RustLexer::lex(EXPECTED_DOMAIN_PRODUCTION).expect("expected domain shape must lex");
    let mut extended = expected.clone();
    extended.extend(RustLexer::lex(ACTION_DOMAIN_ADDITIONS).expect("action additions must lex"));
    let read_model = RustLexer::lex(READ_MODEL_DOMAIN_PRODUCTION)
        .expect("complete read model production fixture must lex");
    let mut teaching = RustLexer::lex(READ_MODEL_DOMAIN_PRODUCTION)
        .expect("complete read model production fixture must lex");
    teaching.extend(
        RustLexer::lex(TEACHING_DOMAIN_PRODUCTION)
            .expect("complete teaching production fixture must lex"),
    );
    if production == expected
        || production == extended
        || production == read_model
        || production == teaching
    {
        Ok(())
    } else {
        Err(SourceBoundaryError::new(
            SourceBoundaryKind::DomainProductionShape,
            token_mismatch("domain.rs production", &expected, production),
        ))
    }
}

fn strip_exact_test_module<'a>(
    tokens: &'a [RustToken],
    label: &str,
    kind: SourceBoundaryKind,
) -> Result<&'a [RustToken], SourceBoundaryError> {
    let header = RustLexer::lex(DOMAIN_TEST_HEADER).expect("expected test header must lex");
    let mut brace_depth = 0_usize;
    let mut start = None;
    for index in 0..tokens.len() {
        if brace_depth == 0 && tokens[index..].starts_with(&header) {
            if start.replace(index).is_some() {
                return Err(SourceBoundaryError::new(
                    kind,
                    format!("{label} has more than one exact test module"),
                ));
            }
            break;
        }
        match tokens[index] {
            RustToken::Punct('{') => brace_depth += 1,
            RustToken::Punct('}') => {
                brace_depth = brace_depth.checked_sub(1).ok_or_else(|| {
                    SourceBoundaryError::new(kind, "unbalanced closing brace before test module")
                })?;
            }
            _ => {}
        }
    }
    let start = start.ok_or_else(|| {
        SourceBoundaryError::new(
            kind,
            format!("{label} is missing exact `#[cfg(test)] mod tests {{ ... }}` wrapper"),
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
                    SourceBoundaryError::new(kind, "unbalanced test-module closing brace")
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
            kind,
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

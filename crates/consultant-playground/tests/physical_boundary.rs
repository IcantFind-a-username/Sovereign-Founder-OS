use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static FIXTURE_SEQUENCE: AtomicUsize = AtomicUsize::new(0);

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn task_one_manifest_is_publish_false_and_dependency_free() {
    manifest_boundary(
        &crate_root().join("Cargo.toml"),
        "sovereign-consultant-playground",
    )
    .expect("Task 1 manifest must be unpublished and dependency-free");
}

#[test]
fn task_one_production_source_closure_has_no_persistence_or_product_surface() {
    let source_root = crate_root().join("src");
    let expected = BTreeSet::from([source_root.join("domain.rs"), source_root.join("lib.rs")]);
    let actual = production_sources(&source_root).expect("source root must be a real directory");
    assert_eq!(actual, expected, "Task 1 source closure changed");

    for path in actual {
        let source = fs::read_to_string(&path).expect("production source must be readable");
        source_boundary(&path, &source)
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    }
}

#[cfg(any(unix, windows))]
#[test]
fn symlinked_source_root_is_rejected_before_traversal_or_manifest_canonicalization() {
    let fixture = match SymlinkedSourceFixture::new() {
        Ok(fixture) => fixture,
        #[cfg(windows)]
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            eprintln!("directory symlinks are unavailable without Windows developer privilege");
            return;
        }
        Err(error) => panic!("source-root fixture must be created: {error}"),
    };

    assert_eq!(
        production_sources(&fixture.source_link),
        Err(SourceRootError::RootSymlink),
        "source inventory must reject the symlink root itself"
    );
    let manifest_error = manifest_boundary(&fixture.base.manifest, "boundary-fixture")
        .expect_err("manifest boundary must reject its symlinked source root");
    assert!(manifest_error.contains("RootSymlink"), "{manifest_error}");
}

#[test]
fn syntax_checker_rejects_grouped_std_alias_and_path_attribute_escapes() {
    let mutations = [
        (
            "grouped std import",
            Path::new("domain.rs"),
            domain_with_extra("use std::{fs};"),
            SourceBoundaryKind::DomainProductionShape,
        ),
        (
            "grouped std alias",
            Path::new("domain.rs"),
            domain_with_extra("use std::{fs as disk};"),
            SourceBoundaryKind::DomainProductionShape,
        ),
        (
            "path attribute",
            Path::new("lib.rs"),
            "#[path=\"../outside.rs\"] mod domain;".into(),
            SourceBoundaryKind::PathAttribute,
        ),
    ];
    for (name, path, source, expected) in mutations {
        assert_source_rejection(name, path, &source, expected);
    }
}

#[test]
fn exact_shape_mutations_are_rejected() {
    let mutations = [
        (
            "cfg-gated domain",
            Path::new("lib.rs"),
            "#[cfg(test)] mod domain;".to_string(),
            SourceBoundaryKind::LibCfgAttrShape,
        ),
        (
            "removed cfg_attr",
            Path::new("lib.rs"),
            "mod domain;".into(),
            SourceBoundaryKind::LibCfgAttrShape,
        ),
        (
            "changed cfg_attr",
            Path::new("lib.rs"),
            "#[cfg_attr(test, allow(dead_code))] mod domain;".into(),
            SourceBoundaryKind::LibCfgAttrShape,
        ),
        (
            "path attribute",
            Path::new("lib.rs"),
            "#[path=\"../outside.rs\"] mod domain;".into(),
            SourceBoundaryKind::PathAttribute,
        ),
        (
            "unknown production attribute",
            Path::new("domain.rs"),
            domain_with_extra("#[allow(unused)] const EXTRA: u8 = 1;"),
            SourceBoundaryKind::DomainProductionShape,
        ),
        (
            "doc-comment attribute",
            Path::new("domain.rs"),
            domain_with_extra("/// hidden attribute"),
            SourceBoundaryKind::Lexical,
        ),
        (
            "env macro",
            Path::new("domain.rs"),
            domain_with_extra("const LEAK: &str = env!(\"SECRET\");"),
            SourceBoundaryKind::DomainProductionShape,
        ),
        (
            "format macro",
            Path::new("domain.rs"),
            domain_with_extra("fn leak() { let _ = format!(\"secret\"); }"),
            SourceBoundaryKind::DomainProductionShape,
        ),
        (
            "owned allocation",
            Path::new("domain.rs"),
            domain_with_extra("fn leak() { let _ = \"secret\".to_owned(); }"),
            SourceBoundaryKind::DomainProductionShape,
        ),
        (
            "unsafe extern",
            Path::new("domain.rs"),
            domain_with_extra(
                "unsafe extern \"C\" { fn open(path: *const u8, flags: i32) -> i32; }",
            ),
            SourceBoundaryKind::DomainProductionShape,
        ),
        (
            "inline assembly",
            Path::new("domain.rs"),
            domain_with_extra("fn leak() { unsafe { core::arch::asm!(\"nop\"); } }"),
            SourceBoundaryKind::DomainProductionShape,
        ),
    ];

    for (name, path, source, expected) in mutations {
        assert_source_rejection(name, path, &source, expected);
    }
}

#[test]
fn domain_test_module_wrapper_must_be_exact_and_terminal() {
    let source = fs::read_to_string(crate_root().join("src/domain.rs"))
        .expect("domain source must be readable");
    let changed_cfg = source.replacen("#[cfg(test)]\nmod tests", "#[cfg(any())]\nmod tests", 1);
    let removed = source
        .split_once("#[cfg(test)]\nmod tests")
        .expect("domain must contain test module")
        .0
        .to_string();

    assert_source_rejection(
        "changed test cfg",
        Path::new("domain.rs"),
        &changed_cfg,
        SourceBoundaryKind::DomainTestModuleShape,
    );
    assert_source_rejection(
        "removed test module",
        Path::new("domain.rs"),
        &removed,
        SourceBoundaryKind::DomainTestModuleShape,
    );
}

fn domain_with_extra(extra: &str) -> String {
    let source = fs::read_to_string(crate_root().join("src/domain.rs"))
        .expect("domain source must be readable");
    let marker = "#[cfg(test)]\nmod tests";
    let offset = source
        .find(marker)
        .expect("domain source must contain its unit-test module");
    format!("{}\n{extra}\n{}", &source[..offset], &source[offset..])
}

fn assert_source_rejection(name: &str, path: &Path, source: &str, expected: SourceBoundaryKind) {
    let error = match source_boundary(path, source) {
        Ok(()) => panic!("source mutation `{name}` was accepted"),
        Err(error) => error,
    };
    assert_eq!(
        error.kind, expected,
        "source mutation `{name}` hit the wrong guard: {error}"
    );
}

#[test]
fn cargo_metadata_rejects_normal_dev_build_target_and_dotted_dependencies() {
    let declarations = [
        "[dependencies]\nforbidden = { path = \"dep\" }",
        "[dev-dependencies]\nforbidden = { path = \"dep\" }",
        "[build-dependencies]\nforbidden = { path = \"dep\" }",
        "[target.'cfg(unix)'.dependencies]\nforbidden = { path = \"dep\" }",
        "[target.'cfg(unix)'.dev-dependencies]\nforbidden = { path = \"dep\" }",
        "[target.'cfg(unix)'.build-dependencies]\nforbidden = { path = \"dep\" }",
        "[dependencies.forbidden]\npath = \"dep\"",
        "[target.'cfg(unix)'.dependencies.forbidden]\npath = \"dep\"",
    ];

    for declaration in declarations {
        let fixture = ManifestFixture::new(declaration);
        let error = manifest_boundary(&fixture.manifest, "boundary-fixture")
            .expect_err("Cargo metadata must expose the dependency declaration");
        assert!(
            error.contains("dependency declarations"),
            "declaration was not parsed as a dependency: {declaration}: {error}"
        );
    }
}

#[test]
fn cargo_metadata_rejects_commented_publish_false_bypass() {
    let fixture = ManifestFixture::with_publish_line("# publish = false", "");
    let error = manifest_boundary(&fixture.manifest, "boundary-fixture")
        .expect_err("Cargo metadata must report the package as publishable");
    assert!(
        error.contains("can be published"),
        "publish rejection must come from parsed metadata: {error}"
    );
}

fn manifest_boundary(manifest_path: &Path, expected_name: &str) -> Result<(), String> {
    let mut command = Command::new(
        std::env::var_os("CARGO").unwrap_or_else(|| std::ffi::OsString::from("cargo")),
    );
    command.args([
        "metadata",
        "--format-version",
        "1",
        "--no-deps",
        "--offline",
        "--manifest-path",
    ]);
    command.arg(manifest_path);
    if manifest_path == crate_root().join("Cargo.toml") {
        command.arg("--locked");
    }
    let output = command
        .output()
        .map_err(|error| format!("could not run Cargo metadata: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "Cargo metadata rejected the manifest: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let metadata = JsonParser::parse(&output.stdout)?;
    let packages = json_array(json_field(&metadata, "packages")?, "packages")?;
    let canonical_manifest = fs::canonicalize(manifest_path)
        .map_err(|error| format!("could not canonicalize manifest: {error}"))?;
    let mut matching = packages.iter().filter(|package| {
        let Ok(path_value) = json_field(package, "manifest_path") else {
            return false;
        };
        let Ok(path) = json_string(path_value, "manifest_path") else {
            return false;
        };
        fs::canonicalize(path).is_ok_and(|path| path == canonical_manifest)
    });
    let package = matching
        .next()
        .ok_or_else(|| "Cargo metadata omitted the selected package".to_string())?;
    if matching.next().is_some() {
        return Err("Cargo metadata returned the selected package twice".into());
    }

    expect_json_string(package, "name", expected_name)?;
    expect_json_string(package, "version", "0.1.0")?;
    expect_json_string(package, "edition", "2021")?;
    expect_json_string(package, "rust_version", "1.97")?;
    expect_json_string(package, "license", "Apache-2.0")?;
    expect_json_string(
        package,
        "repository",
        "https://github.com/IcantFind-a-username/Sovereign-Founder-OS",
    )?;

    match json_field(package, "publish")? {
        JsonValue::Array(registries) if registries.is_empty() => {}
        _ => return Err("Cargo metadata says the package can be published".into()),
    }
    let dependencies = json_array(json_field(package, "dependencies")?, "dependencies")?;
    if !dependencies.is_empty() {
        return Err("Cargo metadata reports dependency declarations".into());
    }
    let features = json_object(json_field(package, "features")?, "features")?;
    if !features.is_empty() {
        return Err("Cargo metadata reports feature declarations".into());
    }
    let package_root = canonical_manifest
        .parent()
        .ok_or_else(|| "selected manifest has no package directory".to_string())?;
    let source_root = package_root.join("src");
    source_root_boundary(&source_root)
        .map_err(|error| format!("Cargo package source root rejected: {error:?}"))?;
    let expected_lib = fs::canonicalize(source_root.join("lib.rs"))
        .map_err(|error| format!("could not canonicalize library source: {error}"))?;
    let mut library_targets = 0;
    for target in json_array(json_field(package, "targets")?, "targets")? {
        let kinds = json_array(json_field(target, "kind")?, "target.kind")?;
        let kinds = kinds
            .iter()
            .map(|kind| json_string(kind, "target kind"))
            .collect::<Result<Vec<_>, _>>()?;
        if kinds.contains(&"custom-build") {
            return Err("Cargo metadata reports a build script".into());
        }
        if kinds.contains(&"lib") {
            library_targets += 1;
            let source = json_string(json_field(target, "src_path")?, "target.src_path")?;
            let source = fs::canonicalize(source)
                .map_err(|error| format!("could not canonicalize target source: {error}"))?;
            if source != expected_lib {
                return Err("Cargo metadata points the library outside `src/lib.rs`".into());
            }
        } else if kinds != ["test"] {
            return Err(format!(
                "Cargo metadata reports unexpected target kinds {kinds:?}"
            ));
        }
    }
    if library_targets != 1 {
        return Err(format!(
            "Cargo metadata reports {library_targets} library targets"
        ));
    }
    Ok(())
}

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
enum SourceBoundaryKind {
    Lexical,
    PathAttribute,
    LibCfgAttrShape,
    LibItemShape,
    DomainTestModuleShape,
    DomainProductionShape,
    UnexpectedSourceFile,
}

#[derive(Debug)]
struct SourceBoundaryError {
    kind: SourceBoundaryKind,
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

fn source_boundary(path: &Path, source: &str) -> Result<(), SourceBoundaryError> {
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

#[derive(Clone, Debug, Eq, PartialEq)]
enum RustToken {
    Ident(String),
    Punct(char),
    Literal(String),
}

struct RustLexer<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

impl<'a> RustLexer<'a> {
    fn lex(source: &'a str) -> Result<Vec<RustToken>, String> {
        let mut lexer = Self {
            bytes: source.as_bytes(),
            cursor: 0,
        };
        let mut tokens = Vec::new();
        while lexer.cursor < lexer.bytes.len() {
            let byte = lexer.bytes[lexer.cursor];
            if byte.is_ascii_whitespace() {
                lexer.cursor += 1;
            } else if lexer.starts_with(b"///")
                || lexer.starts_with(b"//!")
                || lexer.starts_with(b"/**")
                || lexer.starts_with(b"/*!")
            {
                return Err("Rust doc comments are attributes outside the Task 1 grammar".into());
            } else if lexer.starts_with(b"//") {
                lexer.skip_line_comment();
            } else if lexer.starts_with(b"/*") {
                lexer.skip_block_comment()?;
            } else if lexer.raw_string_prefix().is_some() {
                let start = lexer.cursor;
                lexer.skip_raw_string()?;
                tokens.push(RustToken::Literal(lexer.source_slice(start)?));
            } else if byte == b'"' {
                let start = lexer.cursor;
                lexer.skip_quoted(b'"')?;
                tokens.push(RustToken::Literal(lexer.source_slice(start)?));
            } else if lexer.starts_with(b"r#")
                && lexer
                    .bytes
                    .get(lexer.cursor + 2)
                    .is_some_and(|byte| is_ident_start(*byte))
            {
                lexer.cursor += 2;
                tokens.push(RustToken::Ident(lexer.take_identifier()?));
            } else if is_ident_start(byte) {
                tokens.push(RustToken::Ident(lexer.take_identifier()?));
            } else if byte.is_ascii_digit() {
                let start = lexer.cursor;
                lexer.skip_number();
                tokens.push(RustToken::Literal(lexer.source_slice(start)?));
            } else if byte.is_ascii_punctuation() {
                lexer.cursor += 1;
                tokens.push(RustToken::Punct(char::from(byte)));
            } else {
                return Err(format!(
                    "unsupported non-ASCII Rust token at byte {}",
                    lexer.cursor
                ));
            }
        }
        Ok(tokens)
    }

    fn starts_with(&self, value: &[u8]) -> bool {
        self.bytes[self.cursor..].starts_with(value)
    }

    fn take_identifier(&mut self) -> Result<String, String> {
        let start = self.cursor;
        if !self
            .bytes
            .get(self.cursor)
            .is_some_and(|byte| is_ident_start(*byte))
        {
            return Err("identifier has no valid first byte".into());
        }
        self.cursor += 1;
        while self
            .bytes
            .get(self.cursor)
            .is_some_and(|byte| is_ident_continue(*byte))
        {
            self.cursor += 1;
        }
        String::from_utf8(self.bytes[start..self.cursor].to_vec())
            .map_err(|error| format!("identifier is not UTF-8: {error}"))
    }

    fn source_slice(&self, start: usize) -> Result<String, String> {
        String::from_utf8(self.bytes[start..self.cursor].to_vec())
            .map_err(|error| format!("Rust token is not UTF-8: {error}"))
    }

    fn skip_line_comment(&mut self) {
        self.cursor += 2;
        while self.cursor < self.bytes.len() && self.bytes[self.cursor] != b'\n' {
            self.cursor += 1;
        }
    }

    fn skip_block_comment(&mut self) -> Result<(), String> {
        self.cursor += 2;
        let mut depth = 1_u32;
        while self.cursor < self.bytes.len() {
            if self.starts_with(b"/*") {
                depth += 1;
                self.cursor += 2;
            } else if self.starts_with(b"*/") {
                depth -= 1;
                self.cursor += 2;
                if depth == 0 {
                    return Ok(());
                }
            } else {
                self.cursor += 1;
            }
        }
        Err("unterminated Rust block comment".into())
    }

    fn raw_string_prefix(&self) -> Option<(usize, usize)> {
        let prefix = if self.starts_with(b"br") { 2 } else { 1 };
        if self.bytes.get(self.cursor) != Some(&b'r') && !self.starts_with(b"br") {
            return None;
        }
        let mut cursor = self.cursor + prefix;
        let mut hashes = 0;
        while self.bytes.get(cursor) == Some(&b'#') {
            hashes += 1;
            cursor += 1;
        }
        (self.bytes.get(cursor) == Some(&b'"')).then_some((cursor, hashes))
    }

    fn skip_raw_string(&mut self) -> Result<(), String> {
        let (quote, hashes) = self
            .raw_string_prefix()
            .ok_or_else(|| "invalid raw string prefix".to_string())?;
        self.cursor = quote + 1;
        while self.cursor < self.bytes.len() {
            if self.bytes[self.cursor] == b'"'
                && self.bytes.get(self.cursor + 1..self.cursor + 1 + hashes)
                    == Some(&vec![b'#'; hashes][..])
            {
                self.cursor += 1 + hashes;
                return Ok(());
            }
            self.cursor += 1;
        }
        Err("unterminated Rust raw string".into())
    }

    fn skip_quoted(&mut self, quote: u8) -> Result<(), String> {
        self.cursor += 1;
        while self.cursor < self.bytes.len() {
            match self.bytes[self.cursor] {
                b'\\' => self.cursor = (self.cursor + 2).min(self.bytes.len()),
                byte if byte == quote => {
                    self.cursor += 1;
                    return Ok(());
                }
                _ => self.cursor += 1,
            }
        }
        Err("unterminated Rust quoted literal".into())
    }

    fn skip_number(&mut self) {
        while self.bytes.get(self.cursor).is_some_and(|byte| {
            byte.is_ascii_alphanumeric() || matches!(*byte, b'_' | b'.' | b'+' | b'-')
        }) {
            self.cursor += 1;
        }
    }
}

fn is_ident_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

fn is_ident_continue(byte: u8) -> bool {
    is_ident_start(byte) || byte.is_ascii_digit()
}

#[derive(Debug)]
enum JsonValue {
    String(String),
    Array(Vec<JsonValue>),
    Object(BTreeMap<String, JsonValue>),
    Scalar,
}

struct JsonParser<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

impl<'a> JsonParser<'a> {
    fn parse(bytes: &'a [u8]) -> Result<JsonValue, String> {
        let mut parser = Self { bytes, cursor: 0 };
        let value = parser.parse_value()?;
        parser.skip_whitespace();
        if parser.cursor != parser.bytes.len() {
            return Err(format!(
                "unexpected trailing JSON at byte {}",
                parser.cursor
            ));
        }
        Ok(value)
    }

    fn parse_value(&mut self) -> Result<JsonValue, String> {
        self.skip_whitespace();
        match self.bytes.get(self.cursor).copied() {
            Some(b'{') => self.parse_object(),
            Some(b'[') => self.parse_array(),
            Some(b'"') => self.parse_string().map(JsonValue::String),
            Some(b't') => self.parse_keyword(b"true"),
            Some(b'f') => self.parse_keyword(b"false"),
            Some(b'n') => self.parse_keyword(b"null"),
            Some(b'-' | b'0'..=b'9') => self.parse_number(),
            Some(byte) => Err(format!(
                "unexpected JSON byte `{}` at {}",
                char::from(byte),
                self.cursor
            )),
            None => Err("unexpected end of JSON".into()),
        }
    }

    fn parse_object(&mut self) -> Result<JsonValue, String> {
        self.cursor += 1;
        let mut values = BTreeMap::new();
        self.skip_whitespace();
        if self.take(b'}') {
            return Ok(JsonValue::Object(values));
        }
        loop {
            let key = self.parse_string()?;
            self.skip_whitespace();
            self.expect(b':')?;
            let value = self.parse_value()?;
            if values.insert(key.clone(), value).is_some() {
                return Err(format!("duplicate JSON object key `{key}`"));
            }
            self.skip_whitespace();
            if self.take(b'}') {
                break;
            }
            self.expect(b',')?;
            self.skip_whitespace();
        }
        Ok(JsonValue::Object(values))
    }

    fn parse_array(&mut self) -> Result<JsonValue, String> {
        self.cursor += 1;
        let mut values = Vec::new();
        self.skip_whitespace();
        if self.take(b']') {
            return Ok(JsonValue::Array(values));
        }
        loop {
            values.push(self.parse_value()?);
            self.skip_whitespace();
            if self.take(b']') {
                break;
            }
            self.expect(b',')?;
        }
        Ok(JsonValue::Array(values))
    }

    fn parse_string(&mut self) -> Result<String, String> {
        self.expect(b'"')?;
        let mut value = Vec::new();
        while let Some(byte) = self.bytes.get(self.cursor).copied() {
            self.cursor += 1;
            match byte {
                b'"' => {
                    return String::from_utf8(value)
                        .map_err(|error| format!("JSON string is not UTF-8: {error}"));
                }
                b'\\' => self.parse_string_escape(&mut value)?,
                0x00..=0x1f => return Err("JSON string contains a control byte".into()),
                _ => value.push(byte),
            }
        }
        Err("unterminated JSON string".into())
    }

    fn parse_string_escape(&mut self, value: &mut Vec<u8>) -> Result<(), String> {
        let escaped = self
            .bytes
            .get(self.cursor)
            .copied()
            .ok_or_else(|| "unterminated JSON escape".to_string())?;
        self.cursor += 1;
        match escaped {
            b'"' | b'\\' | b'/' => value.push(escaped),
            b'b' => value.push(0x08),
            b'f' => value.push(0x0c),
            b'n' => value.push(b'\n'),
            b'r' => value.push(b'\r'),
            b't' => value.push(b'\t'),
            b'u' => {
                let end = self.cursor + 4;
                let digits = self
                    .bytes
                    .get(self.cursor..end)
                    .ok_or_else(|| "short JSON Unicode escape".to_string())?;
                let digits = std::str::from_utf8(digits)
                    .map_err(|error| format!("invalid JSON Unicode escape: {error}"))?;
                let code = u32::from_str_radix(digits, 16)
                    .map_err(|error| format!("invalid JSON Unicode escape: {error}"))?;
                let character = char::from_u32(code)
                    .ok_or_else(|| "unsupported JSON surrogate escape".to_string())?;
                let mut encoded = [0_u8; 4];
                value.extend_from_slice(character.encode_utf8(&mut encoded).as_bytes());
                self.cursor = end;
            }
            _ => return Err("unknown JSON string escape".into()),
        }
        Ok(())
    }

    fn parse_keyword(&mut self, keyword: &[u8]) -> Result<JsonValue, String> {
        if !self.bytes[self.cursor..].starts_with(keyword) {
            return Err(format!("invalid JSON keyword at byte {}", self.cursor));
        }
        self.cursor += keyword.len();
        Ok(JsonValue::Scalar)
    }

    fn parse_number(&mut self) -> Result<JsonValue, String> {
        let start = self.cursor;
        while self.bytes.get(self.cursor).is_some_and(|byte| {
            byte.is_ascii_digit() || matches!(*byte, b'-' | b'+' | b'.' | b'e' | b'E')
        }) {
            self.cursor += 1;
        }
        if self.cursor == start {
            return Err("empty JSON number".into());
        }
        Ok(JsonValue::Scalar)
    }

    fn skip_whitespace(&mut self) {
        while self
            .bytes
            .get(self.cursor)
            .is_some_and(|byte| byte.is_ascii_whitespace())
        {
            self.cursor += 1;
        }
    }

    fn take(&mut self, expected: u8) -> bool {
        if self.bytes.get(self.cursor) == Some(&expected) {
            self.cursor += 1;
            true
        } else {
            false
        }
    }

    fn expect(&mut self, expected: u8) -> Result<(), String> {
        if self.take(expected) {
            Ok(())
        } else {
            Err(format!(
                "expected JSON `{}` at byte {}",
                char::from(expected),
                self.cursor
            ))
        }
    }
}

fn json_object<'a>(
    value: &'a JsonValue,
    label: &str,
) -> Result<&'a BTreeMap<String, JsonValue>, String> {
    match value {
        JsonValue::Object(value) => Ok(value),
        _ => Err(format!("Cargo metadata `{label}` is not an object")),
    }
}

fn json_array<'a>(value: &'a JsonValue, label: &str) -> Result<&'a [JsonValue], String> {
    match value {
        JsonValue::Array(value) => Ok(value),
        _ => Err(format!("Cargo metadata `{label}` is not an array")),
    }
}

fn json_string<'a>(value: &'a JsonValue, label: &str) -> Result<&'a str, String> {
    match value {
        JsonValue::String(value) => Ok(value),
        _ => Err(format!("Cargo metadata `{label}` is not a string")),
    }
}

fn json_field<'a>(value: &'a JsonValue, field: &str) -> Result<&'a JsonValue, String> {
    json_object(value, "object")?
        .get(field)
        .ok_or_else(|| format!("Cargo metadata omitted `{field}`"))
}

fn expect_json_string(value: &JsonValue, field: &str, expected: &str) -> Result<(), String> {
    let actual = json_string(json_field(value, field)?, field)?;
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "Cargo metadata `{field}` is `{actual}`, expected `{expected}`"
        ))
    }
}

struct ManifestFixture {
    root: PathBuf,
    manifest: PathBuf,
}

#[cfg(any(unix, windows))]
struct SymlinkedSourceFixture {
    base: ManifestFixture,
    source_link: PathBuf,
}

#[cfg(any(unix, windows))]
impl SymlinkedSourceFixture {
    fn new() -> std::io::Result<Self> {
        let base = ManifestFixture::new("");
        let real_source = base.root.join("real-source");
        let source_link = base.root.join("src");
        fs::create_dir(&real_source)?;
        fs::write(real_source.join("lib.rs"), "mod domain;\n")?;
        fs::write(real_source.join("domain.rs"), "")?;
        fs::remove_dir_all(&source_link)?;
        create_directory_symlink(&real_source, &source_link)?;
        Ok(Self { base, source_link })
    }
}

#[cfg(unix)]
fn create_directory_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(windows)]
fn create_directory_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_dir(target, link)
}

impl ManifestFixture {
    fn new(declaration: &str) -> Self {
        Self::with_publish_line("publish = false", declaration)
    }

    fn with_publish_line(publish_line: &str, declaration: &str) -> Self {
        let sequence = FIXTURE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "sovereign-playground-boundary-{}-{sequence}",
            std::process::id()
        ));
        let dependency = root.join("dep");
        fs::create_dir_all(root.join("src")).expect("fixture source directory must be created");
        fs::create_dir_all(dependency.join("src"))
            .expect("fixture dependency source directory must be created");
        fs::write(root.join("src/lib.rs"), "").expect("fixture package source must be written");
        fs::write(dependency.join("src/lib.rs"), "")
            .expect("fixture dependency source must be written");
        fs::write(
            dependency.join("Cargo.toml"),
            "[package]\nname = \"forbidden\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .expect("fixture dependency manifest must be written");
        let manifest = root.join("Cargo.toml");
        fs::write(
            &manifest,
            format!(
                "[package]\nname = \"boundary-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\nrust-version = \"1.97\"\nlicense = \"Apache-2.0\"\nrepository = \"https://github.com/IcantFind-a-username/Sovereign-Founder-OS\"\n{publish_line}\n\n{declaration}\n"
            ),
        )
        .expect("fixture package manifest must be written");
        Self { root, manifest }
    }
}

impl Drop for ManifestFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[derive(Debug, Eq, PartialEq)]
enum SourceRootError {
    MetadataUnreadable,
    RootSymlink,
    RootNotDirectory,
}

fn source_root_boundary(source_root: &Path) -> Result<(), SourceRootError> {
    let metadata =
        fs::symlink_metadata(source_root).map_err(|_| SourceRootError::MetadataUnreadable)?;
    if metadata.file_type().is_symlink() {
        return Err(SourceRootError::RootSymlink);
    }
    if !metadata.is_dir() {
        return Err(SourceRootError::RootNotDirectory);
    }
    Ok(())
}

fn production_sources(source_root: &Path) -> Result<BTreeSet<PathBuf>, SourceRootError> {
    source_root_boundary(source_root)?;
    let mut sources = BTreeSet::new();
    collect_production_sources(source_root, &mut sources);
    Ok(sources)
}

fn collect_production_sources(directory: &Path, sources: &mut BTreeSet<PathBuf>) {
    for entry in fs::read_dir(directory).expect("source directory must be readable") {
        let entry = entry.expect("source entry must be readable");
        let file_type = entry.file_type().expect("source type must be readable");
        assert!(
            !file_type.is_symlink(),
            "production sources must not be symlinks"
        );
        let path = entry.path();
        if file_type.is_dir() {
            collect_production_sources(&path, sources);
            continue;
        }
        assert!(file_type.is_file(), "unexpected production source entry");
        assert_eq!(
            path.extension().and_then(|value| value.to_str()),
            Some("rs")
        );
        sources.insert(path);
    }
}

//! The pure HTTP handler has one frozen production grammar, not an IO exemption.

use std::path::Path;

#[path = "support/boundary.rs"]
mod boundary;
#[path = "support/rust_lexer.rs"]
mod rust_lexer;

use boundary::{source_boundary, SourceBoundaryKind, HTTP_PRODUCTION};

#[test]
fn http_gate_accepts_pinned_complete_handler() {
    let source = format!("{HTTP_PRODUCTION}\n#[cfg(test)]\nmod tests {{}}");
    source_boundary(Path::new("http.rs"), &source).expect("frozen complete pure handler");
}

#[test]
fn http_gate_rejects_authority_body_and_action_widening() {
    let source = http_fixture();
    source_boundary(Path::new("http.rs"), &source).expect("mutation baseline must pass");
    let mutations = [
        ("second handler field", "session: Mutex<PlaygroundSession>,", "session: Mutex<PlaygroundSession>, backend: String,"),
        ("second mutex", "session: Mutex<PlaygroundSession>,", "session: Mutex<PlaygroundSession>, other: Mutex<PlaygroundSession>,"),
        ("exposed session", "    session: Mutex<PlaygroundSession>,", "    pub(crate) session: Mutex<PlaygroundSession>,"),
        ("injected session constructor", "pub(crate) fn new() -> Self", "pub(crate) fn new(session: PlaygroundSession) -> Self"),
        ("injected root constructor", "pub(crate) fn new() -> Self", "pub(crate) fn new(root: &std::path::Path) -> Self"),
        ("request supplies port", "pub(crate) struct HttpRequest<'a> {", "pub(crate) struct HttpRequest<'a> { pub(crate) bound_port: u16,"),
        ("bound port overwritten", "if bound_port == 0 {", "let bound_port = request.target.parse::<u16>().unwrap(); if bound_port == 0 {"),
        ("zero port accepted", "bound_port == 0", "false"),
        ("expected Host from request", "format!(\"127.0.0.1:{bound_port}\")", "unique_header(request.headers, \"Host\").unwrap().unwrap().to_string()"),
        ("expected Origin from Host", "format!(\"http://127.0.0.1:{bound_port}\")", "format!(\"http://{}\", unique_header(request.headers, \"Host\").unwrap().unwrap())"),
        ("Host prefix instead of exact", "host == expected_host", "host.starts_with(\"127.0.0.1\")"),
        ("Host localhost", "host == expected_host", "host == expected_host || host == \"localhost\""),
        ("Host any default port", "bound_port == 80 && host", "bound_port != 0 && host"),
        ("Origin optional check removed", "_ => return reject(ErrorCode::OriginForbidden),", "_ => {},"),
        ("Origin null allowed", "origin == expected_origin", "origin == expected_origin || origin == \"null\""),
        ("Origin https allowed", "origin == expected_origin", "origin == expected_origin || origin == format!(\"https://127.0.0.1:{bound_port}\")"),
        ("duplicate headers ignored", "if values.next().is_some()", "if false"),
        ("header names trimmed", "header.eq_ignore_ascii_case(name)", "header.trim().eq_ignore_ascii_case(name)"),
        ("header values all whitespace trimmed", "value.trim_matches([' ', '\\t'])", "value.trim()"),
        ("target origin-form removed", "!request.target.starts_with('/')", "false"),
        ("query allowed", ".contains(['?', '#', '%', '\\\\', '\\r', '\\n', '\\0'])", ".contains(['#', '%', '\\\\', '\\r', '\\n', '\\0'])"),
        ("percent escape allowed", ".contains(['?', '#', '%', '\\\\', '\\r', '\\n', '\\0'])", ".contains(['?', '#', '\\\\', '\\r', '\\n', '\\0'])"),
        ("double slash allowed", "request.target.contains(\"//\")", "false"),
        ("dot segment allowed", "part == \".\" || part == \"..\"", "part == \"..\""),
        ("route normalized", "Route::from_target(request.target)", "Route::from_target(request.target.trim_end_matches('/'))"),
        ("unknown routes become state", "_ => None,", "_ => Some(Self::State),"),
        ("extra route alias", "\"/api/playground/consultant\" =>", "\"/alias\" | \"/api/playground/consultant\" =>"),
        ("asset path dynamic", "Asset(AssetRoute),", "Asset(String),"),
        ("method case folded", "request.method != method", "!request.method.eq_ignore_ascii_case(method)"),
        ("HEAD accepted", "request.method != method", "request.method != method && request.method != \"HEAD\""),
        ("body cap raised", "request.body.len() > 256", "request.body.len() > 257"),
        ("body character count", "request.body.len() > 256", "String::from_utf8_lossy(request.body).chars().count() > 256"),
        ("trust Content-Length", "request.body.len() > 256", "unique_header(request.headers, \"Content-Length\").unwrap().unwrap_or(\"0\").parse::<usize>().unwrap_or(0) > 256"),
        ("lock before input validation", "if bound_port == 0 {", "let _guard = self.session.lock(); if bound_port == 0 {"),
        ("encoded request accepted", "name.eq_ignore_ascii_case(\"Content-Encoding\")", "name.eq_ignore_ascii_case(\"Unused-Encoding\")"),
        ("GET body allowed", "if !request.body.is_empty()", "if false"),
        ("Content-Type broad prefix", "media_type.eq_ignore_ascii_case(\"application/json\")", "media_type.starts_with(\"application/json\")"),
        ("Content-Type missing allowed", "_ => return reject(ErrorCode::UnsupportedMediaType),", "_ => {},"),
        ("top-level object guard removed", "            if first_json_byte != Some(b\"{\"[0]) {\n                return reject(ErrorCode::InvalidActionRequest);\n            }\n", ""),
        ("top-level arrays allowed", "first_json_byte != Some(b\"{\"[0])", "first_json_byte != Some(b\"{\"[0]) && first_json_byte != Some(b\"[\"[0])"),
        ("only top-level arrays accepted", "first_json_byte != Some(b\"{\"[0])", "first_json_byte != Some(b\"[\"[0])"),
        ("JSON whitespace broadened", "!matches!(*byte, b' ' | b'\\t' | b'\\r' | b'\\n')", "!byte.is_ascii_whitespace()"),
        ("unknown JSON fields allowed", "#[serde(deny_unknown_fields)]", ""),
        ("enum-map action accepted", "#[serde(try_from = \"String\")]", ""),
        ("duplicate JSON fields coalesced", "serde_json::from_slice(request.body)", "serde_json::from_value(serde_json::from_slice::<serde_json::Value>(request.body).unwrap())"),
        ("action any string becomes reset", "_ => Err(\"invalid action\"),", "_ => Ok(Self::Reset),"),
        ("additional wire action", "    CorrectOfferPrice,", "    CorrectOfferPrice,\n    DeleteCustomer,"),
        ("additional action string", "\"Reset\" => Ok(Self::Reset),", "\"Reset\" | \"DeleteCustomer\" => Ok(Self::Reset),"),
        ("action payload", "    CorrectOfferPrice,", "    CorrectOfferPrice(u32),"),
        ("wire action wrong domain mapping", "Self::Reset => PlaygroundAction::Reset,", "Self::Reset => PlaygroundAction::CorrectOfferPrice,"),
        ("wire action not exhaustive", "Self::Reset => PlaygroundAction::Reset,", "_ => PlaygroundAction::Reset,"),
        ("JSON parse error commits reset", "Err(_) => return reject(ErrorCode::InvalidActionRequest),", "Err(_) => ActionRequest { action: WireAction::Reset },"),
        ("poison recovered", "Err(_) => return reject(ErrorCode::SessionUnavailable),", "Err(poisoned) => poisoned.into_inner(),"),
        ("poison cleared", "let mut session = match self.session.lock()", "self.session.clear_poison(); let mut session = match self.session.lock()"),
        ("poison resets state", "Err(_) => return reject(ErrorCode::SessionUnavailable),", "Err(poisoned) => { let mut session = poisoned.into_inner(); *session = PlaygroundSession::new(); session },"),
        ("apply omitted", "session.apply(action);", "let _ = action;"),
        ("snapshot from separate fixture", "StateResponse::snapshot(&session)", "StateResponse::snapshot(&PlaygroundSession::new())"),
        ("CORS header", "(\"Cache-Control\", \"no-store\"),", "(\"Cache-Control\", \"no-store\"), (\"Access-Control-Allow-Origin\", \"*\"),"),
        ("response cookie", "(\"Cache-Control\", \"no-store\"),", "(\"Cache-Control\", \"no-store\"), (\"Set-Cookie\", \"session=1\"),"),
        ("redirect header", "(\"Cache-Control\", \"no-store\"),", "(\"Cache-Control\", \"no-store\"), (\"Location\", \"/\"),"),
        ("input cookie read", "let expected_host =", "let _cookie = unique_header(request.headers, \"Cookie\"); let expected_host ="),
        ("free error message", "    error: ErrorCode,", "    error: ErrorCode,\n    message: String,"),
        ("request reflection", "type Error = &'static str;", "type Error = String;"),
        ("action error reflects input", "_ => Err(\"invalid action\"),", "_ => Err(value),"),
        ("real data claim", "real_data_enabled: false", "real_data_enabled: true"),
        ("persistence claim", "persistence: \"none\"", "persistence: \"disk\""),
        ("wrong profile", "profile: \"synthetic_playground\"", "profile: \"real_company\""),
        ("tagged response", "#[serde(untagged)]", "#[serde(tag = \"kind\")]"),
        ("twelfth error", "    SessionUnavailable,", "    SessionUnavailable,\n    BackendError,"),
        ("extra arbitrary module", "use std::sync::Mutex;", "use std::sync::Mutex; mod backend;"),
        ("backend operation", "session.apply(action);", "backend.execute(action); session.apply(action);"),
        ("policy authorization", "session.apply(action);", "policy.authorize(action); session.apply(action);"),
        ("IO constructor", "session: Mutex::new(PlaygroundSession::new())", "session: { let _ = std::fs::read(\"business.db\"); Mutex::new(PlaygroundSession::new()) }"),
        ("environment constructor", "session: Mutex::new(PlaygroundSession::new())", "session: { let _ = std::env::var(\"COMPANY\"); Mutex::new(PlaygroundSession::new()) }"),
        ("network constructor", "session: Mutex::new(PlaygroundSession::new())", "session: { let _ = std::net::TcpListener::bind(\"127.0.0.1:0\"); Mutex::new(PlaygroundSession::new()) }"),
        ("process constructor", "session: Mutex::new(PlaygroundSession::new())", "session: { let _ = std::process::Command::new(\"sh\").spawn(); Mutex::new(PlaygroundSession::new()) }"),
        ("clock constructor", "session: Mutex::new(PlaygroundSession::new())", "session: { let _ = std::time::SystemTime::now(); Mutex::new(PlaygroundSession::new()) }"),
        ("environment macro", "let expected_host = format!(\"127.0.0.1:{bound_port}\");", "let expected_host = env!(\"HOST\");"),
        ("asset include", "return HandlerOutcome::Asset(asset);", "let _ = include_str!(\"../../private.txt\"); return HandlerOutcome::Asset(asset);"),
        ("unbounded IO", "if request.body.len() > 256", "let mut bytes = Vec::new(); reader.read_to_end(&mut bytes).unwrap(); if request.body.len() > 256"),
        ("unsafe code", "session.apply(action);", "unsafe { session.apply(action); }"),
        ("broad lint exemption", "use std::sync::Mutex;", "#![allow(warnings)] use std::sync::Mutex;"),
    ];
    for (name, target, replacement) in mutations {
        assert!(source.contains(target), "missing mutation target: {name}");
        let changed = source.replacen(target, replacement, 1);
        assert_ne!(changed, source, "no-op mutation: {name}");
        assert_rejected(
            name,
            "http.rs",
            &changed,
            SourceBoundaryKind::HttpProductionShape,
        );
    }
}

#[test]
fn http_gate_preserves_path_wrapper_and_module_rejections() {
    let source = http_fixture();
    source_boundary(Path::new("http.rs"), &source).expect("valid baseline");
    for (name, changed, expected) in [
        (
            "missing wrapper",
            HTTP_PRODUCTION.to_string(),
            SourceBoundaryKind::HttpTestModuleShape,
        ),
        (
            "changed cfg",
            source.replace("#[cfg(test)]", "#[cfg(any())]"),
            SourceBoundaryKind::HttpTestModuleShape,
        ),
        (
            "renamed wrapper",
            source.replace("mod tests", "mod other"),
            SourceBoundaryKind::HttpTestModuleShape,
        ),
        (
            "external wrapper",
            source.replace("mod tests {}", "mod tests;"),
            SourceBoundaryKind::HttpTestModuleShape,
        ),
        (
            "unbalanced wrapper",
            source
                .strip_suffix('}')
                .expect("wrapper ends in brace")
                .to_string(),
            SourceBoundaryKind::HttpTestModuleShape,
        ),
        (
            "second wrapper",
            format!("{source}\n#[cfg(test)] mod tests {{}}"),
            SourceBoundaryKind::HttpTestModuleShape,
        ),
        (
            "trailing code",
            format!("{source}\nfn escaped() {{}}"),
            SourceBoundaryKind::HttpTestModuleShape,
        ),
        (
            "nonterminal module",
            source.replace("#[cfg(test)]", "mod escaped {} #[cfg(test)]"),
            SourceBoundaryKind::HttpProductionShape,
        ),
        (
            "path attribute",
            source.replace(
                "#[cfg(test)]",
                "#[path=\"../outside.rs\"] mod escaped; #[cfg(test)]",
            ),
            SourceBoundaryKind::PathAttribute,
        ),
        (
            "nested path attribute",
            source.replace(
                "#[cfg(test)]",
                "#[cfg_attr(any(), path=\"../outside.rs\")] mod escaped; #[cfg(test)]",
            ),
            SourceBoundaryKind::PathAttribute,
        ),
        (
            "ordinary top-level test leaks",
            source.replace("#[cfg(test)]\nmod tests {}", "#[test] fn leaked() {}"),
            SourceBoundaryKind::HttpTestModuleShape,
        ),
    ] {
        assert_ne!(changed, source, "{name}");
        assert_rejected(name, "http.rs", &changed, expected);
    }
    for changed in [
        source.replace(
            "mod tests {}",
            "mod tests { fn helper() { let _ = \"{ std::fs::read }\"; } }",
        ),
        format!("// Comments do not widen the token grammar.\n{source}\n/* trailing comment */"),
    ] {
        assert_ne!(changed, source);
        source_boundary(Path::new("http.rs"), &changed)
            .expect("exact terminal wrapper and ordinary comments");
    }
    assert_rejected(
        "unknown source",
        "handler.rs",
        &source,
        SourceBoundaryKind::UnexpectedSourceFile,
    );
    let lib = boundary::EXPECTED_LIB_HTTP_SHAPE;
    source_boundary(Path::new("lib.rs"), lib).expect("new lib declaration");
    for (name, changed, kind) in [
        (
            "extra lib module",
            format!("{lib}\nmod network;"),
            SourceBoundaryKind::LibItemShape,
        ),
        (
            "http declaration renamed",
            lib.replace("mod http;", "mod network;"),
            SourceBoundaryKind::LibItemShape,
        ),
        (
            "HTTP cfg changed",
            lib.replace("mod http;", "#[cfg(any())] mod http;"),
            SourceBoundaryKind::LibItemShape,
        ),
        (
            "lib path escape",
            lib.replace("mod http;", "#[path=\"../http.rs\"] mod http;"),
            SourceBoundaryKind::PathAttribute,
        ),
        (
            "HTTP public module",
            lib.replace("mod http;", "pub mod http;"),
            SourceBoundaryKind::LibItemShape,
        ),
    ] {
        assert_ne!(changed, lib, "{name}");
        assert_rejected(name, "lib.rs", &changed, kind);
    }
}

fn http_fixture() -> String {
    format!("{HTTP_PRODUCTION}\n#[cfg(test)]\nmod tests {{}}")
}

fn assert_rejected(name: &str, filename: &str, source: &str, kind: SourceBoundaryKind) {
    let error = source_boundary(Path::new(filename), source).expect_err(name);
    assert_eq!(error.kind, kind, "{name}: {error}");
}

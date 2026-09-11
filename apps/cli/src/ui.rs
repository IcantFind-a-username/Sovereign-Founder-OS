//! Local Security Center: a read-only dashboard over the real kernel state
//! plus an in-memory attack gauntlet, served on loopback only.
//!
//! Trust posture, stated explicitly:
//! - binds 127.0.0.1 only; this is a single-user local preview, not a hosted
//!   service, and it has no authentication because it never leaves the device;
//! - GET endpoints expose no secrets: vault entry *names* only, digests, and
//!   admission-record claims — never vault plaintext or private keys;
//! - the gauntlet runs entirely in memory with the hard-coded demo trust
//!   anchors; it performs no external effects and touches no stored state;
//! - this page hosts an early Founder Command Center: a read-only, at-a-glance
//!   view of the business joined with the kernel evidence that backs it.

use std::io::Read as _;
use std::path::{Path, PathBuf};

use sovereign_artifact::{AdmissionRecordClaimsV1, HARD_MAX_SIGNED_ADMISSION_BYTES};
use sovereign_audit_ledger::AuditLedger;
use sovereign_identity::DeviceIdentity;
use tiny_http::{Header, Method, Response, Server};
use uuid::Uuid;

use crate::demo;
use crate::workspace;

// The frontend is deliberately a zero-dependency static bundle: no framework,
// no npm supply chain, every byte embedded in this binary at compile time and
// served from memory. Front and back communicate only via the /api JSON
// endpoints below — the interface boundary of an SPA without the toolchain.
const UI_HTML: &str = include_str!("../assets/index.html");
const UI_CSS: &str = include_str!("../assets/styles.css");
const UI_I18N_JS: &str = include_str!("../assets/i18n.js");
const UI_APP_JS: &str = include_str!("../assets/app.js");
const UI_I18N_MVP_JS: &str = include_str!("../assets/i18n-mvp.js");
const UI_CRM_JS: &str = include_str!("../assets/crm.js");
const UI_TEAM_JS: &str = include_str!("../assets/team.js");
const UI_COMPLIANCE_JS: &str = include_str!("../assets/compliance.js");
const UI_PRIVACY_JS: &str = include_str!("../assets/privacy.js");
const UI_FAVICON: &str = include_str!("../assets/favicon.svg");
const JS_TYPE: &str = "application/javascript; charset=utf-8";

const MAX_REQUEST_BODY_BYTES: usize = 64 * 1024;
/// The one body the app itself asks a person to post back is an export
/// bundle — "verify a backup file" — and a workspace with a handful of
/// documents and a hundred audit events is already past the general cap.
/// That route takes a larger body; larger is still bounded, for the same
/// reason the general cap exists.
const MAX_VERIFY_EXPORT_BODY_BYTES: usize = 32 * 1024 * 1024;

/// Marker line printed once the server is listening, carrying the address it
/// actually bound. A supervising process (the desktop shell) reads this from
/// stdout instead of guessing a port, which is what makes `--port 0` usable.
pub const READY_PREFIX: &str = "sovereign-ui listening on ";

pub fn run(
    port: u16,
    root: PathBuf,
    open_browser: bool,
    supervised: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let (server, port) = bind(port)?;
    if supervised {
        watch_supervisor();
    }
    let url = format!("http://127.0.0.1:{port}");
    println!("Sovereign Founder OS · local app");
    println!("  {url}");
    println!("  loopback only · encrypted local state · Ctrl-C to stop");
    // Machine-readable and flushed, so a parent process can act on it the
    // moment the socket is accepting rather than polling a guessed port.
    println!("{READY_PREFIX}{url}");
    use std::io::Write as _;
    let _ = std::io::stdout().flush();
    if open_browser {
        launch_browser(&url);
    }
    serve(server, port, &root);
    Ok(())
}

/// Stop when the launching process goes away.
///
/// The supervisor holds this program's stdin open and writes nothing. If it
/// exits — cleanly, crashing, or killed outright — the OS closes the pipe,
/// the read returns end-of-file, and the runtime stops with it. Without this
/// a force-killed desktop shell would leave a server running against the
/// owner's vault. Exiting here is the crash case the storage layer already
/// handles: commits are audit-first and written atomically, so an interrupted
/// operation is retryable, never half-applied.
fn watch_supervisor() {
    std::thread::spawn(|| {
        let mut byte = [0u8; 1];
        loop {
            match std::io::Read::read(&mut std::io::stdin().lock(), &mut byte) {
                // End of file: the supervisor is gone.
                Ok(0) => break,
                // A supervisor that writes is not part of this protocol;
                // ignore the bytes and keep watching the pipe.
                Ok(_) => continue,
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            }
        }
        std::process::exit(0);
    });
}

/// Bind the loopback listener and report the port it actually got. Port 0
/// asks the OS for an ephemeral one, so a supervised launch never collides
/// with something already on 7787.
pub fn bind(port: u16) -> Result<(Server, u16), Box<dyn std::error::Error>> {
    let server = Server::http(("127.0.0.1", port))
        .map_err(|error| format!("cannot bind 127.0.0.1:{port}: {error}"))?;
    let bound = server
        .server_addr()
        .to_ip()
        .map(|address| address.port())
        .ok_or("listener has no IP address")?;
    Ok((server, bound))
}

/// Serve until the listener closes. `port` is the bound port, which the
/// `Host` allow-list checks against.
pub fn serve(server: Server, port: u16, root: &Path) {
    for mut request in server.incoming_requests() {
        let response = route(&mut request, port, root);
        let _ = request.respond(response);
    }
}

/// Best-effort convenience only: a failure to open a browser is silent and
/// harmless; the printed URL remains the source of truth.
fn launch_browser(url: &str) {
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = std::process::Command::new("open");
        command.arg(url);
        command
    };
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = std::process::Command::new("cmd");
        command.args(["/C", "start", "", url]);
        command
    };
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = {
        let mut command = std::process::Command::new("xdg-open");
        command.arg(url);
        command
    };
    let _ = command
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}

type UiResponse = Response<std::io::Cursor<Vec<u8>>>;

fn route(request: &mut tiny_http::Request, port: u16, root: &Path) -> UiResponse {
    // DNS-rebinding defense: a browser lured to attacker.example resolving to
    // 127.0.0.1 still sends the attacker's Host header; refuse it.
    if !host_allowed(request, port) {
        return json_response(&serde_json::json!({ "ok": false, "error": "forbidden host" }))
            .with_status_code(403);
    }
    let url = request.url().to_owned();
    match (request.method().clone(), url.as_str()) {
        (Method::Get, "/") => html_response(UI_HTML),
        (Method::Get, "/assets/styles.css") => asset_response(UI_CSS, "text/css; charset=utf-8"),
        (Method::Get, "/assets/i18n.js") => {
            asset_response(UI_I18N_JS, "application/javascript; charset=utf-8")
        }
        (Method::Get, "/assets/app.js") => {
            asset_response(UI_APP_JS, "application/javascript; charset=utf-8")
        }
        (Method::Get, "/assets/i18n-mvp.js") => asset_response(UI_I18N_MVP_JS, JS_TYPE),
        (Method::Get, "/assets/crm.js") => asset_response(UI_CRM_JS, JS_TYPE),
        (Method::Get, "/assets/team.js") => asset_response(UI_TEAM_JS, JS_TYPE),
        (Method::Get, "/assets/compliance.js") => asset_response(UI_COMPLIANCE_JS, JS_TYPE),
        (Method::Get, "/assets/privacy.js") => asset_response(UI_PRIVACY_JS, JS_TYPE),
        (Method::Get, "/favicon.svg") => asset_response(UI_FAVICON, "image/svg+xml"),
        (Method::Get, "/api/state") => json_response(&state_json(root)),
        (Method::Get, "/api/command-center") => json_response(&command_center_json(root)),
        (Method::Get, "/api/workspace") => json_response(&workspace_get(root)),
        (Method::Get, "/api/export") => export_response(root),
        (Method::Get, "/api/model/status") => json_response(&crate::ui_mvp::model_status(root)),
        (Method::Post, "/api/gauntlet") => match read_json_body(request) {
            Ok(_) => json_response(&crate::ui_gauntlet::gauntlet_json()),
            Err(error) => bad_request(&error),
        },
        (Method::Post, "/api/workspace/assist") => match read_json_body(request) {
            Ok(body) => json_response(&workspace_assist(&body, root)),
            Err(error) => bad_request(&error),
        },
        (Method::Post, "/api/verify-export") => {
            match read_json_body_capped(request, MAX_VERIFY_EXPORT_BODY_BYTES) {
                Ok(body) => json_response(&verify_export_json(&body)),
                Err(error) => bad_request(&error),
            }
        }
        (Method::Post, path)
            if path.starts_with("/api/workspace/") || path.starts_with("/api/privacy/") =>
        {
            match read_json_body(request) {
                Ok(body) => json_response(&workspace_post(path, &body, root)),
                Err(error) => bad_request(&error),
            }
        }
        _ => Response::from_string("not found").with_status_code(404),
    }
}

fn host_allowed(request: &tiny_http::Request, port: u16) -> bool {
    let allowed = [
        format!("127.0.0.1:{port}"),
        format!("localhost:{port}"),
        "127.0.0.1".to_owned(),
        "localhost".to_owned(),
    ];
    request
        .headers()
        .iter()
        .find(|header| header.field.equiv("Host"))
        .map(|header| {
            let value = header.value.as_str();
            allowed.iter().any(|candidate| candidate == value)
        })
        .unwrap_or(false)
}

/// Read a JSON request body. Requiring `Content-Type: application/json` is a
/// CSRF defense: cross-origin pages cannot send that content type without a
/// CORS preflight, which this server never approves.
fn read_json_body(request: &mut tiny_http::Request) -> Result<serde_json::Value, String> {
    read_json_body_capped(request, MAX_REQUEST_BODY_BYTES)
}

fn read_json_body_capped(
    request: &mut tiny_http::Request,
    cap: usize,
) -> Result<serde_json::Value, String> {
    let is_json = request
        .headers()
        .iter()
        .find(|header| header.field.equiv("Content-Type"))
        .map(|header| {
            header
                .value
                .as_str()
                .to_ascii_lowercase()
                .starts_with("application/json")
        })
        .unwrap_or(false);
    if !is_json {
        return Err("Content-Type must be application/json".into());
    }
    let mut body = Vec::new();
    std::io::Read::take(request.as_reader(), (cap + 1) as u64)
        .read_to_end(&mut body)
        .map_err(|error| error.to_string())?;
    if body.len() > cap {
        return Err("request body too large".into());
    }
    if body.is_empty() {
        return Ok(serde_json::json!({}));
    }
    serde_json::from_slice(&body).map_err(|error| format!("invalid JSON body: {error}"))
}

fn bad_request(message: &str) -> UiResponse {
    json_response(&serde_json::json!({ "ok": false, "error": message })).with_status_code(400)
}

fn workspace_get(root: &Path) -> serde_json::Value {
    let result = workspace::Store::open(root).and_then(|store| store.load());
    match result {
        Ok(state) => serde_json::json!({ "ok": true, "workspace": state }),
        Err(error) => serde_json::json!({ "ok": false, "error": error.to_string() }),
    }
}

/// The Founder Command Center: the business at a glance joined with the kernel
/// evidence that backs it. Read-only; it composes two honest sources — the
/// workspace aggregation and the on-disk audit signals — and invents nothing.
fn command_center_json(root: &Path) -> serde_json::Value {
    let result = workspace::Store::open(root).and_then(|store| store.command_center());
    match result {
        Ok(summary) => serde_json::json!({
            "ok": true,
            "summary": summary,
            "kernel": kernel_evidence_json(root),
        }),
        Err(error) => serde_json::json!({ "ok": false, "error": error.to_string() }),
    }
}

/// Compact, verifiable kernel signals for the Command Center header: the audit
/// chain's health, how many model disclosures happened, and how many plugins
/// were admitted. Every number here is re-derivable from the export.
fn kernel_evidence_json(root: &Path) -> serde_json::Value {
    let device = DeviceIdentity::load(&root.join("device.json")).ok();
    let ledger_path = root.join("ledger.json");
    let (present, chain_ok, count, model_disclosures) = match (&device, ledger_path.exists()) {
        (Some(device), true) => match AuditLedger::load(&ledger_path, device.public_key_b64()) {
            Ok(ledger) => {
                let disclosures = ledger
                    .events()
                    .iter()
                    .filter(|event| event.action == "model.drafted")
                    .count();
                let chain_ok = ledger.verify_chain().is_ok();
                (true, chain_ok, ledger.events().len(), disclosures)
            }
            Err(_) => (true, false, 0, 0),
        },
        _ => (false, true, 0, 0),
    };
    serde_json::json!({
        "audit_chain_present": present,
        "audit_chain_ok": chain_ok,
        "audit_events": count,
        "model_disclosures": model_disclosures,
        "admitted_plugins": admitted_plugins_json(root).len(),
    })
}

fn workspace_post(path: &str, body: &serde_json::Value, root: &Path) -> serde_json::Value {
    if let Some(response) = crate::ui_mvp::workspace_post(path, body, root) {
        return response;
    }
    let result = (|| {
        let store = workspace::Store::open(root)?;
        match path {
            "/api/workspace/venture" => {
                store.set_venture(str_field(body, "name")?, str_field(body, "service")?)
            }
            "/api/workspace/customer" => store.add_customer(
                str_field(body, "name")?,
                body.get("email").and_then(|v| v.as_str()).unwrap_or(""),
                body.get("notes").and_then(|v| v.as_str()).unwrap_or(""),
            ),
            "/api/workspace/offer" => store.create_document(
                workspace::DocumentKind::Offer,
                uuid_field(body, "customer_id")?,
                None,
                lang_field(body),
            ),
            "/api/workspace/invoice" => store.create_document(
                workspace::DocumentKind::Invoice,
                uuid_field(body, "customer_id")?,
                Some(workspace::parse_amount_cents(str_field(body, "amount")?)?),
                lang_field(body),
            ),
            "/api/workspace/request-send" => store.request_send(uuid_field(body, "document_id")?),
            "/api/workspace/decide" => store.decide(
                uuid_field(body, "approval_id")?,
                body.get("approve")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
            ),
            "/api/workspace/revoke" => store.revoke_delivery(uuid_field(body, "document_id")?),
            "/api/workspace/confirm-delivery" => {
                store.confirm_delivery(uuid_field(body, "document_id")?)
            }
            _ => Err(workspace::WorkspaceError::NotFound("route".into())),
        }
    })();
    match result {
        Ok(state) => serde_json::json!({ "ok": true, "workspace": state }),
        Err(error) => serde_json::json!({ "ok": false, "error": error.to_string() }),
    }
}

fn workspace_assist(body: &serde_json::Value, root: &Path) -> serde_json::Value {
    let result = (|| {
        let store = workspace::Store::open(root)?;
        store.draft_assistant(uuid_field(body, "customer_id")?, lang_field(body))
    })();
    match result {
        Ok(suggestion) => serde_json::json!({ "ok": true, "suggestion": suggestion }),
        Err(error) => serde_json::json!({ "ok": false, "error": error.to_string() }),
    }
}

/// Verify a bundle the user pasted or loaded in the browser. Pure: it opens no
/// store and needs no device, so it also works for a backup made on another
/// machine. The `bundle` is the exported JSON, nested under a `bundle` key.
fn verify_export_json(body: &serde_json::Value) -> serde_json::Value {
    let bundle = match body.get("bundle") {
        Some(bundle) if !bundle.is_null() => bundle,
        _ => return serde_json::json!({ "ok": false, "error": "no bundle provided" }),
    };
    match workspace::verify_export(bundle) {
        Ok(report) => serde_json::json!({ "ok": true, "report": report }),
        Err(error) => serde_json::json!({ "ok": false, "error": error.to_string() }),
    }
}

fn export_response(root: &Path) -> UiResponse {
    let export = workspace::Store::open(root).and_then(|store| store.export());
    match export {
        Ok(bundle) => {
            let pretty =
                serde_json::to_string_pretty(&bundle).unwrap_or_else(|_| bundle.to_string());
            Response::from_string(pretty)
                .with_header(
                    Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
                        .expect("static header"),
                )
                .with_header(
                    Header::from_bytes(
                        &b"Content-Disposition"[..],
                        &b"attachment; filename=\"sovereign-export.json\""[..],
                    )
                    .expect("static header"),
                )
        }
        Err(error) => {
            json_response(&serde_json::json!({ "ok": false, "error": error.to_string() }))
                .with_status_code(500)
        }
    }
}

pub(crate) fn str_field<'a>(
    body: &'a serde_json::Value,
    field: &str,
) -> Result<&'a str, workspace::WorkspaceError> {
    body.get(field)
        .and_then(|value| value.as_str())
        .ok_or_else(|| workspace::WorkspaceError::Invalid(format!("{field} is required")))
}

pub(crate) fn uuid_field(
    body: &serde_json::Value,
    field: &str,
) -> Result<Uuid, workspace::WorkspaceError> {
    str_field(body, field)?
        .parse()
        .map_err(|_| workspace::WorkspaceError::Invalid(format!("{field} must be a UUID")))
}

fn lang_field(body: &serde_json::Value) -> &str {
    match body.get("lang").and_then(|value| value.as_str()) {
        Some(lang) if lang.starts_with("zh") => "zh",
        _ => "en",
    }
}

fn html_response(body: &str) -> Response<std::io::Cursor<Vec<u8>>> {
    Response::from_string(body).with_header(
        Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..])
            .expect("static header"),
    )
}

/// Serve one compile-time-embedded static asset with its content type.
fn asset_response(body: &str, content_type: &str) -> Response<std::io::Cursor<Vec<u8>>> {
    Response::from_string(body).with_header(
        Header::from_bytes(b"Content-Type", content_type.as_bytes()).expect("static header"),
    )
}

fn json_response(value: &serde_json::Value) -> Response<std::io::Cursor<Vec<u8>>> {
    Response::from_string(value.to_string()).with_header(
        Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).expect("static header"),
    )
}

// ---------------------------------------------------------------------------
// GET /api/state — real on-disk kernel state, no secrets
// ---------------------------------------------------------------------------

fn state_json(root: &Path) -> serde_json::Value {
    let device = DeviceIdentity::load(&root.join("device.json")).ok();
    let device_id = device.as_ref().map(|d| d.device_id().to_owned());

    let vault_entries: Vec<String> = sovereign_vault::Vault::init(root.join("vault"))
        .map(|vault| vault.list().to_vec())
        .unwrap_or_default();

    let ledger_path = root.join("ledger.json");
    let ledger = match (&device, ledger_path.exists()) {
        (Some(device), true) => match AuditLedger::load(&ledger_path, device.public_key_b64()) {
            Ok(ledger) => {
                let events: Vec<serde_json::Value> = ledger
                    .events()
                    .iter()
                    .rev()
                    .take(25)
                    .map(|event| {
                        serde_json::json!({
                            "timestamp": event.timestamp.to_rfc3339(),
                            "actor": event.actor_id,
                            "action": event.action,
                            "resource": event.resource,
                            "hash": &event.event_hash[..12],
                        })
                    })
                    .collect();
                serde_json::json!({
                    "present": true,
                    "chain_ok": true,
                    "count": ledger.events().len(),
                    "events": events,
                })
            }
            Err(error) => serde_json::json!({
                "present": true,
                "chain_ok": false,
                "count": 0,
                "events": [],
                "error": error.to_string(),
            }),
        },
        _ => serde_json::json!({ "present": false, "chain_ok": false, "count": 0, "events": [] }),
    };

    serde_json::json!({
        "device_id": device_id,
        "vault_entries": vault_entries,
        "ledger": ledger,
        "integrity": integrity_json(root),
        "disclosures": disclosures_json(root),
        "plugins": admitted_plugins_json(root),
        "stage": "Stage 1 · Secure Kernel · Founder Command Center (early)",
    })
}

/// The owner-visible data-disclosure log for the Security Center: every time a
/// model provider was shown customer data, newest first. Never the suggestion,
/// only the disclosure. Silent on error rather than inventing a clean log.
fn disclosures_json(root: &Path) -> serde_json::Value {
    let workspace = match workspace::Store::open(root).and_then(|store| store.load()) {
        Ok(workspace) => workspace,
        Err(_) => return serde_json::json!([]),
    };
    // A nil customer is a company-level call (a compliance check of the
    // profile): `null`, which the page names in the founder's language. A
    // customer that has since been removed stays "(unknown)".
    let named = |id: uuid::Uuid| {
        if id.is_nil() {
            return serde_json::Value::Null;
        }
        serde_json::Value::String(
            workspace
                .customers
                .iter()
                .find(|customer| customer.id == id)
                .map(|customer| customer.name.clone())
                .unwrap_or_else(|| "(unknown)".into()),
        )
    };
    let mut entries: Vec<serde_json::Value> = workspace
        .disclosures
        .iter()
        .rev()
        .take(50)
        .map(|disclosure| {
            serde_json::json!({
                "at": disclosure.at,
                "customer": named(disclosure.customer_id),
                "task": disclosure.task,
                "provider": disclosure.provider_id,
                "provider_trust": disclosure.provider_trust,
                "stayed_local": disclosure.stayed_local,
                "data_class": disclosure.data_class,
                "output_chars": disclosure.output_chars,
                "failover_from": disclosure.failover_from,
            })
        })
        .collect();
    entries.truncate(50);
    serde_json::Value::Array(entries)
}

/// Self-audit for the Security Center: reconcile authoritative state against
/// the signed audit chain. Reports the verdict and any divergence; on any
/// internal error it reports that honestly rather than a false "ok".
fn integrity_json(root: &Path) -> serde_json::Value {
    match workspace::Store::open(root).and_then(|store| store.integrity_check()) {
        Ok(report) => serde_json::to_value(report)
            .unwrap_or_else(|_| serde_json::json!({ "ok": false, "error": "serialize" })),
        Err(error) => serde_json::json!({
            "ok": false,
            "chain_verified": false,
            "events": 0,
            "findings": [],
            "error": error.to_string(),
        }),
    }
}

/// List admission records from the on-disk store, verifying each against
/// the keys that sign them: the owner's admission key for what the send path
/// admitted, the demo key for what `sovereign demo` admitted. A record that
/// fails both is still listed — flagged unverified — because showing a
/// tampered record as "absent" would hide evidence from the owner.
fn admitted_plugins_json(root: &Path) -> Vec<serde_json::Value> {
    let admissions_dir = root.join("artifacts").join("admissions");
    let Ok(entries) = std::fs::read_dir(&admissions_dir) else {
        return Vec::new();
    };
    let trust = workspace::admission_trust(root);
    let now_unix = chrono::Utc::now().timestamp();

    let mut plugins = Vec::new();
    let mut names: Vec<_> = entries
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.len() == 64 && name.bytes().all(|b| b.is_ascii_hexdigit()))
        .collect();
    names.sort();
    for name in names {
        let path = admissions_dir.join(&name);
        let record = std::fs::metadata(&path)
            .ok()
            .filter(|metadata| {
                metadata.is_file() && metadata.len() <= HARD_MAX_SIGNED_ADMISSION_BYTES as u64
            })
            .and_then(|_| std::fs::read(&path).ok());
        let Some(record) = record else {
            continue;
        };
        let verified = [workspace::OWNER_ADMISSION_ISSUER, demo::ADMISSION_ISSUER]
            .iter()
            .find_map(|issuer| trust.verify(&record, issuer, now_unix).ok())
            .and_then(|verified| {
                serde_json::from_slice::<AdmissionRecordClaimsV1>(verified.payload()).ok()
            });
        match verified {
            Some(claims) => plugins.push(serde_json::json!({
                "verified": true,
                "admission_id": claims.admission_id,
                "component_digest": &claims.component_digest.as_hex()[..12],
                "manifest_digest": &claims.manifest_digest.as_hex()[..12],
                "risk_class": claims.risk_class,
                "backend": claims.backend,
                "state": claims.installation_state,
                "admitted_at_unix": claims.admitted_at_unix,
                "issuer": claims.admitting_issuer,
            })),
            None => plugins.push(serde_json::json!({
                "verified": false,
                "manifest_digest": &name[..12],
                "error": "admission record failed verification against the owner and demo admission anchors",
            })),
        }
    }
    plugins
}

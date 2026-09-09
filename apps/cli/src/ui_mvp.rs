//! JSON routes for the business graph (projects, tasks, follow-ups, payments,
//! editing) and, in later slices, the AI employees and compliance checks.
//! Same posture as `ui.rs`: loopback only, JSON bodies only, every mutation
//! goes through the workspace store and therefore through policy and the
//! signed audit chain. Split out so `ui.rs` stays readable.

use std::path::Path;

use uuid::Uuid;

use crate::ui::{str_field, uuid_field};
use crate::workspace::{self, ProjectStatus, WorkspaceError};

type Body = serde_json::Value;

/// `GET /api/model/status`: the device's configured providers with live
/// health, and where the founder edits them. Never a secret: model names
/// and loopback addresses only.
pub(crate) fn model_status(root: &Path) -> Body {
    match workspace::provider_status(root) {
        Ok(providers) => serde_json::json!({
            "ok": true,
            "providers": providers,
            "real_model_available": providers.iter().any(|p| p.real_model && p.health == "healthy"),
            "config_path": root.join(workspace::MODEL_CONFIG_FILE).display().to_string(),
        }),
        Err(error) => serde_json::json!({ "ok": false, "error": error.to_string() }),
    }
}

/// Handle one `/api/workspace/...` POST that `ui.rs` does not know. Returns
/// `None` for an unknown path so the caller can report "not found".
pub(crate) fn workspace_post(path: &str, body: &Body, root: &Path) -> Option<Body> {
    let handled = match path {
        "/api/workspace/profile" => mutate(root, |store| {
            let input: workspace::VentureProfileInput = serde_json::from_value(body.clone())
                .map_err(|error| WorkspaceError::Invalid(format!("profile: {error}")))?;
            store.update_venture_profile(input)
        }),
        "/api/workspace/customer/update" => mutate(root, |store| {
            let input: workspace::CustomerInput = serde_json::from_value(body.clone())
                .map_err(|error| WorkspaceError::Invalid(format!("customer: {error}")))?;
            store.update_customer(uuid_field(body, "customer_id")?, input)
        }),
        "/api/workspace/document/update" => mutate(root, |store| {
            store.update_document(
                uuid_field(body, "document_id")?,
                str_field(body, "title")?,
                str_field(body, "body")?,
                optional_amount(body)?,
            )
        }),
        "/api/workspace/offer/accepted" => mutate(root, |store| {
            store.record_offer_accepted(uuid_field(body, "document_id")?)
        }),
        "/api/workspace/project" => mutate(root, |store| {
            store.add_project(
                uuid_field(body, "customer_id")?,
                str_field(body, "name")?,
                optional_uuid(body, "offer_id")?,
                optional_amount_named(body, "budget")?,
            )
        }),
        "/api/workspace/project/status" => mutate(root, |store| {
            let status = match str_field(body, "status")? {
                "proposed" => ProjectStatus::Proposed,
                "active" => ProjectStatus::Active,
                "done" => ProjectStatus::Done,
                other => {
                    return Err(WorkspaceError::Invalid(format!(
                        "unknown project status {other}"
                    )))
                }
            };
            store.set_project_status(uuid_field(body, "project_id")?, status)
        }),
        "/api/workspace/task" => mutate(root, |store| {
            store.add_task(
                uuid_field(body, "project_id")?,
                str_field(body, "title")?,
                time_field(body, "due_at")?,
            )
        }),
        "/api/workspace/task/done" => mutate(root, |store| {
            store.complete_task(uuid_field(body, "task_id")?)
        }),
        "/api/workspace/follow-up" => mutate(root, |store| {
            let due_at = time_field(body, "due_at")?
                .ok_or_else(|| WorkspaceError::Invalid("due_at is required".into()))?;
            store.add_follow_up(
                uuid_field(body, "customer_id")?,
                due_at,
                str_field(body, "note")?,
            )
        }),
        "/api/workspace/follow-up/done" => mutate(root, |store| {
            store.complete_follow_up(uuid_field(body, "follow_up_id")?)
        }),
        "/api/workspace/payment" => mutate(root, |store| {
            let received_at =
                time_field(body, "received_at")?.unwrap_or_else(|| chrono::Utc::now().timestamp());
            store.record_payment(
                uuid_field(body, "invoice_id")?,
                workspace::parse_amount_cents(str_field(body, "amount")?)?,
                received_at,
                body.get("note").and_then(|v| v.as_str()).unwrap_or(""),
            )
        }),
        "/api/workspace/receivables" => read(root, |store| {
            store
                .receivables()
                .map(|rows| serde_json::json!({ "ok": true, "receivables": rows }))
        }),
        "/api/workspace/timeline" => read(root, |store| {
            store
                .customer_timeline(uuid_field(body, "customer_id")?)
                .map(|rows| serde_json::json!({ "ok": true, "timeline": rows }))
        }),
        _ => return None,
    };
    Some(handled)
}

fn mutate(
    root: &Path,
    op: impl FnOnce(&workspace::Store) -> Result<workspace::Workspace, WorkspaceError>,
) -> Body {
    match workspace::Store::open(root).and_then(|store| op(&store)) {
        Ok(state) => serde_json::json!({ "ok": true, "workspace": state }),
        Err(error) => serde_json::json!({ "ok": false, "error": error.to_string() }),
    }
}

fn read(root: &Path, op: impl FnOnce(&workspace::Store) -> Result<Body, WorkspaceError>) -> Body {
    match workspace::Store::open(root).and_then(|store| op(&store)) {
        Ok(value) => value,
        Err(error) => serde_json::json!({ "ok": false, "error": error.to_string() }),
    }
}

fn optional_uuid(body: &Body, field: &str) -> Result<Option<Uuid>, WorkspaceError> {
    match body.get(field).and_then(|value| value.as_str()) {
        Some(text) if !text.trim().is_empty() => text
            .parse()
            .map(Some)
            .map_err(|_| WorkspaceError::Invalid(format!("{field} must be a UUID"))),
        _ => Ok(None),
    }
}

/// `amount` as a decimal string; empty or absent means none.
fn optional_amount(body: &Body) -> Result<Option<u64>, WorkspaceError> {
    optional_amount_named(body, "amount")
}

fn optional_amount_named(body: &Body, field: &str) -> Result<Option<u64>, WorkspaceError> {
    match body.get(field).and_then(|value| value.as_str()) {
        Some(text) if !text.trim().is_empty() => workspace::parse_amount_cents(text).map(Some),
        _ => Ok(None),
    }
}

/// A point in time as either unix seconds or a `YYYY-MM-DD` date (taken as
/// 00:00 UTC). Absent or empty means none.
pub(crate) fn time_field(body: &Body, field: &str) -> Result<Option<i64>, WorkspaceError> {
    match body.get(field) {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::Number(number)) => number
            .as_i64()
            .map(Some)
            .ok_or_else(|| WorkspaceError::Invalid(format!("{field} must be unix seconds"))),
        Some(serde_json::Value::String(text)) if text.trim().is_empty() => Ok(None),
        Some(serde_json::Value::String(text)) => {
            let date = chrono::NaiveDate::parse_from_str(text.trim(), "%Y-%m-%d")
                .map_err(|_| WorkspaceError::Invalid(format!("{field} must be YYYY-MM-DD")))?;
            Ok(Some(
                date.and_hms_opt(0, 0, 0)
                    .expect("midnight exists")
                    .and_utc()
                    .timestamp(),
            ))
        }
        Some(_) => Err(WorkspaceError::Invalid(format!(
            "{field} has the wrong type"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_field_accepts_unix_seconds_and_dates_only() {
        let body = serde_json::json!({ "a": 1_700_000_000, "b": "2026-09-10", "c": "", "d": "soon", "e": true });
        assert_eq!(time_field(&body, "a").unwrap(), Some(1_700_000_000));
        assert_eq!(time_field(&body, "b").unwrap(), Some(1_788_998_400));
        assert_eq!(time_field(&body, "c").unwrap(), None);
        assert_eq!(time_field(&body, "missing").unwrap(), None);
        assert!(time_field(&body, "d").is_err());
        assert!(time_field(&body, "e").is_err());
    }

    #[test]
    fn unknown_paths_are_not_claimed() {
        let dir = tempfile::tempdir().unwrap();
        assert!(
            workspace_post("/api/workspace/nope", &serde_json::json!({}), dir.path()).is_none()
        );
        let response = workspace_post(
            "/api/workspace/receivables",
            &serde_json::json!({}),
            dir.path(),
        )
        .unwrap();
        assert_eq!(response["ok"], true);
        assert_eq!(response["receivables"].as_array().unwrap().len(), 0);
    }
}

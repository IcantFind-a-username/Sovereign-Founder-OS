use super::*;
use tempfile::tempdir;
use uuid::Uuid;

fn count_workspace_entries(path: &std::path::Path) -> usize {
    std::fs::read_dir(path)
        .map(|entries| entries.filter_map(|entry| entry.ok()).count())
        .unwrap_or(0)
}

#[test]
fn indeterminate_execution_records_are_surfaced_on_open() {
    let dir = tempdir().unwrap();
    let execution_id = Uuid::new_v4();
    let journal =
        sovereign_execution::ExecutionJournal::open(dir.path().join("executions")).unwrap();
    {
        let _guard = journal
            .begin(sovereign_execution::ExecutionIntent {
                execution_id,
                component_digest_hex: "aa".repeat(32),
                canonical_input_digest_hex: "bb".repeat(32),
                requested_at_unix: 1_800_000_000,
            })
            .unwrap();
    }

    let outbox_before = count_workspace_entries(&dir.path().join("outbox"));
    let authority_before = count_workspace_entries(&dir.path().join("authority"));

    let store = Store::open(dir.path()).unwrap();

    assert_eq!(
        count_workspace_entries(&dir.path().join("outbox")),
        outbox_before,
        "open must not write to the outbox"
    );
    assert_eq!(
        count_workspace_entries(&dir.path().join("authority")),
        authority_before,
        "open must not touch the authority store"
    );

    let report = store.integrity_check().unwrap();
    assert!(
        report.ok,
        "indeterminate journal records are warnings, not critical findings: {:?}",
        report.findings
    );
    assert!(
        report.findings.iter().any(|finding| {
            finding.severity == "warning"
                && finding.resource == format!("execution:{execution_id}")
                && finding.detail.contains("indeterminate")
        }),
        "expected an indeterminate execution warning, got {:?}",
        report.findings
    );
}

#[test]
fn recover_is_a_no_op_on_a_clean_journal() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    let report = store.integrity_check().unwrap();
    assert!(
        report
            .findings
            .iter()
            .all(|finding| !finding.resource.starts_with("execution:")),
        "unexpected execution journal findings: {:?}",
        report.findings
    );

    let reopened = Store::open(dir.path()).unwrap();
    let report = reopened.integrity_check().unwrap();
    assert!(
        report
            .findings
            .iter()
            .all(|finding| !finding.resource.starts_with("execution:"))
    );
}

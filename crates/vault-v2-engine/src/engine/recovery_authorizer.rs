//! Recovery read-only SQL authorizer (Program 1A Task 2).
//!
//! Installed after the database is keyed and profiled. Denies writes, schema
//! changes, attach/detach, transactions, and pragmas outside a fixed read set.

use rusqlite::hooks::{AuthAction, AuthContext, Authorization};
use rusqlite::Connection;

/// Apply `query_only=ON` and the closed recovery authorizer.
pub(crate) fn harden_recovery_connection(
    connection: &Connection,
) -> Result<(), RecoveryAuthorizerError> {
    connection
        .pragma_update(None, "query_only", true)
        .map_err(|_| RecoveryAuthorizerError)?;
    connection
        .authorizer(Some(recovery_authorizer_callback))
        .map_err(|_| RecoveryAuthorizerError)?;
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RecoveryAuthorizerError;

fn recovery_authorizer_callback(context: AuthContext<'_>) -> Authorization {
    let action = context.action;
    match action {
        AuthAction::Select => Authorization::Allow,
        AuthAction::Read { table_name, .. }
            if table_name == "sqlite_schema" || table_name == "sqlite_master" =>
        {
            Authorization::Allow
        }
        AuthAction::Pragma {
            pragma_name,
            pragma_value,
        } => {
            if recovery_pragma_allowed(pragma_name, pragma_value) {
                Authorization::Allow
            } else {
                Authorization::Deny
            }
        }
        AuthAction::Insert { .. }
        | AuthAction::Update { .. }
        | AuthAction::Delete { .. }
        | AuthAction::CreateTable { .. }
        | AuthAction::CreateIndex { .. }
        | AuthAction::CreateView { .. }
        | AuthAction::CreateTrigger { .. }
        | AuthAction::CreateTempTable { .. }
        | AuthAction::CreateTempIndex { .. }
        | AuthAction::CreateTempView { .. }
        | AuthAction::CreateTempTrigger { .. }
        | AuthAction::DropTable { .. }
        | AuthAction::DropIndex { .. }
        | AuthAction::DropView { .. }
        | AuthAction::DropTrigger { .. }
        | AuthAction::DropTempTable { .. }
        | AuthAction::DropTempIndex { .. }
        | AuthAction::DropTempView { .. }
        | AuthAction::DropTempTrigger { .. }
        | AuthAction::Attach { .. }
        | AuthAction::Detach { .. }
        | AuthAction::AlterTable { .. }
        | AuthAction::Reindex { .. }
        | AuthAction::Analyze { .. }
        | AuthAction::CreateVtable { .. }
        | AuthAction::DropVtable { .. }
        | AuthAction::Transaction { .. }
        | AuthAction::Savepoint { .. }
        | AuthAction::Recursive => Authorization::Deny,
        AuthAction::Function { .. } => Authorization::Allow,
        AuthAction::Read { .. } => Authorization::Deny,
        AuthAction::Unknown { .. } => Authorization::Deny,
        _ => Authorization::Deny,
    }
}

fn recovery_pragma_allowed(name: &str, value: Option<&str>) -> bool {
    let lower = name.to_ascii_lowercase();
    match lower.as_str() {
        "query_only" => !value_is_off(value),
        "cipher_integrity_check"
        | "cipher_provider"
        | "cipher_provider_version"
        | "cipher_version"
        | "integrity_check" => true,
        _ => false,
    }
}

fn value_is_off(value: Option<&str>) -> bool {
    matches!(
        value.map(str::trim),
        Some("0") | Some("false") | Some("FALSE") | Some("off") | Some("OFF")
    )
}

/// Read back `PRAGMA query_only` as SQLite reports it.
pub(crate) fn query_only_is_on(connection: &Connection) -> Result<bool, RecoveryAuthorizerError> {
    let value: i64 = connection
        .pragma_query_value(None, "query_only", |row| row.get(0))
        .map_err(|_| RecoveryAuthorizerError)?;
    Ok(value != 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::process::bootstrap_crypto_process;
    use crate::engine::secret::DbKey;
    use crate::engine::sqlcipher::{open_sqlcipher, ConnectionMode};
    use rusqlite::OptionalExtension;

    fn owner() -> &'static crate::engine::process::CryptoProcessOwner {
        bootstrap_crypto_process().expect("openssl")
    }

    fn readonly_encrypted_db() -> (tempfile::NamedTempFile, DbKey) {
        let dbk = DbKey::from_bytes([0x5c; 32]);
        let file = tempfile::NamedTempFile::new().expect("temp db");
        open_sqlcipher(
            owner(),
            file.path(),
            &dbk,
            ConnectionMode::ReadWriteCreateInternal,
        )
        .expect("create");
        (file, dbk)
    }

    #[test]
    fn recovery_hardening_sets_query_only_and_db_readonly() {
        let (file, dbk) = readonly_encrypted_db();
        let connection =
            open_sqlcipher(owner(), file.path(), &dbk, ConnectionMode::ReadOnlyRecovery)
                .expect("open");
        harden_recovery_connection(connection.rusqlite_connection()).expect("harden");
        assert!(connection.is_db_readonly().expect("readonly flag"));
        assert!(query_only_is_on(connection.rusqlite_connection()).expect("query_only"));
    }

    #[test]
    fn recovery_authorizer_denies_insert_and_clears_query_only_pragma() {
        let (file, dbk) = readonly_encrypted_db();
        let connection =
            open_sqlcipher(owner(), file.path(), &dbk, ConnectionMode::ReadOnlyRecovery)
                .expect("open");
        harden_recovery_connection(connection.rusqlite_connection()).expect("harden");
        let conn = connection.rusqlite_connection();
        assert!(conn
            .execute("INSERT INTO sqlite_schema VALUES (1,2,3,4)", [])
            .is_err());
        assert!(conn.pragma_update(None, "query_only", false).is_err());
        assert!(conn.pragma_update(None, "journal_mode", "WAL").is_err());
    }

    #[test]
    fn recovery_authorizer_allows_schema_probe_and_cipher_integrity() {
        let (file, dbk) = readonly_encrypted_db();
        let connection =
            open_sqlcipher(owner(), file.path(), &dbk, ConnectionMode::ReadOnlyRecovery)
                .expect("open");
        harden_recovery_connection(connection.rusqlite_connection()).expect("harden");
        let conn = connection.rusqlite_connection();
        let _: Option<String> = conn
            .query_row("SELECT name FROM sqlite_schema LIMIT 1", [], |row| {
                row.get(0)
            })
            .optional()
            .expect("schema probe");
        let _ = conn
            .prepare("PRAGMA cipher_integrity_check")
            .expect("cipher integrity pragma");
    }
}

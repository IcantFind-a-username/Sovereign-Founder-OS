//! Closed transactional business schema (RFC 0005 Program 1A Task 3).
//!
//! Fixed DDL, sealed object adapters, prepared-parameter transactions, and a
//! closed SQL authorizer. No string type tags, generic blob writes, or caller SQL.

use super::sqlcipher::HardenedConnection;
use rusqlite::hooks::{AuthAction, AuthContext, Authorization};
use rusqlite::params;
use rusqlite::Connection;
use std::fmt;
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

/// SQLite `application_id` for SFOS (`0x53464f53`).
pub(crate) const APPLICATION_ID: i32 = 0x5346_4f53_u32 as i32;

pub(crate) const USER_VERSION: i32 = 2;
pub(crate) const FORMAT_VERSION: i32 = 2;
pub(crate) const REGISTRY_VERSION: i32 = 1;
pub(crate) const KEY_EPOCH: i32 = 1;
pub(crate) const BACKUP_ELIGIBLE: i32 = 1;

pub(crate) const MAX_CHUNKS_PER_OBJECT: i32 = 64;
pub(crate) const MAX_CHUNK_BYTES: i32 = 4 * 1024 * 1024;
pub(crate) const MAX_OBJECT_BYTES: i64 = 268_435_456;

static DDL_VAULT_METADATA: &str = r#"
CREATE TABLE vault_metadata_v1 (
    singleton INTEGER NOT NULL UNIQUE CHECK (singleton = 1),
    workspace_id BLOB NOT NULL
        CHECK (typeof(workspace_id) = 'blob' AND length(workspace_id) = 32),
    database_id BLOB NOT NULL
        CHECK (typeof(database_id) = 'blob' AND length(database_id) = 32),
    format_version INTEGER NOT NULL CHECK (format_version = 2),
    registry_version INTEGER NOT NULL CHECK (registry_version = 1),
    key_epoch INTEGER NOT NULL CHECK (key_epoch = 1),
    PRIMARY KEY (workspace_id, database_id)
) STRICT, WITHOUT ROWID;
"#;

static DDL_BUSINESS_OBJECT: &str = r#"
CREATE TABLE business_object_v1 (
    workspace_id BLOB NOT NULL,
    database_id BLOB NOT NULL,
    object_id BLOB NOT NULL
        CHECK (typeof(object_id) = 'blob' AND length(object_id) = 32),
    object_type INTEGER NOT NULL CHECK (object_type IN (1, 2)),
    backup_disposition INTEGER NOT NULL CHECK (backup_disposition = 1),
    revision INTEGER NOT NULL CHECK (revision >= 1),
    chunk_count INTEGER NOT NULL CHECK (chunk_count BETWEEN 1 AND 64),
    byte_count INTEGER NOT NULL CHECK (byte_count BETWEEN 0 AND 268435456),
    PRIMARY KEY (workspace_id, database_id, object_id),
    FOREIGN KEY (workspace_id, database_id)
        REFERENCES vault_metadata_v1(workspace_id, database_id)
        ON UPDATE NO ACTION ON DELETE NO ACTION
) STRICT, WITHOUT ROWID;
"#;

static DDL_BUSINESS_CHUNK: &str = r#"
CREATE TABLE business_chunk_v1 (
    workspace_id BLOB NOT NULL,
    database_id BLOB NOT NULL,
    object_id BLOB NOT NULL,
    chunk_index INTEGER NOT NULL CHECK (chunk_index BETWEEN 0 AND 63),
    chunk_bytes BLOB NOT NULL
        CHECK (typeof(chunk_bytes) = 'blob' AND length(chunk_bytes) <= 4194304),
    PRIMARY KEY (workspace_id, database_id, object_id, chunk_index),
    FOREIGN KEY (workspace_id, database_id, object_id)
        REFERENCES business_object_v1(workspace_id, database_id, object_id)
        ON UPDATE NO ACTION ON DELETE NO ACTION
) STRICT, WITHOUT ROWID;
"#;

/// External binding checked on every initialized vault open.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct VaultSchemaBinding {
    pub workspace_id: [u8; 32],
    pub database_id: [u8; 32],
    pub db_key_epoch: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SchemaError {
    Rejected,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ObjectTypeTag {
    BusinessStateV1 = 1,
    VentureProfileV1 = 2,
}

impl ObjectTypeTag {
    fn backup_disposition(self) -> i32 {
        BACKUP_ELIGIBLE
    }

    fn as_i32(self) -> i32 {
        self as i32
    }
}

/// Sealed adapter for the workspace graph object.
pub(crate) struct BusinessStateV1 {
    pub(crate) object_id: [u8; 32],
    pub(crate) revision: i32,
    pub(crate) chunks: Vec<ZeroizingChunk>,
}

/// Sealed adapter for the venture profile object.
pub(crate) struct VentureProfileV1 {
    pub(crate) object_id: [u8; 32],
    pub(crate) revision: i32,
    pub(crate) chunks: Vec<ZeroizingChunk>,
}

#[derive(Zeroize, ZeroizeOnDrop)]
pub(crate) struct ZeroizingChunk(Zeroizing<Vec<u8>>);

impl ZeroizingChunk {
    pub(crate) fn from_bytes(bytes: Vec<u8>) -> Self {
        Self(Zeroizing::new(bytes))
    }

    pub(crate) fn as_slice(&self) -> &[u8] {
        &self.0
    }
}

pub(crate) fn vault_application_profile_is_active(
    connection: &Connection,
) -> Result<bool, SchemaError> {
    let app_id: i32 = connection
        .pragma_query_value(None, "application_id", |row| row.get(0))
        .map_err(|_| SchemaError::Rejected)?;
    let user_version: i32 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(|_| SchemaError::Rejected)?;
    Ok(app_id == APPLICATION_ID && user_version == USER_VERSION)
}

pub(crate) fn initialize_vault_schema(
    hardened: &HardenedConnection,
    binding: &VaultSchemaBinding,
) -> Result<(), SchemaError> {
    if binding.db_key_epoch != 1 {
        return Err(SchemaError::Rejected);
    }
    let connection = hardened.rusqlite_connection();
    connection
        .execute_batch("BEGIN IMMEDIATE;")
        .map_err(|_| SchemaError::Rejected)?;
    let init = |connection: &Connection| -> Result<(), SchemaError> {
        connection
            .pragma_update(None, "application_id", APPLICATION_ID)
            .map_err(|_| SchemaError::Rejected)?;
        connection
            .pragma_update(None, "user_version", USER_VERSION)
            .map_err(|_| SchemaError::Rejected)?;
        connection
            .execute_batch(DDL_VAULT_METADATA)
            .map_err(|_| SchemaError::Rejected)?;
        connection
            .execute_batch(DDL_BUSINESS_OBJECT)
            .map_err(|_| SchemaError::Rejected)?;
        connection
            .execute_batch(DDL_BUSINESS_CHUNK)
            .map_err(|_| SchemaError::Rejected)?;
        connection
            .execute(
                "INSERT INTO vault_metadata_v1 \
                 (singleton, workspace_id, database_id, format_version, registry_version, key_epoch) \
                 VALUES (1, ?1, ?2, ?3, ?4, ?5)",
                params![
                    binding.workspace_id.as_slice(),
                    binding.database_id.as_slice(),
                    FORMAT_VERSION,
                    REGISTRY_VERSION,
                    KEY_EPOCH,
                ],
            )
            .map_err(|_| SchemaError::Rejected)?;
        Ok(())
    };
    if init(connection).is_err() {
        let _ = connection.execute_batch("ROLLBACK;");
        return Err(SchemaError::Rejected);
    }
    connection
        .execute_batch("COMMIT;")
        .map_err(|_| SchemaError::Rejected)?;
    verify_vault_schema_on_open(connection, binding)
}

pub(crate) fn verify_vault_schema_on_open(
    connection: &Connection,
    binding: &VaultSchemaBinding,
) -> Result<(), SchemaError> {
    if !vault_application_profile_is_active(connection)? {
        return Err(SchemaError::Rejected);
    }
    if binding.db_key_epoch != 1 {
        return Err(SchemaError::Rejected);
    }
    let (workspace_id, database_id, format_version, registry_version, key_epoch): (
        Vec<u8>,
        Vec<u8>,
        i32,
        i32,
        i32,
    ) = connection
        .query_row(
            "SELECT workspace_id, database_id, format_version, registry_version, key_epoch \
             FROM vault_metadata_v1 WHERE singleton = 1",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .map_err(|_| SchemaError::Rejected)?;
    if workspace_id.len() != 32
        || database_id.len() != 32
        || workspace_id.as_slice() != binding.workspace_id
        || database_id.as_slice() != binding.database_id
        || format_version != FORMAT_VERSION
        || registry_version != REGISTRY_VERSION
        || key_epoch != KEY_EPOCH
    {
        return Err(SchemaError::Rejected);
    }
    verify_ddl_shape(connection)?;
    Ok(())
}

fn verify_ddl_shape(connection: &Connection) -> Result<(), SchemaError> {
    let tables: Vec<String> = connection
        .prepare(
            "SELECT name FROM sqlite_schema \
             WHERE type = 'table' AND name IN ('vault_metadata_v1', 'business_object_v1', 'business_chunk_v1') \
             ORDER BY name",
        )
        .map_err(|_| SchemaError::Rejected)?
        .query_map([], |row| row.get(0))
        .map_err(|_| SchemaError::Rejected)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| SchemaError::Rejected)?;
    if tables
        != [
            "business_chunk_v1",
            "business_object_v1",
            "vault_metadata_v1",
        ]
    {
        return Err(SchemaError::Rejected);
    }
    Ok(())
}

pub(crate) fn install_business_authorizer(connection: &Connection) -> Result<(), SchemaError> {
    connection
        .authorizer(Some(business_authorizer_callback))
        .map_err(|_| SchemaError::Rejected)
}

fn business_table_allowed(name: &str) -> bool {
    matches!(
        name,
        "vault_metadata_v1" | "business_object_v1" | "business_chunk_v1"
    )
}

fn business_authorizer_callback(context: AuthContext<'_>) -> Authorization {
    match context.action {
        AuthAction::Select => Authorization::Allow,
        AuthAction::Read { table_name, .. } if business_table_allowed(table_name) => {
            Authorization::Allow
        }
        AuthAction::Insert { table_name, .. }
        | AuthAction::Update { table_name, .. }
        | AuthAction::Delete { table_name, .. }
            if business_table_allowed(table_name) =>
        {
            Authorization::Allow
        }
        AuthAction::Function { function_name, .. } => {
            if is_forbidden_function(function_name) {
                Authorization::Deny
            } else {
                Authorization::Allow
            }
        }
        AuthAction::Pragma { pragma_name, .. } => {
            let lower = pragma_name.to_ascii_lowercase();
            if matches!(
                lower.as_str(),
                "foreign_keys" | "query_only" | "journal_mode" | "synchronous" | "temp_store"
            ) {
                Authorization::Allow
            } else {
                Authorization::Deny
            }
        }
        AuthAction::Transaction { .. } => Authorization::Allow,
        _ => Authorization::Deny,
    }
}

fn is_forbidden_function(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "load_extension" | "sqlcipher_export" | "attach"
    )
}

pub(crate) struct BusinessTransaction<'a> {
    connection: &'a Connection,
    binding: VaultSchemaBinding,
}

impl<'a> BusinessTransaction<'a> {
    pub(crate) fn begin(
        hardened: &'a HardenedConnection,
        binding: VaultSchemaBinding,
    ) -> Result<Self, SchemaError> {
        let connection = hardened.rusqlite_connection();
        connection
            .execute_batch("BEGIN IMMEDIATE;")
            .map_err(|_| SchemaError::Rejected)?;
        Ok(Self {
            connection,
            binding,
        })
    }

    fn insert_object(
        &self,
        object_id: [u8; 32],
        tag: ObjectTypeTag,
        revision: i32,
        chunks: &[ZeroizingChunk],
    ) -> Result<(), SchemaError> {
        if chunks.is_empty() || chunks.len() > MAX_CHUNKS_PER_OBJECT as usize {
            return Err(SchemaError::Rejected);
        }
        let mut byte_count: u64 = 0;
        for chunk in chunks {
            let len = chunk.as_slice().len();
            if len > MAX_CHUNK_BYTES as usize {
                return Err(SchemaError::Rejected);
            }
            byte_count = byte_count
                .checked_add(len as u64)
                .ok_or(SchemaError::Rejected)?;
            if byte_count > MAX_OBJECT_BYTES as u64 {
                return Err(SchemaError::Rejected);
            }
        }
        self.connection
            .execute(
                "INSERT INTO business_object_v1 \
                 (workspace_id, database_id, object_id, object_type, backup_disposition, revision, chunk_count, byte_count) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    self.binding.workspace_id.as_slice(),
                    self.binding.database_id.as_slice(),
                    object_id.as_slice(),
                    tag.as_i32(),
                    tag.backup_disposition(),
                    revision,
                    i32::try_from(chunks.len()).map_err(|_| SchemaError::Rejected)?,
                    i64::try_from(byte_count).map_err(|_| SchemaError::Rejected)?,
                ],
            )
            .map_err(|_| SchemaError::Rejected)?;
        for (index, chunk) in chunks.iter().enumerate() {
            self.connection
                .execute(
                    "INSERT INTO business_chunk_v1 \
                     (workspace_id, database_id, object_id, chunk_index, chunk_bytes) \
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![
                        self.binding.workspace_id.as_slice(),
                        self.binding.database_id.as_slice(),
                        object_id.as_slice(),
                        i32::try_from(index).map_err(|_| SchemaError::Rejected)?,
                        chunk.as_slice(),
                    ],
                )
                .map_err(|_| SchemaError::Rejected)?;
        }
        Ok(())
    }

    pub(crate) fn put_business_state_v1(
        &self,
        object: &BusinessStateV1,
    ) -> Result<(), SchemaError> {
        self.insert_object(
            object.object_id,
            ObjectTypeTag::BusinessStateV1,
            object.revision,
            &object.chunks,
        )
    }

    pub(crate) fn put_venture_profile_v1(
        &self,
        object: &VentureProfileV1,
    ) -> Result<(), SchemaError> {
        self.insert_object(
            object.object_id,
            ObjectTypeTag::VentureProfileV1,
            object.revision,
            &object.chunks,
        )
    }

    pub(crate) fn delete_object(&self, object_id: [u8; 32]) -> Result<(), SchemaError> {
        let chunk_count: i32 = self
            .connection
            .query_row(
                "SELECT chunk_count FROM business_object_v1 \
                 WHERE workspace_id = ?1 AND database_id = ?2 AND object_id = ?3",
                params![
                    self.binding.workspace_id.as_slice(),
                    self.binding.database_id.as_slice(),
                    object_id.as_slice(),
                ],
                |row| row.get(0),
            )
            .map_err(|_| SchemaError::Rejected)?;
        let deleted = self
            .connection
            .execute(
                "DELETE FROM business_chunk_v1 \
                 WHERE workspace_id = ?1 AND database_id = ?2 AND object_id = ?3",
                params![
                    self.binding.workspace_id.as_slice(),
                    self.binding.database_id.as_slice(),
                    object_id.as_slice(),
                ],
            )
            .map_err(|_| SchemaError::Rejected)?;
        if i32::try_from(deleted).map_err(|_| SchemaError::Rejected)? != chunk_count {
            return Err(SchemaError::Rejected);
        }
        let parent_deleted = self
            .connection
            .execute(
                "DELETE FROM business_object_v1 \
                 WHERE workspace_id = ?1 AND database_id = ?2 AND object_id = ?3",
                params![
                    self.binding.workspace_id.as_slice(),
                    self.binding.database_id.as_slice(),
                    object_id.as_slice(),
                ],
            )
            .map_err(|_| SchemaError::Rejected)?;
        if parent_deleted != 1 {
            return Err(SchemaError::Rejected);
        }
        Ok(())
    }

    pub(crate) fn commit(self) -> Result<(), SchemaError> {
        self.connection
            .execute_batch("COMMIT;")
            .map_err(|_| SchemaError::Rejected)
    }

    pub(crate) fn rollback(self) -> Result<(), SchemaError> {
        self.connection
            .execute_batch("ROLLBACK;")
            .map_err(|_| SchemaError::Rejected)
    }
}

pub(crate) struct BusinessReadSession<'a> {
    connection: &'a Connection,
    binding: VaultSchemaBinding,
}

impl<'a> BusinessReadSession<'a> {
    pub(crate) fn new(
        hardened: &'a HardenedConnection,
        binding: VaultSchemaBinding,
    ) -> Result<Self, SchemaError> {
        Ok(Self {
            connection: hardened.rusqlite_connection(),
            binding,
        })
    }

    pub(crate) fn read_object(
        &self,
        object_id: [u8; 32],
    ) -> Result<Zeroizing<Vec<u8>>, SchemaError> {
        let (chunk_count, byte_count): (i32, i64) = self
            .connection
            .query_row(
                "SELECT chunk_count, byte_count FROM business_object_v1 \
                 WHERE workspace_id = ?1 AND database_id = ?2 AND object_id = ?3",
                params![
                    self.binding.workspace_id.as_slice(),
                    self.binding.database_id.as_slice(),
                    object_id.as_slice(),
                ],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|_| SchemaError::Rejected)?;
        let mut out: Vec<u8> = Vec::new();
        for index in 0..chunk_count {
            let chunk: Vec<u8> = self
                .connection
                .query_row(
                    "SELECT chunk_bytes FROM business_chunk_v1 \
                     WHERE workspace_id = ?1 AND database_id = ?2 AND object_id = ?3 AND chunk_index = ?4",
                    params![
                        self.binding.workspace_id.as_slice(),
                        self.binding.database_id.as_slice(),
                        object_id.as_slice(),
                        index,
                    ],
                    |row| row.get(0),
                )
                .map_err(|_| SchemaError::Rejected)?;
            if chunk.len() > MAX_CHUNK_BYTES as usize {
                return Err(SchemaError::Rejected);
            }
            let next_len = out
                .len()
                .checked_add(chunk.len())
                .ok_or(SchemaError::Rejected)?;
            if i64::try_from(next_len).map_err(|_| SchemaError::Rejected)? > byte_count {
                return Err(SchemaError::Rejected);
            }
            out.extend_from_slice(&chunk);
        }
        if i64::try_from(out.len()).map_err(|_| SchemaError::Rejected)? != byte_count {
            return Err(SchemaError::Rejected);
        }
        Ok(Zeroizing::new(out))
    }
}

impl fmt::Debug for BusinessStateV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BusinessStateV1")
            .field("object_id", &"[redacted]")
            .field("revision", &self.revision)
            .field("chunks", &self.chunks.len())
            .finish()
    }
}

impl fmt::Debug for VentureProfileV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VentureProfileV1")
            .field("object_id", &"[redacted]")
            .field("revision", &self.revision)
            .field("chunks", &self.chunks.len())
            .finish()
    }
}

#[cfg(test)]
mod tests;

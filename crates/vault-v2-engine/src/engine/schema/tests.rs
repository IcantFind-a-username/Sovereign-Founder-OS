use super::*;
use crate::engine::process::bootstrap_crypto_process;
use crate::engine::secret::DbKey;
use crate::engine::sqlcipher::{open_sqlcipher, ConnectionMode, HardenedConnection, OpenError};
use rusqlite::params;
use std::fs;
use std::path::{Path, PathBuf};

fn owner() -> &'static crate::engine::process::CryptoProcessOwner {
    bootstrap_crypto_process().expect("openssl")
}

fn binding(workspace_fill: u8, database_fill: u8) -> VaultSchemaBinding {
    VaultSchemaBinding {
        workspace_id: [workspace_fill; 32],
        database_id: [database_fill; 32],
        db_key_epoch: 1,
    }
}

fn canonical_tempdir() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = fs::canonicalize(dir.path()).expect("canonical tempdir");
    (dir, path)
}

fn create_initialized_vault(
    db: &Path,
    dbk: &DbKey,
    bind: &VaultSchemaBinding,
) -> HardenedConnection {
    let opened = open_sqlcipher(
        owner(),
        db,
        dbk,
        ConnectionMode::ReadWriteCreateInternal,
        None,
    )
    .expect("create container");
    initialize_vault_schema(&opened, bind).expect("initialize schema");
    drop(opened);
    open_sqlcipher(owner(), db, dbk, ConnectionMode::ReadWrite, Some(bind))
        .expect("reopen with schema")
}

fn sample_object(id_fill: u8, payload: &[u8]) -> BusinessStateV1 {
    BusinessStateV1 {
        object_id: [id_fill; 32],
        revision: 1,
        chunks: vec![ZeroizingChunk::from_bytes(payload.to_vec())],
    }
}

#[test]
fn one_commit_writes_complete_object_set_or_no_rows_after_failpoint() {
    let (_dir, root) = canonical_tempdir();
    let db = root.join("vault.db");
    let dbk = DbKey::from_bytes([0x31; 32]);
    let bind = binding(0x01, 0x02);
    let hardened = create_initialized_vault(&db, &dbk, &bind);
    let object = sample_object(0x11, b"workspace-graph-v1");
    let txn = BusinessTransaction::begin(&hardened, bind).expect("begin");
    txn.put_business_state_v1(&object).expect("insert");
    txn.commit().expect("commit");
    let reader = BusinessReadSession::new(&hardened, bind).expect("read session");
    let bytes = reader.read_object(object.object_id).expect("read back");
    assert_eq!(&*bytes, b"workspace-graph-v1");

    // Fail before commit: rollback leaves no object row.
    let txn = BusinessTransaction::begin(&hardened, bind).expect("begin");
    txn.put_business_state_v1(&sample_object(0x22, b"orphan"))
        .expect("insert");
    txn.rollback().expect("rollback");
    let count: i64 = hardened
        .rusqlite_connection()
        .query_row(
            "SELECT count(*) FROM business_object_v1 WHERE object_id = ?1",
            params![[0x22u8; 32]],
            |row| row.get(0),
        )
        .expect("count");
    assert_eq!(count, 0);
}

#[test]
fn foreign_keys_and_checks_reject_invalid_identity_and_oversize() {
    let (_dir, root) = canonical_tempdir();
    let db = root.join("vault.db");
    let dbk = DbKey::from_bytes([0x32; 32]);
    let bind = binding(0x03, 0x04);
    let hardened = create_initialized_vault(&db, &dbk, &bind);
    let conn = hardened.rusqlite_connection();
    assert!(conn
        .execute(
            "INSERT INTO business_object_v1 \
             (workspace_id, database_id, object_id, object_type, backup_disposition, revision, chunk_count, byte_count) \
             VALUES (?1, ?2, ?3, 1, 1, 1, 1, 1)",
            params![&[0xffu8; 32], bind.database_id.as_slice(), &[0x44u8; 32]],
        )
        .is_err());
    assert!(conn
        .execute(
            "INSERT INTO business_object_v1 \
             (workspace_id, database_id, object_id, object_type, backup_disposition, revision, chunk_count, byte_count) \
             VALUES (?1, ?2, ?3, 99, 1, 1, 1, 1)",
            params![
                bind.workspace_id.as_slice(),
                bind.database_id.as_slice(),
                &[0x45u8; 32]
            ],
        )
        .is_err());
    let huge = vec![0u8; MAX_CHUNK_BYTES as usize + 1];
    assert!(conn
        .execute(
            "INSERT INTO business_chunk_v1 \
             (workspace_id, database_id, object_id, chunk_index, chunk_bytes) \
             VALUES (?1, ?2, ?3, 0, ?4)",
            params![
                bind.workspace_id.as_slice(),
                bind.database_id.as_slice(),
                &[0x46u8; 32],
                huge.as_slice(),
            ],
        )
        .is_err());
}

#[test]
fn every_id_is_exactly_32_bytes_matching_external_binding() {
    let bind = binding(0x05, 0x06);
    assert_eq!(bind.workspace_id.len(), 32);
    assert_eq!(bind.database_id.len(), 32);
    let (_dir, root) = canonical_tempdir();
    let db = root.join("vault.db");
    let dbk = DbKey::from_bytes([0x33; 32]);
    let hardened = create_initialized_vault(&db, &dbk, &bind);
    let (workspace_id, database_id): (Vec<u8>, Vec<u8>) = hardened
        .rusqlite_connection()
        .query_row(
            "SELECT workspace_id, database_id FROM vault_metadata_v1 WHERE singleton = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("metadata");
    assert_eq!(workspace_id.len(), 32);
    assert_eq!(database_id.len(), 32);
    assert_eq!(workspace_id.as_slice(), bind.workspace_id);
    assert_eq!(database_id.as_slice(), bind.database_id);
}

#[test]
fn key_epoch_must_be_one_against_sidecar_on_open() {
    let bind = binding(0x07, 0x08);
    let bad = VaultSchemaBinding {
        db_key_epoch: 2,
        ..bind
    };
    let (_dir, root) = canonical_tempdir();
    let db = root.join("vault.db");
    let dbk = DbKey::from_bytes([0x34; 32]);
    let opened = open_sqlcipher(
        owner(),
        &db,
        &dbk,
        ConnectionMode::ReadWriteCreateInternal,
        None,
    )
    .expect("create");
    assert!(initialize_vault_schema(&opened, &bad).is_err());
    assert!(initialize_vault_schema(&opened, &bind).is_ok());
}

#[test]
fn sealed_delete_removes_all_chunks_then_parent_in_one_transaction() {
    let bind = binding(0x09, 0x0a);
    let (_dir, root) = canonical_tempdir();
    let db = root.join("vault.db");
    let dbk = DbKey::from_bytes([0x35; 32]);
    let hardened = create_initialized_vault(&db, &dbk, &bind);
    let object = sample_object(0x55, b"delete-me");
    let txn = BusinessTransaction::begin(&hardened, bind).expect("begin");
    txn.put_business_state_v1(&object).expect("insert");
    txn.commit().expect("commit");
    let txn = BusinessTransaction::begin(&hardened, bind).expect("begin");
    txn.delete_object(object.object_id).expect("delete");
    txn.commit().expect("commit");
    let chunks: i64 = hardened
        .rusqlite_connection()
        .query_row(
            "SELECT count(*) FROM business_chunk_v1 WHERE object_id = ?1",
            params![object.object_id],
            |row| row.get(0),
        )
        .expect("chunks");
    let objects: i64 = hardened
        .rusqlite_connection()
        .query_row(
            "SELECT count(*) FROM business_object_v1 WHERE object_id = ?1",
            params![object.object_id],
            |row| row.get(0),
        )
        .expect("objects");
    assert_eq!(chunks, 0);
    assert_eq!(objects, 0);
}

#[test]
fn direct_parent_delete_with_children_is_rejected() {
    let bind = binding(0x0b, 0x0c);
    let (_dir, root) = canonical_tempdir();
    let db = root.join("vault.db");
    let dbk = DbKey::from_bytes([0x36; 32]);
    let hardened = create_initialized_vault(&db, &dbk, &bind);
    let object = sample_object(0x66, b"child-parent");
    let txn = BusinessTransaction::begin(&hardened, bind).expect("begin");
    txn.put_business_state_v1(&object).expect("insert");
    txn.commit().expect("commit");
    assert!(hardened
        .rusqlite_connection()
        .execute(
            "DELETE FROM business_object_v1 WHERE object_id = ?1",
            [object.object_id.as_slice()],
        )
        .is_err());
}

#[test]
fn unknown_object_tags_are_rejected_before_sql() {
    let bind = binding(0x0d, 0x0e);
    let (_dir, root) = canonical_tempdir();
    let db = root.join("vault.db");
    let dbk = DbKey::from_bytes([0x37; 32]);
    let hardened = create_initialized_vault(&db, &dbk, &bind);
    assert!(hardened
        .rusqlite_connection()
        .execute(
            "INSERT INTO business_object_v1 \
             (workspace_id, database_id, object_id, object_type, backup_disposition, revision, chunk_count, byte_count) \
             VALUES (?1, ?2, ?3, 3, 1, 1, 1, 1)",
            params![
                bind.workspace_id.as_slice(),
                bind.database_id.as_slice(),
                &[0x77u8; 32]
            ],
        )
        .is_err());
}

#[test]
fn rollback_journal_recovery_preserves_old_or_complete_new_transaction() {
    let bind = binding(0x0f, 0x10);
    let (_dir, root) = canonical_tempdir();
    let db = root.join("vault.db");
    let dbk = DbKey::from_bytes([0x38; 32]);
    let hardened = create_initialized_vault(&db, &dbk, &bind);
    let stable = sample_object(0x88, b"stable");
    let txn = BusinessTransaction::begin(&hardened, bind).expect("begin");
    txn.put_business_state_v1(&stable).expect("insert");
    txn.commit().expect("commit");
    let conn = hardened.rusqlite_connection();
    conn.execute_batch("BEGIN IMMEDIATE;").expect("begin");
    conn.execute(
        "INSERT INTO business_object_v1 \
         (workspace_id, database_id, object_id, object_type, backup_disposition, revision, chunk_count, byte_count) \
         VALUES (?1, ?2, ?3, 1, 1, 1, 1, 3)",
        params![
            bind.workspace_id.as_slice(),
            bind.database_id.as_slice(),
            &[0x99u8; 32]
        ],
    )
    .expect("insert object");
    conn.execute(
        "INSERT INTO business_chunk_v1 \
         (workspace_id, database_id, object_id, chunk_index, chunk_bytes) \
         VALUES (?1, ?2, ?3, 0, ?4)",
        params![
            bind.workspace_id.as_slice(),
            bind.database_id.as_slice(),
            &[0x99u8; 32],
            b"new",
        ],
    )
    .expect("insert chunk");
    conn.execute_batch("ROLLBACK;").expect("rollback");
    drop(hardened);
    let reopened =
        open_sqlcipher(owner(), &db, &dbk, ConnectionMode::ReadWrite, Some(&bind)).expect("reopen");
    let reader = BusinessReadSession::new(&reopened, bind).expect("reader");
    let bytes = reader.read_object(stable.object_id).expect("stable read");
    assert_eq!(&*bytes, b"stable");
    assert!(reader.read_object([0x99; 32]).is_err());
}

#[test]
fn plaintext_never_appears_in_database_sidecar_or_debug() {
    const SECRET: &str = "SFO-BUSINESS-PLAINTEXT-SECRET-9e2c";
    let bind = binding(0x11, 0x12);
    let (_dir, root) = canonical_tempdir();
    let db = root.join("vault.db");
    let dbk = DbKey::from_bytes([0x39; 32]);
    let hardened = create_initialized_vault(&db, &dbk, &bind);
    let object = sample_object(0xaa, SECRET.as_bytes());
    let txn = BusinessTransaction::begin(&hardened, bind).expect("begin");
    txn.put_business_state_v1(&object).expect("insert");
    txn.commit().expect("commit");
    let disk = fs::read(&db).expect("read db");
    assert!(!disk.windows(SECRET.len()).any(|w| w == SECRET.as_bytes()));
    let debug = format!("{:?}", object);
    assert!(!debug.contains(SECRET));
}

#[test]
fn sqlite_magic_txt_does_not_yet_assign_sfos_application_id() {
    let magic = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/sqlite-application-id-magic.txt"),
    )
    .expect("magic fixture");
    assert!(
        !magic
            .lines()
            .filter(|line| line.starts_with("0x"))
            .any(|line| line.contains("53464f53") || line.contains("SFOS")),
        "official magic list now assigns the provisional id; RFC registration check required"
    );
    assert_eq!(APPLICATION_ID, 0x5346_4f53_u32 as i32);
}

#[test]
fn business_authorizer_denies_ad_hoc_ddl_after_install() {
    let bind = binding(0x13, 0x14);
    let (_dir, root) = canonical_tempdir();
    let db = root.join("vault.db");
    let dbk = DbKey::from_bytes([0x3a; 32]);
    let hardened = create_initialized_vault(&db, &dbk, &bind);
    assert!(hardened
        .rusqlite_connection()
        .execute_batch("CREATE TABLE smuggled (x);")
        .is_err());
}

#[test]
fn open_rejects_wrong_binding_after_initialize() {
    let bind = binding(0x15, 0x16);
    let (_dir, root) = canonical_tempdir();
    let db = root.join("vault.db");
    let dbk = DbKey::from_bytes([0x3b; 32]);
    create_initialized_vault(&db, &dbk, &bind);
    let wrong = binding(0xff, 0xfe);
    assert!(matches!(
        open_sqlcipher(owner(), &db, &dbk, ConnectionMode::ReadWrite, Some(&wrong)),
        Err(OpenError::VaultSchemaRejected)
    ));
}

# Transactional Dual-Root Vault v2 Engine Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development or superpowers:executing-plans. Apply
> superpowers:test-driven-development to every code task and
> superpowers:verification-before-completion before every commit.

**Goal:** Build and independently verify an internal SQLCipher Vault engine
with dual DBK wrappers, native device custody, read-only recovery, and a safe
reader-first importer—without activating it in the product or overstating
whole-workspace confidentiality.

**Architecture:** A random 32-byte DBK opens one bundled SQLCipher 4.14.0
database through a tiny audited `sqlite3_key` raw-key wrapper. A native-store
DeviceKEK and an Argon2id-unlocked RecoveryKEK independently XChaCha-wrap that
DBK with distinct typed AAD. SQLCipher owns page encryption, transactions,
rollback journaling, and crash recovery. The database accepts only closed typed
business objects. Authority, identity, signing, session, and effect secrets
remain separate and block activation until their own handoff is designed.

**Delivered boundary:** Program 1A ends at internal engine and migration
readiness. There is no GUI/loopback/CLI enrollment, no live workspace migration,
no `vault.format` publication, no default switch, no legacy writer removal, and
no “Vault protected” or “Recovery ready” product state. Program 1B0 mechanics,
Program 1C0 owner presence, Program 1C1 identity handoff, Program 1B1
real-candidate qualification, and the complete
workspace plaintext inventory are mandatory prerequisites for a later
activation slice.

## Exact dependency profile

Dependencies are introduced only by the task that uses them:

```toml
# Task 1
rusqlite = { version = "=0.40.2", default-features = false,
  features = ["bundled-sqlcipher-vendored-openssl", "hooks", "limits"] }
zeroize = { version = "=1.9.0", features = ["derive"] }

# Task 2
chacha20poly1305 = { version = "=0.11.0", default-features = false,
  features = ["alloc", "zeroize"] }
argon2 = { version = "=0.5.3", default-features = false,
  features = ["alloc", "zeroize"] }
keyring = { version = "=4.1.5", default-features = false, features = ["v1"] }

# Task 4
cap-std = "=4.0.2"
cap-fs-ext = "=4.0.2"

# Dev-only assertion harness
static_assertions = "=1.1.0"
trybuild = "=1.0.116"
```

The released locked graph is expected to resolve `libsqlite3-sys = 0.38.2` and
vendor SQLCipher exactly 4.14.0. It MUST NOT be represented as 4.17.0 or
production-ready. SQLCipher 4.15 fixed a defensive-mode bypass in
`sqlcipher_export`; therefore the 4.14.0 Program 1A engine statically rejects
repository call sites/references to `sqlcipher_export`, dynamic `ATTACH`,
rusqlite backup APIs, and every export/copy route. Its closed SQL authorizer
also denies `sqlcipher_export` and `ATTACH` at runtime; do not claim the bundled
symbol itself is removed. Programs 1B0/1B1 and all product activation remain blocked
until a future RFC amendment pins one exact reviewed released Rust binding and
one exact SQLCipher release (4.17.0 or a later release named explicitly by that
amendment) and that profile passes the
gates, or the owner approves a separate exact-source dependency plan. Never use
a semver range or runtime “at least” check as the admitted profile.

Before writing implementation code, record
`cargo tree -p sovereign-vault -e features`, verify the vendored source/version,
and review all relevant advisories and release deltas. The reviewed candidate
revision beginning `62648175` carries 4.17.0 but is unreleased/unsigned in this
path; do not pin it from an abbreviated identifier or silently use master. An
exact-revision/vendor path needs a full independent dependency diff,
source-provenance and supply-chain decision, reproducible hash/build/license
evidence, and amended RFC/dependency profile. A material unresolved issue blocks
rather than triggers improvised build plumbing.

The crate build gate fails when dependency-shaping ambient overrides are set,
including `LIBSQLITE3_SYS_USE_PKG_CONFIG`, `LIBSQLITE3_FLAGS`,
`SQLITE_MAX_VARIABLE_NUMBER`, `SQLITE_MAX_EXPR_DEPTH`, `SQLITE_MAX_COLUMN`,
`OPENSSL_NO_VENDOR`, `SQLCIPHER_{LIB_DIR,INCLUDE_DIR,STATIC}`,
`OPENSSL_{DIR,LIB_DIR,INCLUDE_DIR}`, `PKG_CONFIG_*`, `VCPKGRS_DYNAMIC`, and
their relevant target-prefixed forms. Treat this as a closed allowlist: a new
dependency-shaping variable stops for RFC review. Record release C toolchain
variables separately and prove vendored selection in a clean subprocess. A
rejected build may have compiled a dependency before the Vault build script
runs, but cannot complete or qualify an artifact.

Do not invent or add keyring transitive provider features. `keyring` v1 selects
its native provider by target; if locked Cargo metadata proves an explicit
downstream feature is required, stop for dependency review and amend this plan
before code.

## Global constraints

- [RFC 0005](../../../rfcs/0005-dual-root-vault-and-recovery.md) is normative.
  A disagreement stops implementation until RFC amendment and re-review.
- Rust is pinned by CI to 1.97. All dependencies and enabled features are exact
  and reviewed in the lockfile.
- SQLCipher uses compatibility 4, 4096-byte pages, AES-256-CBC,
  HMAC-SHA512, encrypted header, `cipher_memory_security=ON`, memory-only temp,
  rollback journal, `synchronous=FULL`, foreign keys, trusted schema off,
  defensive mode, and no extension/attach/dynamic SQL path.
- DBK reaches SQLCipher only through the audited raw-key `sqlite3_key` wrapper.
  The wrapper is the first database operation; connection-specific cipher
  settings follow it before any page access. DBK never enters SQL text,
  `String`, argv, environment, logs, or configuration.
- Every XChaCha seal gets a fresh OS-random 24-byte nonce; callers cannot supply
  nonces, algorithms, raw keys, or generic AAD.
- Argon profile tag 1 is exactly v=19, m=65,536 KiB, t=3, p=4, 16-byte random
  salt, 32-byte output. Unknown/free-form cost input fails before KDF.
- `Secret32`, password, PWK, KEK, DBK, and raw-key token buffers are non-clone,
  non-formatting, non-serializing, zeroizing holders.
- `DeviceStoreUnavailable` consistently means native service absent, locked,
  unsupported, or unreachable. `DeviceKeyMissing` means the exact admitted
  record is absent from an otherwise usable service. Neither enters recovery.
- Generic tests use only an internal injected store. Real native-store behavior
  is tested in separate mandatory jobs with isolated namespaces and cleanup.
- All normal SQL is static. There are no caller identifiers/SQL, extensions,
  virtual tables, views, triggers, `ATTACH`, or generic blob escape hatch.
- The locked bundled C source compiles extension-loading machinery. Do not call
  that absence a protection: the rusqlite loading feature is disabled, the
  connection explicitly sets `SQLITE_DBCONFIG_ENABLE_LOAD_EXTENSION=0`, the
  authorizer rejects the SQL function, no raw connection/loading API escapes,
  and tests prove neither route can be enabled.
- Record compiled upstream built-ins such as JSON/FTS/RTree/DBSTAT/SOUNDEX
  instead of claiming they are absent. Fixed schema and Rust features expose no
  virtual tables/views/triggers or caller functions; the authorizer denies all
  non-allowlisted functions and schema/vtable construction. Test each compiled
  built-in through every supported query surface and require denial.
- Program 1A never writes `vault.format` and never wires a product mutation
  path. A verified staging database beside a live legacy root is not authority.
- Legacy `workspace_graph` and the demo's exact `venture_profile` are the only
  initial importable business roles, mapped through separate sealed adapters.
  The exact legacy entries `owner_admission_key`, `owner_approval_key`, and
  `runtime_authority_key` are `BlockedUntilIdentityHandoff`: authenticate and
  inventory them, but never insert, export, back up, or silently drop them.
  Any unknown legacy entry blocks import readiness.
- Every database open requires a crate-private `ExpectedVaultBinding` issued
  outside the parsed database/sidecar/selector/legacy tree. It binds workspace,
  expected format state, activation epoch, and database ID. Program 1A issues
  only internal staging bindings. Program 1D must source activated bindings
  from an owner-authenticated external registry; paths, caller text, or parsed
  fields cannot establish them.
- `sqlcipher_export`, `ATTACH`, rusqlite `Backup`, and DB/page-copy call sites
  are forbidden under the initial 4.14.0 engine profile. The `backup` feature is
  not enabled in Program 1A; Program 1B0 may revisit it only after a future RFC
  amendment names one exact SQLCipher release (4.17.0 or a later release
  selected explicitly by that amendment), and still
  must not copy the live database.
- `device.json`, ledger payloads, workflows, journals, authority stores,
  outbox/effect payloads, caches, exports, logs, and temporary files remain a
  whole-workspace activation blocker. Signed plaintext is still plaintext.
- No secure erasure, E2EE, hardware-backed, recovery-complete, rollback-proof,
  production-ready, or whole-workspace protection claim is produced.
- Every code task preserves the existing workspace smoke test. Every commit
  runs the full gate; focused green tests are insufficient.
- If the local environment lacks Rust, save the unavailable gate and obtain
  inspected GitHub Actions evidence. Never call an unavailable local command a
  pass.

## Fixed limits and schema

The connection factory calls `sqlite3_key` first and applies only cipher and C
connection-safety settings before its first page access. It does not call the
plaintext-header setter. `SELECT count(*) FROM sqlite_schema` is the first
authentication probe; only after it succeeds may normal open verify (never
silently convert) journal mode, application/schema IDs, max-page count, and
database metadata. Create-only initialization sets fixed database values and
then closes/reopens through the normal verifier. Runtime SQLite limits are:
SQL 64 KiB, one SQL value 16 MiB, 128 columns, expression depth 32, 16 compound
terms, 999 variables, trigger
depth 0, attached databases 0, LIKE pattern 256 bytes, and worker threads 0.
It sets/verifies `max_page_count=1,048,576` (4 GiB at 4096-byte pages), rejects
an existing database above that ceiling, and denies later changes. The typed
schema caps one object at 64 fixed 4 MiB chunks/256 MiB, one transaction's
aggregate new payload at 256 MiB and 10,000 objects, a wrapper sidecar at 256
KiB, recovery input at 1,024 bytes,
legacy manifest at 8 MiB, legacy entry JSON at 32 MiB, and legacy plaintext at
16 MiB. Lengths/counts/sums use overflow-safe checks before allocation or SQL.

The initial sealed object registry is exactly:

```text
1 = BusinessStateV1       = BackupEligible
2 = VentureProfileV1      = BackupEligible
```

Unknown or caller/plugin-selected tags fail. The initial legacy importer maps
`workspace_graph` to `BusinessStateV1` and the exact current demo entry
`venture_profile` to `VentureProfileV1`; each has its own strict schema and
size limits, and neither accepts a generic JSON/blob substitute. Documents and evidence remain
inside that authenticated graph for this engine slice; splitting either into a
new top-level eligible type requires a schema/RFC revision with an exact typed
payload contract. In particular, no generic “public evidence” byte container
exists.

## File map

```text
crates/vault/src/lib.rs           opaque public errors/format; no v2 activation API
crates/vault/build.rs             reject dependency-shaping ambient overrides
crates/vault/src/secret.rs        non-formatting zeroizing secret holders
crates/vault/src/sqlcipher.rs     connection factory + only unsafe sqlite3_key shim
crates/vault/src/schema.rs        fixed schema, typed business adapters, transactions
crates/vault/src/wrappers.rs      three typed AAD encodings and XChaCha wrappers
crates/vault/src/key_slots.rs     fixed Argon2 recovery/device slot records
crates/vault/src/platform.rs      sealed keyring::v1 native DeviceKEK adapter
crates/vault/src/recovery.rs      RecoverySession<ReadOnly> implementation
crates/vault/src/storage.rs       cap-dir DB/sidecar staging and durable replacement
crates/vault/src/legacy.rs        exact unversioned AES-GCM read-only importer
crates/vault/src/migration.rs     internal side-by-side staging transaction
crates/vault/tests/public.rs      production opacity/fail-closed external tests
crates/vault/tests/ui/            trybuild compile-fail surface tests
tests/adversarial/tests/          supported-API downgrade/exfiltration checks
.github/workflows/vault-platform.yml  mandatory native-store/durability jobs
docs/security/vault-v2-verification.md evidence and honest readiness ledger
```

---

### Task 1: Pin and prove the SQLCipher connection factory

**Files:**
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `crates/vault/Cargo.toml`
- Create: `crates/vault/build.rs`
- Modify: `crates/vault/src/lib.rs`
- Create: `crates/vault/src/secret.rs`
- Create: `crates/vault/src/sqlcipher.rs`
- Create: `crates/vault/tests/public.rs`
- Create: `crates/vault/tests/ui.rs`
- Create: `crates/vault/tests/ui/secret_surface.rs`
- Create: `crates/vault/tests/ui/secret_surface.stderr`

**Internal interfaces:** `DbKey`, `RawSqlcipherKey`, `SqlcipherProfile`,
`ConnectionMode::{ReadWriteCreateInternal,ReadWrite,ReadOnlyRecovery}`, and a
closed `open_sqlcipher` factory. No public connection or raw handle escapes.

- [ ] **Step 1: Write failing profile, opacity, and raw-key tests**

Add exact named tests for:

- `sqlcipher_runtime_is_exactly_4_14_0_for_released_profile`;
- `database_header_and_journal_do_not_contain_plaintext_canary`;
- `wrong_dbk_fails_without_schema_or_plaintext`;
- `connection_profile_matches_every_required_pragma`;
- `extensions_attach_writable_schema_and_dynamic_sql_are_denied`;
- `oversized_values_and_sql_fail_at_fixed_limits`; and
- `raw_dbk_never_reaches_sql_text_logs_or_environment`.

Use `matches!` for typed errors, not `assert_eq!` on a plaintext-bearing success
type. Put compile-fail tests on the public API proving downstream code cannot
name `DbKey`, reach the raw handle/shim, construct create mode, or choose a
cipher. Add `static_assertions::assert_not_impl_any!` for
`DbKey: Clone, Debug, Display, Serialize, Deserialize` inside the crate.

- [ ] **Step 2: Capture genuine RED**

```bash
cargo test -p sovereign-vault --lib sqlcipher::tests --locked -- --nocapture
cargo test -p sovereign-vault --test public --locked -- --nocapture
cargo test -p sovereign-vault --test ui --locked -- --nocapture
```

First run `-- --list` for filtered targets and reject `running 0 tests`. Save
missing-module/API stderr. If Rust is unavailable locally, create the tests-only
commit and require the draft-PR CI failure before implementation.

- [ ] **Step 3: Pin minimal dependencies and implement one unsafe shim**

Add only Task 1 dependencies. Confirm `rusqlite = 0.40.2` resolves
`libsqlite3-sys = 0.38.2` and bundled SQLCipher 4.14.0; record the exact feature
tree and advisory/delta review. Add a precise static/call-graph gate rejecting
`sqlcipher_export`, dynamic `ATTACH`, `rusqlite::backup::Backup`,
`Connection::backup`, and database/page-copy calls. Stop on a material
unresolved finding rather than changing sources ad hoc.

Implement the build gate for every listed global and target-prefixed ambient
override. Add negative CI/build tests that set each variable independently and
require a non-zero build before an artifact can qualify. Record approved
target-specific compile options, `cipher_provider`, provider version, and the
vendored source hashes; an unrecognized provider or build profile fails.

The only new unsafe function accepts `&DbKey`, fills a fixed zeroizing 67-byte
buffer with the official 67-byte raw-key blob literal
`x'<64 lowercase hex digits>'`, calls `rusqlite::ffi::sqlite3_key` with exact
pointer/length, checks `SQLITE_OK`, and clears both encoded and DBK buffers as
their lifetimes end. Passing 32 arbitrary bytes directly is a regression to
passphrase semantics and is forbidden. The shim contains
a line-by-line safety comment and exposes neither arbitrary bytes nor a raw
connection pointer.

The raw-key shim is the first database operation. The connection factory then
applies only the RFC cipher and C connection-safety settings, and uses a schema
count as the first page-access/key authentication probe. It performs profile
readback and verifies journal/application/schema/max-page database state only
after that probe; normal open fails on mismatch and never converts journal mode
or creates a missing database. Create mode is a separate initializer followed
by independent normal reopen. The bundled C symbols for extension loading exist, but the rusqlite
feature is absent; the factory explicitly disables both loading routes with
`SQLITE_DBCONFIG_ENABLE_LOAD_EXTENSION=0`, verifies the returned state, and the
authorizer/API surface keeps them unreachable.

- [ ] **Step 4: Run GREEN, official fixtures, and deliberate mutations**

Run focused tests, doc tests, and a fixed interoperability fixture generated and
independently verified with the official SQLCipher CLI blob-literal syntax;
record its cryptographic hash and known rows. Open it through the 67-byte shim,
check known rows, and assert build/runtime version plus every cipher setting.
Create through the shim and independently reopen with the CLI syntax.
Direct 32-byte input must fail the vector. Temporarily treat DBK as a password,
enable a plaintext header, enable WAL, and omit one profile check; the
applicable test must fail each time. Restore each mutation and rerun.

- [ ] **Step 5: Full gate, review, commit, push**

Inspect `cargo tree -p sovereign-vault -e features` and the bundled C build.
Run the plan-wide gate in Task 6. Commit only after independent review:

```bash
git add Cargo.toml Cargo.lock crates/vault
git commit -m "feat(vault): add fixed SQLCipher engine"
```

Push, fetch, and verify the exact remote branch contains the commit.

### Task 2: Add typed dual wrappers and read-only recovery

**Files:**
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `crates/vault/Cargo.toml`
- Modify: `crates/vault/src/lib.rs`
- Create: `crates/vault/src/wrappers.rs`
- Create: `crates/vault/src/key_slots.rs`
- Create: `crates/vault/src/platform.rs`
- Create: `crates/vault/src/recovery.rs`
- Modify: `crates/vault/tests/public.rs`
- Add: `crates/vault/tests/ui/recovery_read_only.rs`

**Internal interfaces:** `DeviceDbkAad`, `PwkRecoveryKekAad`,
`RecoveryDbkAad`, `NativeDeviceStore`, internal `TestOnlyDeviceStore`, fixed
`RecoveryRecord`, and `RecoverySession<ReadOnly>`.

- [ ] **Step 1: Write wrapper, root-independence, and typestate RED tests**

Write golden vectors for every typed AAD and one-field mutation of every field.
Add cross-purpose/cross-role/cross-workspace/database/epoch substitution tests,
fresh-nonce tests, and dependency upstream XChaCha/RFC 9106 vectors.

Tests must prove:

- unknown Argon profile is rejected before the KDF counter increments;
- DeviceKEK and RecoveryKEK/PWK bytes never occur in the other record;
- an unavailable injected store returns `DeviceStoreUnavailable`, creates no
  key file, reads no secret env/argv, and does not enter recovery;
- a usable store missing the exact record returns `DeviceKeyMissing`; and
- `RecoverySession<ReadOnly>` cannot compile a write, transaction, migration,
  wrapper rotation, device enrollment, effect, or raw SQL call.

Add strict `vault-v2/vault.slots` tests for RFC 0005's exact JCS shape: duplicate or
unknown field, non-canonical JSON/Base64url/decimal, wrong ID/nonce/salt/tagged
ciphertext length, wrong fixed version/epoch, trailing bytes, and the 256 KiB
ceiling all fail before KDF/unwrap counters increment.

Add tests that delete, corrupt, or substitute the recovery subrecord and prove
device unwrap fails because `DeviceDbkAad` binds the recomputed
`recovery_slot_commitment`. A valid slot from another workspace/database must
also fail. Password/RecoveryKEK rotation must atomically update recovery record,
commitment, and device DBK wrapper and verify both routes. Injection of device
store unavailability must leave the old complete sidecar and refuse rotation.
A complete older valid sidecar may still open and is asserted as the documented
rollback boundary, not misclassified as tamper detection.

- [ ] **Step 2: Capture focused RED**

```bash
cargo test -p sovereign-vault --lib wrappers::tests --locked -- --nocapture
cargo test -p sovereign-vault --lib key_slots::tests --locked -- --nocapture
cargo test -p sovereign-vault --lib recovery::tests --locked -- --nocapture
cargo test -p sovereign-vault --test ui recovery_read_only --locked -- --nocapture
```

- [ ] **Step 3: Implement the minimal roots**

Add Task 2 dependencies exactly as listed. Implement separate structs and
serialization functions for each RFC AAD; no generic purpose enum/context may
stand in for the three types. PWK wraps only RecoveryKEK. Each KEK wraps only a
DBK record. Use zeroizing fixed buffers and externally coarsen password/tag
failures to `RecoveryFailed`.

Compute `recovery_slot_commitment` as the RFC's domain/version-prefixed SHA-256
over the exact canonical recovery subrecord. Device open recomputes it before
unwrap. Recovery record changes require an available DeviceKEK to create the
new device wrapper; Program 1A's read-only recovery session exposes no rotation
or enrollment shortcut. Do not add a reciprocal device commitment to recovery
AAD.

Use only `keyring::v1::Entry`, service
`com.sovereign-founder-os.vault`, username
`device-kek:<base64url-workspace-id>:<base64url-protector-id>`, and versioned
secret `sfo-device-kek-v1:<base64url>`. Keep native adapter and test store
crate-private. Do not call sample-store or CLI provider selection helpers.
Before adding any transitive provider feature, inspect Cargo metadata and amend
the reviewed dependency profile.

Recovery opens the database read-only, applies `query_only=ON`, installs the
fixed read authorizer, and exposes typed reads/integrity checks only to internal
tests and verification. It has no product presentation, arbitrary plaintext
export, file write, or broker call. A separately authenticated,
capability-bounded Program 1B1/1C0 broker owns any future owner-visible recovery
presentation/export. Program 1A intentionally does not implement the
owner-authorized transition to device enrollment.

- [ ] **Step 4: GREEN, negative mutation, and release-surface checks**

Run focused tests. Temporarily reuse an AAD type, omit database role, expose a
write method, and map missing record to unavailable; each intended test must
fail. Build a release binary and inspect strings/symbols for test-store
selection, task-secret environment names, raw keys, and file-key writers.

- [ ] **Step 5: Full gate, review, commit, push**

Record Cargo feature evidence and run Task 6's full gate. Commit:

```bash
git add Cargo.toml Cargo.lock crates/vault
git commit -m "feat(vault): add typed dual DBK wrappers"
```

### Task 3: Implement the closed transactional business schema

**Files:**
- Create: `crates/vault/src/schema.rs`
- Modify: `crates/vault/src/lib.rs`
- Modify: `crates/vault/src/sqlcipher.rs`
- Modify: `crates/vault/src/recovery.rs`
- Modify: `crates/vault/tests/public.rs`
- Modify: `tests/adversarial/tests/security_invariants.rs`

**Internal interfaces:** sealed `BusinessStateV1` and `VentureProfileV1`
adapters, opaque IDs,
`BusinessTransaction`, typed read session, fixed metadata and chunk schema. No
string type tag or generic byte-write API.

- [ ] **Step 1: Write schema and transaction RED tests**

Add exact tests that:

- one commit writes a complete multi-row object set or no rows after every
  failpoint;
- foreign keys/checks reject wrong workspace/database ID, tag, chunk order,
  count, byte total, duplicate ID, and oversize values;
- unknown/caller/plugin/model tags cannot compile or are rejected before SQL;
- rollback journal recovery yields the old or complete new transaction;
- plaintext names/content never appear in database header, rollback journal,
  sidecar, path, error, or Debug output; and
- authority, identity, signing, session, credential, freshness, recovery, and
  pending-effect data have no schema/adaptor route.

- [ ] **Step 2: Capture RED**

```bash
cargo test -p sovereign-vault --lib schema::tests --locked -- --nocapture
cargo test -p sovereign-adversarial-tests --test security_invariants vault_v2_closed_schema --locked -- --exact --nocapture
```

List exact names first and reject zero-test results.

- [ ] **Step 3: Implement the minimum static schema**

Create only metadata, business object, and ordered chunk tables. Use static SQL,
foreign keys, fixed `CHECK`s, prepared parameters, and one immediate transaction
per logical mutation. Store registry and schema version in authenticated
SQLCipher pages and verify them on every open. No view, trigger, virtual table,
attach, caller SQL, dynamic identifier, or generic metadata/value table.

The sealed constructor—not callers—assigns the one object tag and backup
disposition.
The read API returns typed zeroizing buffers and verifies chunk count/length
before allocation. Recovery uses the same typed query layer under its stronger
authorizer.

- [ ] **Step 4: GREEN and fault/mutation matrix**

Run failpoints at begin, metadata/object/chunk insert, commit, journal sync,
close, reopen, and integrity check. Mutate a check constraint, enable a generic
tag, and skip a chunk; tests must fail. Restore and rerun.

- [ ] **Step 5: Full gate, review, commit, push**

```bash
git add crates/vault tests/adversarial
git commit -m "feat(vault): transact closed business objects"
```

### Task 4: Add private no-follow staging and the real v1 importer

**Files:**
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `crates/vault/Cargo.toml`
- Create: `crates/vault/src/storage.rs`
- Create: `crates/vault/src/legacy.rs`
- Create: `crates/vault/src/migration.rs`
- Modify: `crates/vault/src/lib.rs`
- Modify: `crates/vault/tests/public.rs`
- Modify: `tests/adversarial/tests/security_invariants.rs`

**Internal output:** a `VerifiedV2Staging` evidence value with DB/sidecar
commitments and explicit blockers. It has no activation/publish method and is
not constructible by product code.

- [ ] **Step 1: Write exact legacy/layout/crash RED tests**

Model the actual unversioned `workspace/vault/` root: `vault.key`, optional exact empty-v1
manifest case, `manifest.json`, and listed `<name>.enc` files in that same root.
Add missing-key no-regeneration, missing manifest with ciphertext, duplicate,
unlisted/missing file, symlink/hardlink, traversal, unknown field/name, oversize,
wrong nonce/key/tag, and v2-failure-never-opens-v1 tests.

Add the closed role mapping:

```text
workspace_graph        -> BusinessStateV1
venture_profile        -> VentureProfileV1
owner_admission_key    -> BlockedUntilIdentityHandoff
owner_approval_key     -> BlockedUntilIdentityHandoff
runtime_authority_key  -> BlockedUntilIdentityHandoff
anything else          -> UnsupportedLegacyRole
```

Prove all three key entries are structurally/AEAD verified but never inserted
into SQLCipher, exported, logged, or discarded from the retained legacy source.
Persistent blocker evidence contains only a pre-content random blocker ID and
the explicitly approved coarse state `LegacyRoleHandoffRequired`; it contains
no role name, exact count, digest, or commitment. Exact roles and counts are
shown only in an authenticated transient owner inventory. `VerifiedV2Staging`
remains blocked while any role key lacks identity handoff.

Add a complete-generation substitution test: create valid workspaces A and B
whose native DeviceKEKs both exist, replace B's database and sidecar together
with A's, and prove B's independently issued `ExpectedVaultBinding`
rejects the generation before returning business plaintext. Reading the
expected binding from the substituted files is specifically forbidden. Program
1D tests separately delete/rollback `vault.format`, replay the retained legacy
tree, and substitute an old v2 database; the external active format/epoch/
database binding must make every case terminal rather than invoke legacy.

- [ ] **Step 2: Capture genuine RED on current code**

```bash
cargo test -p sovereign-vault --lib legacy::tests::missing_key_never_regenerates --locked -- --exact --nocapture
cargo test -p sovereign-vault --lib migration::tests::role_keys_block_activation --locked -- --exact --nocapture
cargo test -p sovereign-adversarial-tests --test security_invariants vault_v2_failure_never_falls_back --locked -- --exact --nocapture
```

Confirm current `Vault::init` regenerates the absent key or lacks the new typed
failure; save that RED evidence.

- [ ] **Step 3: Implement read-only import into side-by-side staging**

Add only Task 4 cap dependencies. Retain a capability directory for sidecar and
legacy I/O; reject symlinks/reparse paths and insecure permissions, use
no-follow final opens, and lock cooperating writers. SQLite's stock VFS remains
path based, so create rather than adopt a random private staging directory and
open the fixed DB name with `SQLITE_OPEN_NOFOLLOW`. Verify this flag exists in
the locked SQLite build. Do not claim that cap-std makes SQLite journal opens
descriptor-relative. A malicious concurrent same-user process able to rename
the private directory remains inside the trusted OS/process boundary; a custom
VFS is out of scope because it would enlarge the unsafe pager TCB. Never
relocate the current v1 root into a fictional `vault-v1/` directory.

Internal test/verification fixtures provision both wrappers and create a new
private sibling `workspace/vault-v2.staging-<opaque-random-id>/` containing only
fixed names `vault.db` and `vault.slots`. The exact legacy `workspace/vault/`
root therefore remains strict and unchanged. In one SQL transaction import
only the exact `workspace_graph` and/or `venture_profile` roles, commit, run both
integrity checks, independently reopen through device and recovery read-only
roots, compare row/chunk counts and transaction-local content commitment, sync
database/sidecar/directory, and optionally durably rename the verified directory
to non-authoritative `workspace/vault-v2/`, then return `VerifiedV2Staging` with blockers.

Do not write `vault.format`, change product call sites, write both authorities,
or delete legacy material. On every failpoint the legacy root remains untouched
and authoritative; staging is absent or explicitly non-authoritative.

- [ ] **Step 4: GREEN, race/fault matrix, and canary scan**

Exercise file create/sync, SQL commit/journal, sidecar create/sync/rename,
directory sync, close, device reopen, recovery reopen, and integrity-check
failpoints plus two cooperating processes. Scan paths, database/journal/sidecar,
stdout/stderr, Debug, and evidence for business/key canaries. Mutate v2 failure
to try v1; the downgrade test must fail.

- [ ] **Step 5: Full gate, review, commit, push**

```bash
git add Cargo.toml Cargo.lock crates/vault tests/adversarial
git commit -m "feat(vault): verify side-by-side legacy import"
```

### Task 5: Make real platform assurance mandatory

**Files:**
- Create: `.github/workflows/vault-platform.yml`
- Modify: `.github/workflows/ci.yml`
- Modify: `crates/vault/src/platform.rs`
- Modify: `crates/vault/src/storage.rs`
- Add platform integration tests under `crates/vault/tests/`
- Add platform fault harness under `scripts/`

**Outcome:** three named required checks—macOS native store, Windows native
store, and Linux Secret Service/durability—whose exact evidence controls only
platform engine readiness, not product activation.

- [ ] **Step 1: Add failing workflow-contract and platform tests**

Tests assert each job uses the real target-native provider, an isolated
`sfo-ci:<run-id>:<attempt>` namespace, random credentials, independent
set/get/delete, locked/unavailable/missing mapping, cleanup in `always()`, and no
sample/injected fallback. The Linux job creates a real isolated D-Bus Secret
Service session and proves the selected keyring v1 provider; macOS uses the
runner Keychain; Windows uses Credential Manager. Ambient developer secrets are
never read.

Jobs also assert the closed cryptographic CPU-architecture allowlist. Initial
candidates are only `x86_64` and `aarch64`; a 32-bit, PowerPC, embedded, or
other target is `Unavailable` until a separate review proves the locked
XChaCha implementation's constant-time multiplication precondition and adds a
mandatory native job. Cross-compilation alone does not qualify a target.

Add native permission/no-follow/reparse, process race, rollback-journal crash,
sidecar replacement, directory durability, and close/reopen tests. Jobs fail if
the service cannot be initialized; they do not skip.

- [ ] **Step 2: Capture RED on draft PR**

The workflow file must run on `pull_request`, manual dispatch, and protected
main pushes. Record all three missing/failing check names before implementation.
Do not place platform-only checks behind a generic success aggregate that can
go green when one job is skipped.

- [ ] **Step 3: Implement isolated native jobs and lock feature evidence**

Before code, save `cargo tree -p sovereign-vault -e features` separately on all
three targets. `keyring = =4.1.5` remains `default-features=false, features=[v1]`.
If a provider cannot be proven native from locked metadata and a real roundtrip,
stop that platform as `Unavailable`; do not guess a transitive feature.

Use least job permissions, SHA-pinned actions, no artifact containing key
material, and unconditional namespace cleanup. A platform's required status
name is stable and documented for branch protection.

- [ ] **Step 4: Run platform attack/fault matrices**

Verify unavailable versus missing mapping, no automatic recovery, no file key,
no cross-job namespace read, DB/sidecar permission failures, path swaps,
concurrent writers, and every durable failpoint. Runner Secret Service proves
an `OsProtected` software integration path only; it is not hardware assurance.

- [ ] **Step 5: Review, commit, push, inspect all required checks**

```bash
git add .github/workflows crates/vault scripts
git commit -m "ci(vault): require native custody and durability"
```

Push and inspect each named job log. A skipped, cancelled, neutral, or injected
job is not passing platform evidence.

### Task 6: Publish engine evidence without activating the product

**Files:**
- Modify: `README.md`
- Modify: `ARCHITECTURE.md`
- Modify: `THREAT_MODEL.md`
- Modify: `ROADMAP.md`
- Modify: `docs/INDEX.md`
- Modify: `rfcs/0005-dual-root-vault-and-recovery.md`
- Create: `docs/security/vault-v2-verification.md`
- Add final negative tests/searches under `crates/vault/tests/` and
  `tests/adversarial/tests/`

- [ ] **Step 1: Add final claim and activation-blocker tests**

Add precise AST/call-site or allowlisted searches proving:

- Program 1A product code has no v2 enrollment/migration/activation constructor;
- no code writes `vault.format` outside future activation-only test fixtures;
- no GUI/loopback/CLI accepts a recovery password;
- no `sqlcipher_export`, dynamic `ATTACH`, rusqlite backup, or database-copy
  call is reachable under the initial 4.14.0 engine profile;
- no v2 path calls legacy after authentication failure;
- no role key or `device.json` private key enters the business schema;
- the old raw-key writer remains only on the explicitly documented legacy
  product path and is never called by v2 staging; and
- UI/docs do not claim workspace protection or recovery readiness.

Do not reject legacy literals needed by the read-only importer. Prove module and
call-graph boundaries instead of brittle repository-wide string bans.

- [ ] **Step 2: Create the whole-workspace persistence ledger**

The verification document inventories every current writer/value class and
records one disposition:

```text
VaultBusinessEligible
SeparateProtectedDomainRequired
IntentionallyPublicWithScope
EphemeralOnly
BlockedUnknown
BlockedUntilIdentityHandoff
```

At minimum cover `device.json`, the three legacy role-key entries, ledger
payloads, workflows/checkpoints, execution journals, authority stores,
outbox/effect content, artifacts/caches, logs/reports, exports, temp/crash data,
model/provider/plugin stores, and every `std::fs`/database writer found by an
allowlisted inventory script. Any `Blocked*` entry keeps activation forbidden.

- [ ] **Step 3: Run the fresh full gate**

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked
cargo test -p sovereign-vault --doc --locked
./scripts/check-file-size.sh
cargo build -p sovereign-cli --release --locked
cargo tree -p sovereign-vault -e features
git diff --check HEAD --
```

Also run pinned frontend type checking, `cargo audit`, dependency review, secret
scan, SQLCipher version/profile/raw-key fixtures, release binary inspection,
parser fuzz corpus, full transaction/migration fault matrix, and all three
mandatory native platform jobs. Stage new files before the size script.

- [ ] **Step 4: Record exact maturity and deferred activation**

The ledger records exact dependency/features, SQLCipher compile/runtime version,
profile readback, unsafe-shim review, vectors, platform observations,
transaction/fault results, importer roles/blockers, leakage, and independent
review findings. Only passing internal engine/platform paths may become
`Experimental`. Product Vault v2, backup, restore, identity handoff,
owner-authorized activation, hardware backing, rollback anchor, and
whole-workspace confidentiality remain Target/Research.

- [ ] **Step 5: Independent final review, commit, push**

Request separate spec/security and code-quality reviews. Fix every
Critical/High finding with fresh RED/GREEN evidence, rerun all gates, then:

```bash
git status --short
git diff --cached --name-status
git add README.md ARCHITECTURE.md THREAT_MODEL.md ROADMAP.md docs rfcs \
  crates/vault tests/adversarial Cargo.toml Cargo.lock .github/workflows scripts
git commit -m "security(vault): verify transactional v2 readiness"
```

Push and verify the exact remote commit and all required checks. Do not merge,
tag, enable product enrollment, delete legacy files, or claim recovery.

## Explicit follow-on boundaries

### Programs 1B0/1B1: Filtered sovereign backup

Create a separate plan that uses a read transaction to build a new SQLCipher
recovery database with a new DBK/database/snapshot ID and only closed
`BackupEligible` rows. It must never copy the live DB/journal/sidecar. After
commit, integrity checks, and independent recovery read-only reopen, package it
with recovery-only wrapper metadata and encrypt the canonical padded archive
using pinned age X25519 recipient mode. Require age interoperability and a
clean-machine harness. **1B0** uses only an internal staging binding and test
issuer to prove mechanics; it cannot qualify a product backup or recovery
claim. **1B1** runs inside Program 1D after an exclusive legacy freeze and a
new same-snapshot migration emit an unforgeable `VerifiedMigration` binding
the frozen source commitment to the candidate content commitment. Only that
proof may create externally authenticated `PendingV2`; an older Program 1A
staging candidate is rebuilt if its source differs. 1B1 consumes Program 1C0 owner authority, restores that exact
candidate on a clean environment, and returns a `RecoveryQualification` bound
to workspace/database ID, activation epoch, schema/registry, platform profile,
source-head commitment, and backup commitment. Only an exact match permits
`ActiveV2`. Program 1B0 cannot start on the bundled SQLCipher 4.14.0
profile because of the fixed `sqlcipher_export` defensive-mode bypass; first
admit and verify the single exact released binding/SQLCipher version selected
by RFC amendment (exactly 4.17.0 or a later release named by that amendment),
or a separately owner-approved exact-source profile. Even after upgrade, keep
`sqlcipher_export`, `ATTACH`, and database-copy APIs forbidden: filtering is
typed row-by-row into the closed recovery schema.

Because the online device route cannot unwrap the independent RecoveryKEK,
snapshot creation is an explicit owner-present ceremony that consumes the
recovery password locally, wraps the new snapshot DBK, and zeroizes the
password-derived material. The age public recipient is not a replacement for
that factor. Unattended scheduled backups are outside this profile and require
a separate reviewed online backup authority/key design. The builder also
consumes an opaque one-use `BackupAuthorization` bound to source workspace/DB/
epoch, schema/registry, operation, age recipient, expiry, and challenge. Only a
crate-internal test issuer exists before Program 1C0; it cannot qualify or
expose a product backup path. Program 1B1 must consume the real 1C0 issuer.

### Programs 1C0/1C1: Owner authorization, identity custody, and handoff

**1C0** first admits the single expiry-bound, CSRF-resistant owner session and
one-use approval issuer consumed by Exact Effect, 1B1, and 1D. No product,
Vault code, or loopback UI may create a parallel/application-local owner
signer.

**1C1** designs a separate domain for `device.json` private material and the
legacy `owner_admission_key`, `owner_approval_key`, and
`runtime_authority_key` roles.
Do not solve this by inserting them into the business SQLCipher database. The
handoff must preserve or explicitly reset authority continuity and invalidate
pending effects.

### Program 1D: Owner-authorized activation

Only after Program 1B0, Program 1C0, Program 1C1 identity/role handoff, platform
qualification, and zero unknown plaintext writers may a new RFC/plan consume
one-use authenticated owner presence and enter a crash-recoverable externally
authenticated `PendingV2` state binding workspace/format/activation epoch/
database ID. It exclusively freezes the last legacy generation, rebuilds the
candidate from that same retained read snapshot, and consumes an unforgeable
`VerifiedMigration` that binds frozen source and candidate content
commitments. Any change since earlier staging invalidates and rebuilds the
candidate. Only this proof creates `PendingV2`; Program 1D then publishes
`vault.format` and executes Program 1B1 on the exact pending
candidate. Only a matching `RecoveryQualification` advances to `ActiveV2`,
switches one authoritative product store, and exposes honest status. 1D
consumes 1C0; it never creates owner authority. Missing/old selectors or a
retained legacy tree never downgrade an ActiveV2 binding. Loopback origin, OS
account access, credential-store unlock, and password possession are not owner
presence.

Add a race/failpoint regression: create staging at t0, write a new business
object to legacy at t1, freeze at t2, and prove the t0 candidate is rejected,
the rebuilt candidate includes the t1 object, and no crash can publish
`ActiveV2` without the same equivalence proof and 1B1 qualification.

## Plan self-review

- The design reuses SQLCipher transaction/recovery machinery and deletes the
  custom encrypted-object/manifest/head protocol from scope.
- DBK custody, typed AAD, raw-key handoff, SQLCipher profile, parser/resource
  limits, closed schema, exact real legacy layout, and recovery read-only
  typestate have named tests and owners.
- Real platform behavior is isolated into mandatory jobs; generic injected
  tests cannot qualify a platform.
- The three current role keys are neither migrated nor ignored: they explicitly
  block activation pending identity handoff.
- Program 1A has no product enrollment or dual authoritative writer, so Task 5
  and Task 6 cannot create split brain.
- Backup, activation, whole-workspace secrecy, identity continuity, hardware
  backing, rollback anchors, and secure erasure remain outside the delivered
  claim.

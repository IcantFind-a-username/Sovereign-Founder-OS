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
database through one audited FFI module with exactly two narrow entry points:
process bootstrap and raw-key/connection hardening. A native-store
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

The engine is a separate `publish = false` workspace crate,
`sovereign-vault-v2-engine`. During Program 1A neither the shipped CLI nor the
legacy `sovereign-vault` crate depends on it. Do not implement this separation
as a default-off Cargo feature: workspace `--all-features`, feature unification,
or an accidental downstream selection could otherwise link the admitted-only
SQLCipher 4.14.0 objects into a product binary. Program 1D and the newer exact
SQLCipher profile require a distinct RFC-reviewed dependency-edge change.

## Exact dependency profile

Dependencies are introduced only by the task that uses them:

```toml
# Task 1
rusqlite = { version = "=0.40.2", default-features = false,
  features = ["bundled-sqlcipher-vendored-openssl", "hooks", "limits"] }
openssl-sys = { version = "=0.9.117", default-features = false }
zeroize = { version = "=1.9.0", features = ["derive"] }

# Task 1 dev-only evidence harness
serde = { version = "=1.0.228", default-features = false,
  features = ["derive", "std"] }
tempfile = "=3.27.0"
sha2 = { version = "=0.10.9", default-features = false }
syn = { version = "=2.0.118", default-features = false,
  features = ["full", "parsing"] }
static_assertions = "=1.1.0"
trybuild = "=1.0.116"

# Task 2 normal dependencies. Move, do not duplicate, the exact serde and sha2
# entries above from dev-dependencies to dependencies.
chacha20poly1305 = { version = "=0.11.0", default-features = false,
  features = ["alloc", "zeroize"] }
argon2 = { version = "=0.5.3", default-features = false,
  features = ["alloc", "zeroize"] }
keyring = { version = "=4.1.5", default-features = false, features = ["v1"] }
getrandom = { version = "=0.4.3", default-features = false }
serde_json = { version = "=1.0.150", default-features = false,
  features = ["std"] }
serde_json_canonicalizer = "=0.3.2"
base64 = { version = "=0.22.1", default-features = false, features = ["alloc"] }

# Task 4
cap-std = "=4.0.2"
cap-fs-ext = "=4.0.2"
aes-gcm = { version = "=0.10.3", default-features = false,
  features = ["aes", "alloc", "zeroize"] }
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

Before writing implementation code, record the frozen engine feature tree
through `scripts/qualify-vault-v2.sh`, verify the vendored source/version,
and review all relevant advisories and release deltas. The reviewed candidate
revision beginning `62648175` carries 4.17.0 but is unreleased/unsigned in this
path; do not pin it from an abbreviated identifier or silently use master. An
exact-revision/vendor path needs a full independent dependency diff,
source-provenance and supply-chain decision, reproducible hash/build/license
evidence, and amended RFC/dependency profile. A material unresolved issue blocks
rather than triggers improvised build plumbing.

The single executable qualification entry point is
`scripts/qualify-vault-v2.sh`. It rejects unapproved ambient input, constructs
a positive-allowlist child environment, creates and owns a fresh dedicated
`CARGO_TARGET_DIR`, and only then invokes the exact Cargo command. Every command
that resolves or builds `sovereign-vault-v2-engine`—including workspace
Clippy/tests, docs, metadata/tree inspection, CI, and recorded local
qualification—goes through this wrapper. Its `full` mode reuses one fresh
target only within that invocation; no qualification restores or writes a
shared target cache.

The wrapper fails when dependency-shaping or vendored-build-tool ambient
overrides are set,
including `LIBSQLITE3_SYS_USE_PKG_CONFIG`, `LIBSQLITE3_FLAGS`,
`SQLITE_MAX_VARIABLE_NUMBER`, `SQLITE_MAX_EXPR_DEPTH`, `SQLITE_MAX_COLUMN`,
`OPENSSL_NO_VENDOR`, `SQLCIPHER_{LIB_DIR,INCLUDE_DIR,STATIC}`,
`OPENSSL_{DIR,LIB_DIR,INCLUDE_DIR,CONFIG_DIR,LIBS,STATIC}`,
`OPENSSL_SRC_PERL`, `PERL`, `OPENSSL_RUST_USE_NASM`, `PKG_CONFIG_*`,
`VCPKGRS_DYNAMIC`, and their relevant target-prefixed forms. The direct
`openssl-src` tool variables are rejected in their exact global forms. Treat
this as a closed allowlist: a newly observed dependency-shaping or tool-
selection variable stops for RFC review. It also removes `PERL5OPT`,
`PERL5LIB`, `RUSTFLAGS`, `CARGO_ENCODED_RUSTFLAGS`, and unapproved linker/tool
overrides; resolves Cargo, rustc, C compiler, archiver, ranlib, Perl, and make
from an approved path set; and records their canonical paths, versions, and
SHA-256 digests. Qualification is `--frozen --offline` after a separate
`cargo fetch --locked` acquisition step, so the controlled child does not run
network acquisition. A new tool or environment input stops for review.

The child starts from `env -i` and receives only fresh `HOME`/`TMPDIR`, the
reviewed Cargo/rustup homes, `LANG`/`LC_ALL`, fixed `SOURCE_DATE_EPOCH`, exact
absolute Cargo/rustc/rustdoc/C compiler/archiver/ranlib paths, the exact
`CARGO_BUILD_TARGET`, `CARGO_NET_OFFLINE=true`, and a PATH assembled only from
the reviewed Perl/make/linker directories. Repository and Cargo source-config
replacement is rejected. Cargo-created build-script variables such as
`HOST`, `TARGET`, `OUT_DIR`, and `CARGO_MAKEFLAGS` are allowed only inside the
child and are recorded where relevant; they are not accepted from the caller.

`crates/vault-v2-engine/build.rs` repeats the checks as defense in depth, but a
downstream build script runs too late to prevent an overridden upstream
dependency from compiling or being reused from Cargo's cache. It is never the
qualification gate by itself. Negative tests set each variable independently,
invoke the wrapper, and require refusal before Cargo. CI has a dedicated
uncached qualification step; ordinary cached product jobs exclude the unlinked
engine package.

Do not invent or add keyring transitive provider features. `keyring` v1 selects
its native provider by target; if locked Cargo metadata proves an explicit
downstream feature is required, stop for dependency review and amend this plan
before code.

Task 2 uses only the direct fallible `getrandom::fill` API for every DBK, KEK,
salt, nonce, and protocol ID. It does not enable or call an AEAD convenience
generator, `rand`, a custom backend, or handwritten operating-system RNG FFI.
An entropy error zeroizes every partially filled supported buffer, publishes no
wrapper/sidecar/key-store/database state, and returns one value-free terminal
error. The exact native-target gate also rejects opt-in/custom `getrandom`
backends. A crate-private `SystemEntropy` wrapper is the only production
implementation; a crate-private test double may inject failure at each fill
boundary but is absent from normal/release binaries. Task 2's JSON stack is not
a generic persistence escape hatch: it
parses only the bounded closed sidecar type with duplicate/unknown-field denial,
then requires byte-for-byte equality with RFC 8785 serialization before any
KDF or unwrap. Base64 is the exact unpadded URL-safe alphabet; SHA-256 is used
only for the RFC-defined domain-separated recovery-slot commitment.

Task 4's pinned AES-GCM dependency is legacy-reader-only. It may authenticate
and decrypt the exact current 12-byte-nonce/full-tag record into a zeroizing
typed importer buffer, but exposes no legacy encrypt, key-generation, arbitrary
AAD, or write API. It never falls back between AES-GCM and the v2 profiles after
an authentication failure.

Task 1 qualifies exactly `x86_64-unknown-linux-gnu`, with native
`HOST == TARGET`. The wrapper verifies the rustc host, rejects incoming
`--target`/`CARGO_BUILD_TARGET` and target-linker overrides, then sets that
exact target itself before Cargo. Every other triple/ABI—including musl, wasm,
cross-compiles, Windows, and macOS—fails the engine gate until Task 5 adds its
own exact triple and real native mandatory job. This does not block product
builds because the product has no dependency on the engine.

Program 1A is a binary-first dedicated experimental process, not a reusable
database library. `src/main.rs` immediately obtains a private,
non-constructible `CryptoProcessOwner` from the sole unsafe bootstrap before
dispatching any command. The bootstrap declares the exact official C ABI and
the pinned `OPENSSL_INIT_NO_LOAD_CONFIG` constant locally because
`openssl-sys 0.9.117` does not expose them, calls
`OPENSSL_init_crypto(..., NULL)`, and requires return value `1`. The exact
direct `openssl-sys` dependency supplies the reviewed link/version boundary;
the AST/call-graph gate permits no `openssl_sys::init`, other OpenSSL caller, or
alternate bootstrap. Every SQLCipher open requires `&CryptoProcessOwner`.

The package library target contains only a value-free protocol/version surface;
DBK, connection, and FFI code compile only into the private process target.
Behavioral qualification invokes the real binary in a fresh subprocess. A
test-only fresh worker first calls `OPENSSL_INIT_LOAD_CONFIG` with a hostile
configuration, then proves the later bootstrap/profile gate refuses it; a
boolean marker is not accepted as evidence of global OpenSSL state. Normal
fresh-process tests set hostile `OPENSSL_CONF`, `OPENSSL_CONF_INCLUDE`,
`OPENSSL_ENGINES`, and `OPENSSL_MODULES` and require the identical admitted
provider/profile. Program 1D still must keep a dedicated authenticated broker
or separately prove a single process-wide initialization owner; arbitrary
in-process embedding remains forbidden.

The LOAD_CONFIG adversary is the only additional raw OpenSSL call, lives under
`cfg(test)` inside the same FFI module, and is proven absent from the normal and
release binaries. “Exactly two entry points” refers to production code; the
source gate separately allowlists this one test-only negative control.

## Global constraints

- [RFC 0005](../../../rfcs/0005-dual-root-vault-and-recovery.md) is normative.
  A disagreement stops implementation until RFC amendment and re-review.
- Rust is pinned by CI to 1.97. All dependencies and enabled features are exact
  and reviewed in the lockfile.
- SQLCipher uses compatibility 4, 4096-byte pages, AES-256-CBC,
  HMAC-SHA512, encrypted header, `cipher_memory_security=ON`, memory-only temp,
  rollback journal, `synchronous=FULL`, foreign keys, trusted schema off,
  defensive mode, and no extension/attach/dynamic SQL path.
- DBK reaches SQLCipher only through one audited unsafe connection-hardening
  entry point. It owns `sqlite3_open_v2`, calls `sqlite3_key` as the first
  post-open database operation, then disables/verifies both extension routes
  with `sqlite3_enable_load_extension(handle, 0)` and
  `sqlite3_db_config(handle, SQLITE_DBCONFIG_ENABLE_LOAD_EXTENSION, 0, &out)`.
  The locked safe rusqlite API cannot express option 1005 without enabling the
  forbidden loading feature, so no extra raw-handle shim is permitted. The
  only other unsafe entry point is the process-first OpenSSL bootstrap; unsafe
  code outside this single FFI module is forbidden. Cipher
  settings and no-page C limits follow before first page access. DBK never
  enters SQL text, `String`, argv, environment, logs, or configuration.
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
  and tests verify both initialization switches are disabled and that no
  supported/public path can re-enable either route. This does not claim hostile
  new unsafe code holding a raw handle is cryptographically unable to re-enable
  compiled C machinery; the AST/raw-handle gate excludes that code.
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

After raw `sqlite3_open_v2`, the factory calls `sqlite3_key` first, disables extension loading,
and applies cipher/C connection safety plus every fixed `sqlite3_limit` before
its first page access. It does not call the plaintext-header setter. For an
existing database, `SELECT count(*) FROM sqlite_schema` is the first
authentication probe; only after it succeeds may normal open inspect database
PRAGMAs. For a zero-byte create-only database it is merely an empty-schema
probe: before its first write the initializer sets and reads back
`max_page_count=1,048,576`, runs the one fixed create/insert/drop page-forcing
transaction, commits and closes, then independently reopens through the normal
no-`CREATE` verifier before returning. Runtime SQLite limits are:
SQL 64 KiB, one SQL value 16 MiB, 128 columns, expression depth 32, 16 compound
terms, 999 variables, trigger
depth 0, attached databases 0, LIKE pattern 256 bytes, and worker threads 0.
After authentication it rejects `page_count > 1,048,576`, then sets and
verifies the per-connection `max_page_count=1,048,576` ceiling and denies later
changes; it does not treat that PRAGMA as persistent metadata. Task 1 does not
invent an application/schema version. Task 3 fixes `application_id`,
`user_version`, registry, and metadata; SQLite's internal `schema_version`
cookie is never the application version. The typed
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
crates/vault-v2-engine/src/lib.rs           value-free protocol/version only; no engine API
crates/vault-v2-engine/src/main.rs          sole dedicated-process bootstrap and dispatcher
crates/vault-v2-engine/src/engine/mod.rs    private process-owned engine state
crates/vault-v2-engine/src/engine/process.rs private CryptoProcessOwner/OpenSSL bootstrap
crates/vault-v2-engine/src/engine/ffi.rs    sole unsafe C ABI/open/key/hardening boundary
crates/vault-v2-engine/build.rs             reject dependency-shaping ambient overrides
crates/vault-v2-engine/src/engine/secret.rs non-formatting zeroizing secret holders
crates/vault-v2-engine/src/engine/sqlcipher.rs closed connection/profile state machine
crates/vault-v2-engine/src/engine/schema.rs        fixed schema, typed business adapters, transactions
crates/vault-v2-engine/src/engine/wrappers.rs      three typed AAD encodings and XChaCha wrappers
crates/vault-v2-engine/src/engine/key_slots.rs     fixed Argon2 recovery/device slot records
crates/vault-v2-engine/src/engine/platform.rs      sealed keyring::v1 native DeviceKEK adapter
crates/vault-v2-engine/src/engine/recovery.rs      RecoverySession<ReadOnly> implementation
crates/vault-v2-engine/src/engine/storage.rs       cap-dir DB/sidecar staging and durable replacement
crates/vault-v2-engine/src/engine/legacy.rs        exact unversioned AES-GCM read-only importer
crates/vault-v2-engine/src/engine/migration.rs     internal side-by-side staging transaction
crates/vault-v2-engine/tests/public.rs      protocol opacity/metadata/process behavior tests
crates/vault-v2-engine/tests/process.rs     fresh real-binary qualification tests
crates/vault-v2-engine/tests/ui/            trybuild compile-fail surface tests
tests/adversarial/tests/          supported-API downgrade/exfiltration checks
.github/workflows/vault-platform.yml  mandatory native-store/durability jobs
docs/security/vault-v2-verification.md evidence and honest readiness ledger
scripts/qualify-vault-v2.sh      sole sanitized Cargo qualification entry point
```

---

### Task 1: Pin and prove the SQLCipher connection factory

**Files:**
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Create: `crates/vault-v2-engine/Cargo.toml` (`publish = false`)
- Create: `crates/vault-v2-engine/build.rs`
- Create: `crates/vault-v2-engine/src/lib.rs`
- Create: `crates/vault-v2-engine/src/main.rs`
- Create: `crates/vault-v2-engine/src/engine/mod.rs`
- Create: `crates/vault-v2-engine/src/engine/process.rs`
- Create: `crates/vault-v2-engine/src/engine/ffi.rs`
- Create: `crates/vault-v2-engine/src/engine/secret.rs`
- Create: `crates/vault-v2-engine/src/engine/sqlcipher.rs`
- Create: `crates/vault-v2-engine/tests/public.rs`
- Create: `crates/vault-v2-engine/tests/process.rs`
- Create: `crates/vault-v2-engine/tests/ui.rs`
- Create: five independent `crates/vault-v2-engine/tests/ui/*.rs` fixtures and
  their reviewed Rust-1.97 `.stderr` files
- Create: `scripts/qualify-vault-v2.sh`
- Modify: `.github/workflows/ci.yml`

**Private process interfaces:** `CryptoProcessOwner`, `DbKey`,
`RawSqlcipherKey`, `SqlcipherProfile`,
`ConnectionMode::{ReadWriteCreateInternal,ReadWrite,ReadOnlyRecovery}`, and a
closed `open_sqlcipher(&CryptoProcessOwner, ...)` factory. The library target
contains none of these types. No public connection or raw handle escapes.

- [ ] **Step 1: Write failing profile, opacity, and raw-key tests**

Add exact named tests for:

- `sqlcipher_runtime_is_exactly_4_14_0_for_released_profile`;
- `database_header_and_journal_do_not_contain_plaintext_canary`;
- `wrong_dbk_fails_without_schema_or_plaintext`;
- `connection_profile_matches_every_required_pragma`;
- `extensions_attach_writable_schema_and_dynamic_sql_are_denied`;
- `oversized_values_and_sql_fail_at_fixed_limits`; and
- `raw_dbk_never_reaches_sql_text_logs_or_environment`;
- `fresh_process_ignores_hostile_openssl_configuration`; and
- `prior_load_config_process_is_rejected_by_profile_gate`.

The tests must derive evidence independently rather than trust a production
“observed profile” or operation transcript. Add exact 67-byte encoder vectors
for DBKs containing `00`, `0f`, `ab`, and `ff`; verify every byte and the
absence of NUL, and scan DB/journal for raw DBK, 64-byte hex, and full token.
Read cipher/provider/provider version, SQLCipher/SQLite runtime versions,
compile options, every available C connection setting, and every SQLite limit
through the actual engine/C readback. SQLite has no API that reads back the
original `sqlite3_open_v2` flags, so exact `syn` AST/call-site inspection proves
the fixed flag expression while missing-file, `NOFOLLOW`, URI, read-only, and
create-only behavior tests prove its effects. The AST gate also proves the
OpenSSL bootstrap is the first project-controlled crypto action, the raw-key
call is the first database action after open, and there is exactly one unsafe
FFI module with the two named entry points. Each resource limit has boundary
and boundary-plus-one coverage.
Source/AST gates prove the plaintext-header setter, arbitrary SQL, raw handle,
backup/export/copy, and additional OpenSSL/SQLite unsafe calls are absent.

Normal-open tests cover missing file with no creation, symlink/`NOFOLLOW`, URI
rejection, wrong key, WAL/profile mismatch, and an unexpected object in the
Task 1 empty pre-schema container. They compare pre/post file hashes to prove
the factory fails without repair. Create tests prove the exact fixed
create/drop page-forcing transaction is the only initializer schema operation,
an interrupted initializer never returns, and the completed empty container
rejects a wrong key after independent reopen. Journal tests scan both old and
replacement canaries while a real transaction is active and cover commit plus
rollback.

Use `matches!` for typed errors, not `assert_eq!` on a plaintext-bearing success
type. Put compile-fail tests on the public API proving downstream code cannot
name `DbKey`, reach the raw handle/shim, construct create mode, or choose a
cipher. Add `static_assertions::assert_not_impl_any!` for
both `DbKey` and `RawSqlcipherKey`: `Clone`, `Debug`, `Display`, `Serialize`,
and `DeserializeOwned`; positively assert the intended zeroize-on-drop traits.
Assert `HardenedConnection: !Send + !Sync` inside the private binary unit tests.
The five public-surface trybuild cases are independent, and their final stderr must fail for
privacy, not an unresolved crate or test-harness dependency.

- [ ] **Step 2: Capture genuine RED**

```bash
./scripts/qualify-vault-v2.sh cargo test -p sovereign-vault-v2-engine --bin sovereign-vault-v2-engine engine::sqlcipher::tests --frozen -- --nocapture
./scripts/qualify-vault-v2.sh cargo test -p sovereign-vault-v2-engine --test public --frozen -- --nocapture
./scripts/qualify-vault-v2.sh cargo test -p sovereign-vault-v2-engine --test process --frozen -- --nocapture
./scripts/qualify-vault-v2.sh cargo test -p sovereign-vault-v2-engine --test ui --frozen -- --nocapture
```

First create the private crate, exact dev dependencies, and locked graph so the
harness itself compiles. Then run `-- --list` for filtered targets and reject
`running 0 tests`. Save missing-engine-API stderr; missing `trybuild`,
`static_assertions`, package, or lockfile is not valid RED. The stacked PR must
target `main` (or an explicitly amended CI trigger) because current CI runs
only for `main` pull requests; absence of a workflow run is never evidence.

In the same Task 1 slice, change CI so dependency acquisition is an explicit
`cargo fetch --locked` step, then run engine/workspace qualification only via
`./scripts/qualify-vault-v2.sh full` in a job that restores no target cache.
Any ordinary cached product-only job must pass
`--exclude sovereign-vault-v2-engine`. CI directly invokes the wrapper's
negative environment matrix and exact host/target check. A bare workspace
Cargo command is not accepted as engine evidence.

- [ ] **Step 3: Pin minimal dependencies and implement one unsafe shim**

Add only Task 1 dependencies. Confirm `rusqlite = 0.40.2` resolves
`libsqlite3-sys = 0.38.2` and bundled SQLCipher 4.14.0; record the exact feature
tree, crates.io checksums, native-target resolution, vendored amalgamation
hashes, and advisory/delta review. The recorded review includes the 4.15
`sqlcipher_export` fix, SQLite/FTS5 issues, and the OpenSSL 3.6.3 OCSP/TLS
issue with exact non-reachability reasoning rather than “no advisories.” Add a
precise static/call-graph gate rejecting
`sqlcipher_export`, dynamic `ATTACH`, `rusqlite::backup::Backup`,
`Connection::backup`, and database/page-copy calls. Stop on a material
unresolved finding rather than changing sources ad hoc.

Implement the build gate for every listed global and target-prefixed ambient
override. Add negative CI/build tests that set each variable independently and
require a non-zero build before an artifact can qualify. Record approved
target-specific compile options, `cipher_provider`, provider version, and the
vendored source hashes; an unrecognized provider or build profile fails.
Run all qualification Cargo commands through the wrapper and its fresh target
directory. Use `--frozen`; the exact transitive
`libsqlite3-sys` version is guaranteed by the reviewed lockfile, not by
rusqlite's semver declaration alone.

The only unsafe module has exactly two reviewed entry points. First,
`bootstrap_crypto_process` declares the pinned official OpenSSL C ABI/constant,
calls `OPENSSL_init_crypto(OPENSSL_INIT_NO_LOAD_CONFIG, NULL)` as `main`'s
first project-controlled crypto action, checks return `1`, and returns the
private `CryptoProcessOwner`. Second, `open_key_and_harden_connection` accepts
`&CryptoProcessOwner`, the closed path/mode, and `&DbKey`; first calls
`sqlite3_threadsafe()` and requires admitted compile-time value `1`, then calls
raw `sqlite3_open_v2` with the exact flags and, on success, makes
`sqlite3_key` the first post-open SQLite call. This avoids rusqlite's safe open
path, which installs a busy timeout before returning and therefore cannot prove
the required order. The function fills a fixed zeroizing 67-byte buffer with
the official raw-key blob literal
`x'<64 lowercase hex digits>'`, and calls `rusqlite::ffi::sqlite3_key` with
the exact pointer/length as the first post-open database operation. It next calls
`sqlite3_enable_load_extension(handle, 0)` and
`sqlite3_db_config(handle, SQLITE_DBCONFIG_ENABLE_LOAD_EXTENSION, 0, &out)`,
requires both `SQLITE_OK` and `out == 0`, then transfers its solely owned handle
once through `Connection::from_handle_owned`. Every earlier error closes the
handle exactly once. The locked safe `DbConfig` omits option 1005 and the safe
extension helper needs the forbidden feature; no third unsafe entry point or
second raw-handle shim is allowed. Passing 32 arbitrary bytes directly is a
regression to passphrase semantics and is forbidden. Both functions contain
line-by-line safety comments and expose neither arbitrary bytes nor a raw
connection pointer.

The resulting private `HardenedConnection` adds a non-send/non-sync marker and
never crosses the dedicated process thread. After ownership transfer, the
factory sets the fixed busy timeout and remaining safe no-page controls before
authentication. Private static assertions pin this replacement for rusqlite's
bypassed safe-open bookkeeping.

The raw-key call is the first post-open database operation. The factory then
applies only the RFC cipher/C connection-safety settings, installs the closed
authorizer, and sets/reads back every fixed `sqlite3_limit` before page access.
For an existing database it uses the schema count as the first authentication
probe. It verifies profile/journal/page-count state only afterward; Task 3,
not Task 1, defines `application_id`, `user_version`, registry and metadata.
Normal open fails on mismatch without changing the file, converting journal
mode, or creating a missing database. Create mode treats the zero-byte schema
query only as an empty-schema probe. Before any initializer write it sets and
reads back the page ceiling, then runs one fixed create/insert/drop transaction
solely to force an encrypted page, commits/closes, and returns success only
after an independent normal no-`CREATE` reopen authenticates the encrypted
pages, finds an empty schema, and sets/verifies the ceiling again. The bundled
C symbols for extension loading exist, but the rusqlite feature is absent and
the authorizer/API surface keeps them unreachable.

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

Inspect the wrapper-recorded
`./scripts/qualify-vault-v2.sh cargo tree -p sovereign-vault-v2-engine -e features --frozen`
output and bundled C build.
Also save `cargo tree -p sovereign-cli -e features --locked` and require it to
contain none of `sovereign-vault-v2-engine`, `rusqlite`, `libsqlite3-sys`, or
`openssl-sys`. Build the release CLI and inspect symbols/strings for
`sqlite3_key`, `sqlcipher_export`, and `cipher_version`; any match introduced
by Program 1A is a release-boundary failure.
Run the plan-wide gate in Task 6. Commit only after independent review:

```bash
git add Cargo.toml Cargo.lock crates/vault-v2-engine \
  scripts/qualify-vault-v2.sh .github/workflows/ci.yml
git commit -m "feat(vault): add fixed SQLCipher engine"
```

Push, fetch, and verify the exact remote branch contains the commit.

### Task 2: Add typed dual wrappers and read-only recovery

**Files:**
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `crates/vault-v2-engine/Cargo.toml`
- Modify: `crates/vault-v2-engine/src/engine/mod.rs`
- Create: `crates/vault-v2-engine/src/engine/wrappers.rs`
- Create: `crates/vault-v2-engine/src/engine/key_slots.rs`
- Create: `crates/vault-v2-engine/src/engine/platform.rs`
- Create: `crates/vault-v2-engine/src/engine/recovery.rs`
- Modify: `crates/vault-v2-engine/tests/public.rs`
- Add: `crates/vault-v2-engine/tests/ui/recovery_read_only.rs`

**Internal interfaces:** `DeviceDbkAad`, `PwkRecoveryKekAad`,
`RecoveryDbkAad`, `NativeDeviceStore`, internal `TestOnlyDeviceStore`, fixed
`RecoveryRecord`, and `RecoverySession<ReadOnly>`.

- [ ] **Step 1: Write wrapper, root-independence, and typestate RED tests**

Write golden vectors for every typed AAD and one-field mutation of every field.
Add cross-purpose/cross-role/cross-workspace/database/epoch substitution tests,
fresh-nonce tests, direct fallible OS-entropy tests, and dependency upstream
XChaCha/RFC 9106 vectors. Inject an entropy failure before and partway through
each required DBK/KEK/salt/nonce/ID fill and prove no database, sidecar, keyring
record, wrapper, temporary file, or other durable state was created or changed;
the returned error and its Debug/source chain are value-free.

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
./scripts/qualify-vault-v2.sh cargo test -p sovereign-vault-v2-engine --bin sovereign-vault-v2-engine engine::wrappers::tests --frozen -- --nocapture
./scripts/qualify-vault-v2.sh cargo test -p sovereign-vault-v2-engine --bin sovereign-vault-v2-engine engine::key_slots::tests --frozen -- --nocapture
./scripts/qualify-vault-v2.sh cargo test -p sovereign-vault-v2-engine --bin sovereign-vault-v2-engine engine::recovery::tests --frozen -- --nocapture
./scripts/qualify-vault-v2.sh cargo test -p sovereign-vault-v2-engine --test ui recovery_read_only --frozen -- --nocapture
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
git add Cargo.toml Cargo.lock crates/vault-v2-engine
git commit -m "feat(vault): add typed dual DBK wrappers"
```

### Task 3: Implement the closed transactional business schema

**Files:**
- Create: `crates/vault-v2-engine/src/engine/schema.rs`
- Modify: `crates/vault-v2-engine/src/engine/mod.rs`
- Modify: `crates/vault-v2-engine/src/engine/sqlcipher.rs`
- Modify: `crates/vault-v2-engine/src/engine/recovery.rs`
- Modify: `crates/vault-v2-engine/tests/public.rs`
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
- every workspace/database/object ID is exactly 32 bytes, matching the
  external binding and typed AAD profile;
- metadata `key_epoch`, the authenticated sidecar `db_key_epoch`, and the
  profile constant must all be exactly `1`; any other value fails open rather
  than being treated as a rotatable current profile;
- with trigger depth fixed at zero and schema/user triggers forbidden, the
  sealed delete path removes all exact
  child chunks and then the parent object in one immediate transaction;
  direct parent deletion, partial deletion, wrong identity, affected-row-count
  mismatch, and every intervening failpoint leave the complete old object;
- unknown/caller/plugin/model tags cannot compile or are rejected before SQL;
- rollback journal recovery yields the old or complete new transaction;
- plaintext names/content never appear in database header, rollback journal,
  sidecar, path, error, or Debug output; and
- authority, identity, signing, session, credential, freshness, recovery, and
  pending-effect data have no schema/adaptor route.

- [ ] **Step 2: Capture RED**

```bash
./scripts/qualify-vault-v2.sh cargo test -p sovereign-vault-v2-engine --bin sovereign-vault-v2-engine engine::schema::tests --frozen -- --nocapture
./scripts/qualify-vault-v2.sh cargo test -p sovereign-adversarial-tests --test security_invariants vault_v2_closed_schema --frozen -- --exact --nocapture
```

List exact names first and reject zero-test results.

- [ ] **Step 3: Implement the minimum static schema**

Implement the exact RFC DDL and constants: `application_id=0x53464f53`,
`user_version=2`, registry version `1`, and only
`vault_metadata_v1`, `business_object_v1`, and `business_chunk_v1`. Create the
tables, one metadata row, and both application-version PRAGMAs in one immediate
transaction, close, and verify them through normal open. Use only the RFC's
static SQL, foreign keys, fixed `CHECK`s, prepared parameters, and one immediate
transaction per logical mutation. Verify exactly one metadata row, identity,
versions, exact key epoch `1` against the authenticated sidecar, DDL shape,
contiguous chunk count/order, and overflow-safe byte sum on every open/read.
The authorizer's closed function set includes only the exact
DDL/check/query functions required by this schema. No view, trigger, virtual
table, attach, caller SQL, dynamic identifier, or generic metadata/value table.
Foreign keys use immediate `NO ACTION`, never `RESTRICT` or cascade; the typed
object-delete transaction deletes and verifies the complete child set before
deleting the parent. Real behavior tests set
`SQLITE_LIMIT_TRIGGER_DEPTH=0`, prove a direct parent delete with children is
rejected, and prove the exact child-first transaction succeeds. The static
schema, authorizer, and DDL verification separately forbid schema/user triggers.

Record a current check of SQLite's official `magic.txt` for the provisional
`0x53464f53` value. This internal pre-release engine does not claim the value
is registered; stable external format publication is blocked until registration
or an RFC-defined ID migration.

The sealed constructor—not callers—assigns the one object tag and backup
disposition.
The read API returns typed zeroizing buffers and verifies chunk count/length
before allocation. Recovery uses the same typed query layer under its stronger
authorizer.

Changing any constant, DDL token, object tag, or backup disposition requires a
new RFC-defined versioned migration; SQLite's internal `schema_version` is
observed only as SQLite state and is never used as the application version.

- [ ] **Step 4: GREEN and fault/mutation matrix**

Run failpoints at begin, metadata/object/chunk insert, commit, journal sync,
close, reopen, and integrity check. Mutate a check constraint, enable a generic
tag, and skip a chunk; tests must fail. Restore and rerun.

- [ ] **Step 5: Full gate, review, commit, push**

```bash
git add crates/vault-v2-engine tests/adversarial
git commit -m "feat(vault): transact closed business objects"
```

### Task 4: Add private no-follow staging and the real v1 importer

**Files:**
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `crates/vault-v2-engine/Cargo.toml`
- Create: `crates/vault-v2-engine/src/engine/storage.rs`
- Create: `crates/vault-v2-engine/src/engine/legacy.rs`
- Create: `crates/vault-v2-engine/src/engine/migration.rs`
- Modify: `crates/vault-v2-engine/src/engine/mod.rs`
- Modify: `crates/vault-v2-engine/tests/public.rs`
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
./scripts/qualify-vault-v2.sh cargo test -p sovereign-vault-v2-engine --bin sovereign-vault-v2-engine engine::legacy::tests::missing_key_never_regenerates --frozen -- --exact --nocapture
./scripts/qualify-vault-v2.sh cargo test -p sovereign-vault-v2-engine --bin sovereign-vault-v2-engine engine::migration::tests::role_keys_block_activation --frozen -- --exact --nocapture
./scripts/qualify-vault-v2.sh cargo test -p sovereign-adversarial-tests --test security_invariants vault_v2_failure_never_falls_back --frozen -- --exact --nocapture
```

Confirm current `Vault::init` regenerates the absent key or lacks the new typed
failure; save that RED evidence.

- [ ] **Step 3: Implement read-only import into side-by-side staging**

Add only the exact Task 4 dependencies listed above: `cap-std`, `cap-fs-ext`,
and legacy-reader-only `aes-gcm`. Retain a capability directory for sidecar and
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
git add Cargo.toml Cargo.lock crates/vault-v2-engine tests/adversarial
git commit -m "feat(vault): verify side-by-side legacy import"
```

### Task 5: Make real platform assurance mandatory

**Files:**
- Create: `.github/workflows/vault-platform.yml`
- Modify: `.github/workflows/ci.yml`
- Modify: `crates/vault-v2-engine/src/engine/platform.rs`
- Modify: `crates/vault-v2-engine/src/engine/storage.rs`
- Add platform integration tests under `crates/vault-v2-engine/tests/`
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

Jobs assert exact native triples, not OS/architecture classes. Task 1 admits
only `x86_64-unknown-linux-gnu`; Task 5 may add exactly
`aarch64-apple-darwin` and `x86_64-pc-windows-msvc` after each runner proves
`HOST == TARGET`, the native key store, ABI/profile, and all mandatory tests.
Musl, GNU Windows, x86_64 macOS, aarch64 Linux, 32-bit, PowerPC, embedded, and
every other target remain `Unavailable` until a separate review names the
exact triple, proves the locked primitive/toolchain assumptions, and adds a
mandatory real native job. Cross-compilation is never qualification.

Add native permission/no-follow/reparse, process race, rollback-journal crash,
sidecar replacement, directory durability, and close/reopen tests. Jobs fail if
the service cannot be initialized; they do not skip.

- [ ] **Step 2: Capture RED on draft PR**

The workflow file must run on `pull_request`, manual dispatch, and protected
main pushes. Record all three missing/failing check names before implementation.
Do not place platform-only checks behind a generic success aggregate that can
go green when one job is skipped.

- [ ] **Step 3: Implement isolated native jobs and lock feature evidence**

Before code, run the qualification wrapper and save
`./scripts/qualify-vault-v2.sh cargo tree -p sovereign-vault-v2-engine -e features --frozen`
separately on all three exact native triples. `keyring = =4.1.5` remains
`default-features=false, features=[v1]`.
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
git add .github/workflows crates/vault-v2-engine scripts
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
- Add final negative tests/searches under `crates/vault-v2-engine/tests/` and
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
./scripts/qualify-vault-v2.sh full
./scripts/check-file-size.sh
cargo build -p sovereign-cli --release --locked
cargo tree -p sovereign-cli -e features --locked
git diff --check HEAD --
```

`full` runs, in one sanitized child environment and one newly created target,
workspace Clippy with all targets/features and `-D warnings`, all workspace
tests, engine docs, the frozen engine feature tree, and the engine
release/process checks, all with `--frozen`. The script deletes the
target on exit and emits the tool/source/profile evidence manifest. It rejects
an unknown subcommand rather than becoming a general shell launcher.

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
  crates/vault-v2-engine tests/adversarial Cargo.toml Cargo.lock .github/workflows scripts
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

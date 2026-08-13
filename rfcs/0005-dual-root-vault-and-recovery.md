# RFC 0005: Transactional Dual-Root Vault and Recovery

**Status:** Draft; approved implementation target
**Implementation:** None
**Maturity:** Target design; no current protection claim
**Security impact:** Critical

## Summary

Vault v2 is one SQLCipher database with two and only two unlock domains:

1. a normally online device domain whose admitted native credential store
   protects a random device key-encryption key (`DeviceKEK`); and
2. an explicitly offline recovery domain in which Argon2id derives a password
   wrapping key (`PWK`) that unwraps an independent random `RecoveryKEK`.

The `DeviceKEK` and `RecoveryKEK` independently wrap the same random 32-byte
SQLCipher database key (`DBK`) with XChaCha20-Poly1305. SQLCipher encrypts and
authenticates the database pages. Sovereign does not implement its own object
encryption, encrypted manifest, transaction journal, or multi-file database
commit protocol.

The database contains only closed, typed business objects. Device identity,
authority, approval and audit signing keys, session/ratchet keys, effect
authority, credentials, and rollback anchors remain distinct domains and MUST
NOT be derived from or stored under the DBK.

Program 1A implements and tests only the internal engine, native device-store
adapters, recovery read-only path, and a reader-first legacy importer. It does
not expose product enrollment, migrate a live workspace, select v2, or claim a
workspace is protected. Product activation requires all of:

- Program 1B0 filtered-backup and restore mechanics, followed by a Program 1B1
  clean-machine qualification of the exact `PendingV2` candidate;
- Program 1C0 authenticated owner presence and Program 1C1 identity/role-key
  custody and handoff;
- passing mandatory native-store/durability jobs on the enabled platform; and
- closure of the whole-workspace plaintext inventory; followed by
- one exact, one-use Program 1D activation authorization issued through the
  Program 1C0 owner-presence boundary.

The dedicated age identity used by Programs 1B0/1B1 is an offline backup transport
recipient, not a third Vault unlock root. Clean restore requires both the age
identity and recovery-password chain.

This RFC replaces the custom per-object/encrypted-manifest design previously
considered for [RFC 0004](0004-data-sovereignty-boundaries.md). It deliberately
uses SQLCipher's mature transactional and recovery machinery to reduce the
amount of new cryptographic and crash-consistency code in Sovereign.

## Normative vocabulary and maturity

The keywords MUST, MUST NOT, SHOULD, SHOULD NOT, and MAY are requirements for
the supported Vault boundary.

`Current` means implemented in merged code and verified by passing repository
and required platform tests. `Target` is an approved but unshipped contract.
`Research` requires further validation. Every feature here is `Target` unless
explicitly labelled otherwise. A tagged release and a product claim are
separate gates.

## Goals

- A copied Vault database, sidecar, and workspace directory are not decryptable
  with a plaintext key stored beside them.
- Normal business writes use database transactions with a fixed SQLCipher
  profile and fail-closed open sequence.
- Loss of an enrolled device remains recoverable with an offline recovery kit.
- Device and recovery compromise are independent.
- Password and device-wrapper rotation do not rewrite the database.
- Recovery unlock is structurally read-only until separately authorized device
  enrollment completes.
- Legacy import is explicit, bounded, side-by-side, crash recoverable, and never
  an authentication fallback.
- Backup is a filtered recovery database, not a copy of the live database.
- CI tests failure semantics without treating injected stores or simulators as
  production assurance.

## Non-goals

- Protecting plaintext after a legitimately unlocked or compromised process
  reads it.
- Guaranteed erasure from snapshots, swap, crash dumps, flash media, old
  backups, or SQLCipher free pages.
- Hiding database size, page count, file timestamps, open timing, or access
  patterns.
- Storing authority, identity, signing, session, credential, or effect secrets
  in the business-object database.
- Making a password high-entropy key material.
- Treating AEAD or SQLCipher integrity as freshness or full-device rollback
  detection.
- Inventing cryptographic primitives, a database pager, or an age/SQLCipher
  variant.

## Threat model and trust boundary

Vault v2 treats the database, wrapper sidecar, format selector, legacy files,
backup archive, and every parsed field as attacker-controlled bytes. It
protects a committed v2 database against offline workspace theft, subject to
the stated metadata leakage. It does not retroactively protect disks,
snapshots, exports, or backups that retained the legacy raw key or plaintext
workspace records.

The OS, boot chain, native credential service, SQLCipher process, SQLite pager,
and Vault core are trusted while device unlock is used. `OsProtected` means a
DeviceKEK may be returned to trusted Rust memory; it does not mean hardware
backed or non-exportable. A compromised unlocked OS is outside this boundary.

Recovery resists offline guessing only to the extent of password entropy and
the fixed Argon2id work profile. The recovery kit is an offline high-value
secret. A complete older, internally valid workspace can be replayed; rollback
limitations are explicit below.

## Architecture and key hierarchy

```text
Device unlock domain                       Offline recovery domain
--------------------                       -----------------------
NativeDeviceStore                          recovery password
        |                                         |
        | returns random DeviceKEK                | Argon2id
        v                                         v
DeviceKEK -- XChaCha20-Poly1305 --> DBK     PWK (ephemeral)
                                               |
                                               | XChaCha20-Poly1305
                                               v
                                          RecoveryKEK (random)
                                               |
                                               | XChaCha20-Poly1305
                                               v
                                              DBK

DBK (random 32 bytes) -- raw-key sqlite3_key --> SQLCipher database
```

### Key requirements

- Every `DeviceKEK`, `RecoveryKEK`, and `DBK` is 32 uniformly random bytes from
  the operating-system CSPRNG. Passwords, machine IDs, hostnames, usernames,
  hardware serials, and hashes of them are forbidden key sources.
- The DBK has an opaque random `database_id` and fixed `db_key_epoch = 1` in
  this profile. IDs are never key fingerprints.
- Every open receives an opaque `ExpectedVaultBinding` from outside the
  database, sidecar, selector, and legacy tree. It binds workspace ID, expected
  format state, activation epoch, and expected database ID. Program 1A can
  issue only an internal staging binding; Program 1D must source an activated
  binding from the independently authenticated owner/workspace registry
  established by Programs 1C0/1C1 and consumed by 1D. Fields read from
  `vault.slots`, `vault.format`,
  the database, a path, or caller text cannot establish this expectation.
- `workspace_id`, `database_id`, every wrapper/record/key ID, and every
  business `object_id` are independent random 32-byte
  protocol identifiers. They are never paths, labels, counters, or hashes of
  keys/content.
- DeviceKEK and RecoveryKEK independently wrap only the complete DBK record.
- PWK wraps only RecoveryKEK. It never wraps the DBK or database pages.
- Keys are used for exactly one algorithm and purpose.
- Program 1A does not implement DBK rotation. SQLCipher rekey plus sidecar
  publication requires a separately journaled, reviewed protocol. Device and
  recovery wrappers may rotate around the unchanged DBK.
- Secret holders MUST NOT implement `Clone`, `Debug`, `Display`, `Serialize`,
  or `Deserialize`; they zeroize supported buffers on drop. This is
  defense-in-depth, not a secure-erasure claim.

## Fixed cryptographic profile

| Purpose | Primitive | Required profile |
| --- | --- | --- |
| Password KDF | Argon2id | `argon2id-rfc9106-lowmem-v1`: v=19, m=65,536 KiB, t=3, p=4, 16 random salt bytes, 32-byte output |
| Software key wrappers | XChaCha20-Poly1305-IETF | 32-byte key, fresh random 24-byte nonce, full 16-byte tag |
| Database | SQLCipher | released rusqlite 0.40.2 bundle: SQLCipher exactly 4.14.0; fixed profile below |
| Backup transport | age v1 | standard X25519 recipient mode; Programs 1B0/1B1 |

All keys, protocol IDs, salts, and XChaCha nonces come through a direct,
fallible operating-system CSPRNG call and are never caller supplied, derived,
or reused. Any entropy failure zeroizes supported partial buffers and occurs
before any database, sidecar, key-store record, wrapper, or temporary state is
published. Production has one closed system-entropy adapter and no custom RNG
backend; only crate-private tests inject failures. Authentication completes
before plaintext is released. Only maintained upstream implementations and
published formats are used; Sovereign implements no primitive.

### Argon2id policy

The recovery record stores only profile tag `1` and a random 16-byte salt. Tag
`1` maps exactly to `argon2id-rfc9106-lowmem-v1`. Unknown tags, extra cost
fields, incorrect lengths, or another Argon suite are rejected before KDF work.
The exact confirmed UTF-8 password bytes are consumed without trimming,
case-folding, or silent normalization changes.

The RecoveryKEK wrapper is necessarily an offline guessing oracle. There is no
separate password verifier, hint, or security question. Local rate limiting
does not stop offline attacks. Before product activation, the profile is
benchmarked on the weakest supported recovery device. A changed cost requires
a new reviewed profile tag, not mutation of tag `1`.

## Typed wrapper AAD

A generic, optional-field context object is forbidden. Each wrapper has a
distinct internal Rust AAD type. Integers are unsigned big-endian, every ID in
this profile is exactly 32 bytes, literals are exact ASCII without a
terminator, and no field is optional:

```text
DeviceDbkAad =
  "sovereign:vault:v2:device-dbk-wrap" ||
  u16(wrapper_version=1) || u16(suite_version=1) ||
  workspace_id[32] || database_id[32] || protector_record_id[32] ||
  device_wrapper_id[32] || u64(db_key_epoch) ||
  recovery_slot_commitment[32]

PwkRecoveryKekAad =
  "sovereign:vault:v2:pwk-recovery-kek-wrap" ||
  u16(wrapper_version=1) || u16(suite_version=1) ||
  workspace_id[32] || database_id[32] || recovery_record_id[32] ||
  recovery_kek_id[32] || u16(argon_profile_tag=1) || argon_salt[16]

RecoveryDbkAad =
  "sovereign:vault:v2:recovery-dbk-wrap" ||
  u16(wrapper_version=1) || u16(suite_version=1) ||
  workspace_id[32] || database_id[32] || recovery_record_id[32] ||
  recovery_kek_id[32] || u64(db_key_epoch) || u8(database_role)
```

`database_role` is `1=live` or `2=filtered-recovery-snapshot`. A snapshot uses
a new random database ID and DBK and additionally binds its random `snapshot_id`
through a version-2 `RecoveryDbkAad`; Program 1B0 must define that exact additive
encoding before implementation. It MUST NOT reuse role `1` with an omitted ID.

Golden vectors bind every type and field boundary. One-field mutation,
cross-purpose substitution, wrong role, and wrapper swapping all fail.

`recovery_slot_commitment` is
`SHA-256("sovereign:vault:v2:recovery-slot-commitment" || u16(1) ||
recovery_subrecord_jcs_bytes)`, where `recovery_subrecord_jcs_bytes` is the
exact canonical JCS byte sequence of the complete recovery subrecord defined
below. Device open first bounds and canonicalizes that subrecord, recomputes
the commitment, and then uses it in `DeviceDbkAad`; deletion, corruption, or
substitution of recovery material therefore makes normal device unwrap fail
instead of silently leaving a workspace without its enrolled recovery route.
There is deliberately no reciprocal device-slot commitment inside recovery AAD,
avoiding a circular construction.

## SQLCipher transactional database

### Exact dependency and build

The first implementation pins:

```toml
rusqlite = { version = "=0.40.2", default-features = false,
  features = ["bundled-sqlcipher-vendored-openssl", "hooks", "limits"] }
openssl-sys = { version = "=0.9.117", default-features = false }
```

These dependencies live only in the private, `publish = false` workspace crate
`sovereign-vault-v2-engine`. The shipped CLI and the legacy
`sovereign-vault` crate have no dependency edge to that engine during Program
1A. This physical separation is part of the admission boundary: the default
release CLI dependency tree and binary MUST contain no engine, rusqlite,
libsqlite3, OpenSSL, `sqlite3_key`, `sqlcipher_export`, or `cipher_version`
symbol introduced by this profile. Program 1D may add a narrow product
dependency only after this RFC has admitted the newer exact SQLCipher profile
and all activation prerequisites have passed. A downstream Cargo feature is
not an adequate substitute for this dependency-graph separation.

That released lockfile resolution currently bundles SQLCipher exactly `4.14.0`
through `libsqlite3-sys = 0.38.2`; it MUST NOT be described as 4.17.0 or
production-ready. SQLCipher 4.15 fixed a defensive-mode bypass in
`sqlcipher_export` and recommends upgrade. Program 1A may use 4.14.0 only for
the internal, non-activated engine while repository call sites/references to
`sqlcipher_export`, dynamic `ATTACH`, the rusqlite backup API, and every
export/copy path are statically absent. The closed SQL authorizer separately
denies `sqlcipher_export` invocation and `ATTACH`; the bundled symbol's mere
presence is not misreported as removable by a Rust call-site gate. Program 1B0,
product activation, and a production claim are blocked until a future RFC
amendment pins exactly one reviewed released Rust binding and one exact
SQLCipher release (4.17.0 or a later release named explicitly by that
amendment) and that exact profile passes all gates,
or the owner separately accepts an exact-source dependency plan after review.
A semver range or runtime “at least” check is not an admitted profile.

The reviewed candidate revision beginning `62648175` carries 4.17.0 but is
unreleased and unsigned in this dependency path; it MUST NOT be selected
silently or pinned from this abbreviated identifier. An exact revision/vendor
path requires independent dependency diff and supply-chain
review, reproducible hashes/builds, license evidence, upstream source
provenance, and this RFC's dependency-profile amendment. A material unresolved
advisory blocks rather than triggers improvised build plumbing.

Startup and CI check `PRAGMA cipher_version`, `cipher_provider`, provider
version, and an approved target-specific `PRAGMA compile_options` profile;
anything else fails rather than silently continuing. The locked bundled source
currently compiles SQLite's load-extension machinery, so this RFC does not
claim the symbols are absent. The `rusqlite/load_extension` feature is absent;
the factory explicitly sets `SQLITE_DBCONFIG_ENABLE_LOAD_EXTENSION=0`, checks
the result, denies the SQL function in its authorizer, and exposes neither the
C loading API nor a raw connection. No supported call may enable it.

The same vendored build may compile upstream built-ins such as JSON, FTS,
RTree, DBSTAT, or SOUNDEX. Their presence is recorded rather than hidden. The
application schema contains no virtual table, view, trigger, or generated
expression; the Rust feature surface exposes no vtab or caller-function
registration; the authorizer denies every function outside a closed internal
allowlist and every virtual-table/schema construction; and product callers
cannot submit SQL. A new compile option or newly reachable built-in fails the
admitted profile.

The only qualification entry point is the checked-in
`scripts/qualify-vault-v2.sh` command wrapper. It rejects ambient dependency
overrides including
`LIBSQLITE3_SYS_USE_PKG_CONFIG`, `LIBSQLITE3_FLAGS`,
`SQLITE_MAX_VARIABLE_NUMBER`, `SQLITE_MAX_EXPR_DEPTH`, `SQLITE_MAX_COLUMN`,
`OPENSSL_NO_VENDOR`, `SQLCIPHER_LIB_DIR`, `SQLCIPHER_INCLUDE_DIR`,
`SQLCIPHER_STATIC`, `OPENSSL_DIR`, `OPENSSL_LIB_DIR`, `OPENSSL_INCLUDE_DIR`,
`OPENSSL_CONFIG_DIR`, `OPENSSL_LIBS`, `OPENSSL_STATIC`, `OPENSSL_SRC_PERL`,
`PERL`, `OPENSSL_RUST_USE_NASM`, `PKG_CONFIG_*`, `VCPKGRS_DYNAMIC`, and every
relevant Cargo target-prefixed form read by `libsqlite3-sys` or `openssl-sys`.
`openssl-src` reads `OPENSSL_SRC_PERL`, `PERL`, and
`OPENSSL_RUST_USE_NASM` directly rather than through the target-prefix helper;
the wrapper rejects those exact global forms. The gate uses a closed
allowlist: any newly observed dependency-shaping or build-tool-selection
variable fails until this RFC is amended. It removes Perl injection variables,
Rust/linker flags, and unapproved tool overrides; constructs a positive-
allowlist child environment; resolves and records canonical path, version, and
SHA-256 for Cargo, rustc, C compiler, archiver, ranlib, Perl, and make; creates
and owns a new `CARGO_TARGET_DIR`; and runs Cargo `--frozen --offline` only
after a separate `cargo fetch --locked` acquisition step. Its `full` mode
reuses the fresh target only within that one invocation and deletes it on exit.
The child starts from `env -i` and receives only fresh home/temp directories,
reviewed Cargo/rustup homes, locale/reproducibility values, exact absolute tool
paths, the exact build target, offline mode, and a PATH composed from reviewed
Perl/make/linker directories. Repository/Cargo source replacement is rejected.
Cargo-generated `HOST`, `TARGET`, `OUT_DIR`, and `CARGO_MAKEFLAGS` are allowed
only inside the child, never inherited from the caller.

A downstream crate `build.rs` runs after dependencies may already have built
and Cargo may reuse cached artifacts, so it is defense in depth only; it MUST
NOT be represented as the upstream dependency gate. Every engine-resolving
command, including workspace Clippy/tests/docs and metadata/tree inspection,
runs through the wrapper. CI performs dependency acquisition separately and
then uses an uncached wrapper job; a cached product-only job excludes this
unlinked package. A negative matrix sets every listed variable independently
and proves qualification stops before Cargo. A clean subprocess build then
proves the vendored source/provider is selected. No system
SQLite/SQLCipher, alternate crypto provider, or runtime replacement is
accepted. Cargo.lock, enabled features, vendored C compiler flags, provider
identity/version, notices, hashes, and the pre-implementation advisory
decision are recorded in the verification ledger.

Task 1 admits exactly the native triple `x86_64-unknown-linux-gnu`. The wrapper
verifies the rustc host, rejects incoming target/linker selection, sets that
exact target, and requires Cargo build-script `HOST == TARGET`. Musl, wasm,
cross-builds, Windows, macOS, and every other ABI fail the engine gate until a
later amendment names the exact triple and a mandatory real native job passes.
This does not block product builds because the product has no dependency on the
engine. Task 5 may add only the separately qualified triples named there.

The pinned graph resolves statically linked OpenSSL 3.6.3, whose configuration
loading is process-global. Program 1A is therefore a binary-first dedicated
experimental process, not a reusable database library. `main` first calls the
sole private process bootstrap and receives a non-constructible
`CryptoProcessOwner`; all database opens require that token. DBK, raw handle,
connection, and FFI code compile only into the process target. The package's
library target exposes only a value-free protocol/version surface.

`openssl-sys 0.9.117` does not expose `OPENSSL_init_crypto` or
`OPENSSL_INIT_NO_LOAD_CONFIG`. The one audited FFI module therefore declares
that exact official C ABI and pinned constant locally, backed by the exact
direct `openssl-sys` link/version dependency. Its process-bootstrap entry point
calls `OPENSSL_init_crypto(OPENSSL_INIT_NO_LOAD_CONFIG, NULL)` before dispatch,
requires return `1`, and produces the owner token. An AST/call-graph gate
proves there is no other OpenSSL caller, `openssl_sys::init`, or process
bootstrap and that `main` cannot dispatch first.

Behavioral qualification starts the real binary in fresh subprocesses.
Hostile `OPENSSL_CONF`, include, engine, and module paths must leave the exact
provider/profile unchanged. A separate test-only fresh worker first performs a
real `OPENSSL_INIT_LOAD_CONFIG` call under a profile-changing configuration and
proves the later bootstrap/profile admission fails; an in-memory boolean is
not evidence of actual OpenSSL global state.

The real LOAD_CONFIG negative control is the only additional raw call, is
compiled under `cfg(test)` inside the same FFI module, and is absent from the
normal/release process. The production AST still contains exactly the two
entry points named by this RFC.

Because OpenSSL initialization cannot be retroactively changed, this result
does not authorize in-process product embedding. Program 1D must keep a
dedicated authenticated broker or separately prove one process-wide
initialization owner before every dependency use. The
shipped CLI remains physically unlinked from Program 1A, so it cannot
accidentally become that process-global owner.

### Raw DBK handoff

Sovereign uses one reviewed private FFI module with exactly two unsafe entry
points: the process bootstrap above and this SQLCipher connection-hardening
operation. No unsafe code exists elsewhere. The latter requires
`&CryptoProcessOwner`, first requires `sqlite3_threadsafe() == 1`, owns the
exact raw `sqlite3_open_v2` call, and calls
`sqlite3_key` as the first post-open database operation. It does not use
rusqlite's safe open path, which installs a busy timeout before returning. It
then calls both
`sqlite3_enable_load_extension(handle, 0)` and
`sqlite3_db_config(handle, SQLITE_DBCONFIG_ENABLE_LOAD_EXTENSION, 0, &out)` and
requires `SQLITE_OK` plus `out == 0`, then transfers the solely owned handle
once through `Connection::from_handle_owned`; every earlier error closes it
exactly once. The locked rusqlite `DbConfig` enum does
not expose option `1005`, and its safe extension helper requires the forbidden
`load_extension` feature, so these operations are deliberately consolidated in
the same audited unsafe function rather than spread across additional shims.
No third unsafe entry point exists, and no page-reading call may occur inside
or before this boundary.

The returned private `HardenedConnection` is explicitly `!Send + !Sync`; the
dedicated process thread owns it for its lifetime. After ownership transfer,
the factory sets the fixed busy timeout and remaining safe no-page controls.
This deliberately replaces the rusqlite safe-open bookkeeping that could not
be used without violating raw-key order, and static assertions pin the
replacement.

The official API documentation applies the same key
interpretation rules as
`PRAGMA key`; passing the 32 arbitrary DBK bytes directly would therefore use
passphrase semantics and is forbidden. The wrapper hex-encodes the DBK into the
exact zeroizing ASCII raw-key blob literal `x'<64 lowercase hex digits>'` in a
fixed 67-byte buffer and passes that buffer and exact length directly to
`sqlite3_key`. The DBK is never placed in
SQL text, a Rust `String`, argv, an environment variable, a log, or a generic
configuration map. The wrapper checks the return code and exposes no raw
connection pointer or arbitrary key input to product code.

This 67-byte syntax selects SQLCipher raw-key mode; the random DBK is not
treated as a password and bypasses the password-to-page-encryption-key PBKDF2
step. SQLCipher still derives its separate HMAC key according to the admitted
raw-key profile and still uses its normal encrypted header and per-database
salt; the implementation does not claim that all SQLCipher PBKDF2 work is
absent. Because upstream
does not publish this repository's fixture as an official vector, CI generates
and independently verifies a fixed interoperability fixture with the official
SQLCipher CLI blob-literal syntax, records its cryptographic hash and known
rows, then opens it through the shim. A repository-created database is reopened
with the CLI syntax in the other direction. Both tests assert SQLCipher
build/runtime version and the complete cipher profile. Direct 32-byte input
must fail this interoperability test.

### Exact SQLCipher and SQLite profile

Before any schema or data query, the closed connection factory:

1. opens only a fixed database name inside a newly created, private, verified
   staging directory with the exact applicable flags
   `READ_ONLY` or `READ_WRITE[|CREATE for the internal initializer only]`,
   `NOFOLLOW`, `PRIVATE_CACHE`, `NO_MUTEX`, and `EXRESCODE`; URI,
   shared-cache, create-on-normal-open, and extension flags are absent. The
   connection is never shared concurrently, and tests pin these flags and the
   missing-file/symlink/URI failure behavior;
2. passes the raw DBK to `sqlite3_key` as the first post-open database
   operation, checks
   its return code, and performs no page-reading or page-writing operation yet;
3. sets only the connection cipher profile that SQLCipher requires after keying
   and before first page access: compatibility `4`, page size `4096`,
   AES-256-CBC page
   encryption, HMAC-SHA512 page authentication,
   `cipher_kdf_algorithm=PBKDF2_HMAC_SHA512`, and HMAC enabled. Raw-key mode
   bypasses password-to-DBK derivation, but the profile remains pinned;
4. before page access, sets `cipher_memory_security=ON`, enables defensive mode,
   disables DQS/trusted schema and both extension-loading routes through the C
   configuration APIs, installs the closed authorizer, sets and reads back all
   fixed `sqlite3_limit` ceilings through the no-page-access C API, and verifies
   each returned connection flag and limit;
5. for an existing database, performs the first authentication probe after
   those resource ceilings are active and before any database-mutating PRAGMA:
   `SELECT count(*) FROM sqlite_schema`. A failure closes the handle without
   journal-mode conversion, schema write, fallback, or partial plaintext. For
   a newly created zero-byte database the same query is only an empty-schema
   probe and is not called key authentication. Before any write, the initializer
   sets and reads back `max_page_count=1,048,576`. It then runs exactly
   `BEGIN IMMEDIATE; CREATE TABLE __sovereign_v2_page_probe
   (only_row INTEGER PRIMARY KEY CHECK (only_row = 1)) STRICT;
   INSERT INTO __sovereign_v2_page_probe VALUES (1);
   DROP TABLE __sovereign_v2_page_probe; COMMIT;` and closes. No other schema
   statement is allowed in Task 1. This forces encrypted pages while leaving
   the application schema empty. It then uses the normal
   no-`CREATE` factory to reopen and authenticate the resulting encrypted
   pages, require zero application objects, and set/verify the page ceiling
   again. An interrupted initializer never returns an admitted connection, and
   the completed database must reject a wrong DBK on that independent reopen;
6. after the probe, reads back `cipher_status`, cipher/provider versions and
   every observable cipher profile value. The encrypted-header setting remains
   its SQLCipher default; the factory does **not** call the
   `cipher_plaintext_header_size=0` setter and instead verifies readback `0`;
7. for an existing database, reads and verifies `journal_mode=DELETE` without
   converting it. It first rejects `page_count > 1,048,576`, then sets the
   per-connection `max_page_count=1,048,576` ceiling and verifies the returned
   value; `max_page_count` is not treated as persistent metadata. Task 1 owns
   only the encrypted container/profile. Task 3 defines the exact
   `application_id`, `user_version`, registry, and metadata schema and then
   adds their post-authentication verification. SQLite's internal
   `schema_version` cookie is never used as an application version; and
8. sets and verifies the remaining connection/runtime controls
   `temp_store=MEMORY`, `synchronous=FULL`, `foreign_keys=ON`, and the
   authorizer's final deny set before returning the typed connection.

The default encrypted SQLCipher header is mandatory; no plaintext-header setter
is called and readback must be exactly `0`. WAL is disabled. Rollback journal
plus `synchronous=FULL` is the v1 durability profile. Database and rollback
journal use fixed names inside the verified private directory; temporary SQL
storage is memory-only.
Readback `0` alone does not prove the plaintext-header setter was absent: a
source/AST gate and ordered no-page operation test separately forbid any
`cipher_plaintext_header_size` assignment. Normal-open profile or journal
mismatches compare pre/post file hashes and fail without repair; a successful
readback after silent conversion is not accepted evidence.
`cipher_integrity_check` and SQLite `integrity_check` run after initialization,
migration, backup construction, and before activation; routine open uses the
authenticated schema probe and the exact profile/container checks available at
that implementation stage. Task 3 adds the fixed application metadata checks.

SQLCipher is responsible for page encryption and transaction recovery. This
RFC does not claim power-loss guarantees beyond the tested filesystem/platform
behavior, and mandatory fault jobs remain release gates.

The stock SQLite VFS is path based; a Rust capability directory does not turn
SQLite's journal opens into descriptor-relative operations. Sovereign therefore
uses `SQLITE_OPEN_NOFOLLOW`, creates rather than adopts the private staging
directory, rejects symlinks/reparse points and broad permissions, and treats a
malicious concurrent same-user process that can rename that directory as part
of the trusted-OS/process boundary. A custom VFS is rejected for v1 because its
additional unsafe pager/filesystem TCB outweighs that unclaimed protection.

### Parser and resource ceilings

`rusqlite`'s `limits` API sets these connection limits before untrusted data is
processed:

| Limit | v1 value |
| --- | ---: |
| SQL length | 64 KiB; all statements are static |
| One SQL value / row payload | 16 MiB |
| Columns | 128 |
| Expression depth | 32 |
| Compound SELECT terms | 16 |
| Bound variables | 999 |
| Trigger depth | 0; schema/user triggers and cascading foreign-key actions are forbidden |
| Attached databases | 0 |
| LIKE/GLOB pattern | 256 bytes |
| Worker threads | 0 |
| Database pages | at most 1,048,576 pages (4 GiB at 4096 bytes) |

The schema additionally limits one business object to 256 MiB represented by
at most 64 fixed 4 MiB chunks, one transaction's aggregate new payload to
256 MiB, 10,000 objects per workspace transaction, a
1,024-byte recovery password input, a 256 KiB wrapper sidecar, an 8 MiB legacy
manifest, a 32 MiB legacy entry JSON, and a 16 MiB legacy decrypted entry.
The factory sets and verifies `max_page_count`, rejects an existing database
already above the cap, and the authorizer denies later changes. Counts, sums,
and lengths are checked with overflow-safe arithmetic before allocation or SQL.
Deployments may lower but not raise these values under profile v1.

## Closed business-object schema

The SQLCipher database contains fixed metadata plus business objects issued by
sealed authoritative adapters. The initial registry is:

| Core object type | Stable tag | Backup disposition |
| --- | ---: | --- |
| `BusinessStateV1` | 1 | `BackupEligible` |
| `VentureProfileV1` | 2 | `BackupEligible` |

Program 1A Task 3 fixes the first application schema exactly. The SQLite
`application_id` is `0x53464f53` (`SFOS`), `user_version` is `2`, and the
closed object-registry version is `1`. These values are set in the same
transaction that creates the following static DDL; routine open verifies all
three after raw-key authentication and before returning typed access:

`0x53464f53` is a pre-release project identifier and is not represented as an
assigned SQLite magic value. Task 3 records a fresh check against SQLite's
official `magic.txt`; before any stable external file-format promise, the
project must either register that value or amend this RFC and migrate to a
different exact ID. A collision blocks the format claim but is not treated as
a cryptographic authentication mechanism.

```sql
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
```

Exactly one metadata row is required. Its key epoch must equal the authenticated
sidecar `db_key_epoch` and the fixed profile value `1` on insert and every open;
a future DBK epoch requires the separately journaled RFC migration already
required above. The typed transaction checks that the
ordered chunk set is contiguous, its count equals `chunk_count`, and its
overflow-safe byte sum equals `byte_count` before commit and again on read.
The fixed profile uses immediate `NO ACTION` foreign keys, not `RESTRICT` or a
cascading action: SQLite can enforce this schema while trigger depth is zero.
Foreign-key cascades and schema/user triggers are forbidden. The only typed
delete transaction first deletes every
chunk for one exact `(workspace_id, database_id, object_id)`, verifies the
affected-row count against the authenticated object row, then deletes that
object row in the same immediate transaction. Deleting metadata remains
forbidden. A direct parent delete, partial child delete, wrong identity, or
failpoint rolls back the whole mutation.
There is no generic metadata/value table. Any DDL, tag, disposition,
`application_id`, `user_version`, or registry change is a versioned migration
and RFC amendment; SQLite's mutable internal `schema_version` cookie is never
an application version.

Unknown tags are invalid. Strings, plugins, model output, manifests, and callers
cannot create a tag or change disposition. `VentureProfileV1` exists only for
the exact current demo `venture_profile` schema; it is not a generic JSON
container. A new eligible type requires an RFC, schema migration, and a new
registry version. Credentials, identity, authority, audit signing, sessions,
recovery internals, freshness anchors, and unfinished
effects have no table or generic blob escape hatch in this database.

Foreign keys and `CHECK` constraints enforce identity, fixed tags, per-row
indices, and per-row size bounds; the sealed transaction and read verifier
enforce the cross-row chunk order, count, and aggregate byte total. Logical names and
business content are columns inside SQLCipher, never file names or clear
sidecar fields. The schema has no views, triggers, virtual tables, dynamic SQL,
or caller-defined table/column names.

## Device unlock root

Core owns a sealed `DeviceKeyStore` interface. Product callers never receive a
DeviceKEK or choose a backend. The native adapter stores a workspace/device
unique random DeviceKEK, returns it only inside the core wrapper operation, and
zeroizes supported copies.

Observed capability classes are `HardwareBacked`, `OsProtected`,
`TestOnlyEphemeral`, and `Unavailable`. The first implementation is at most
`OsProtected`; hardware backing requires a separately reviewed native
seal/unwrap backend. Backend names and flags cannot upgrade capability.

An absent, locked, unsupported, or unreachable native service returns
`DeviceStoreUnavailable`. An individually absent enrolled credential returns
`DeviceKeyMissing`. Neither condition creates a file key, selects a sample
store, derives a machine key, or enters recovery automatically.

Production headless Linux requires an explicitly supported, unlocked Secret
Service path and otherwise fails closed. A later TPM2 adapter may be
`HardwareBacked` only after policy/measurement and real-hardware validation.

## Recovery unlock and owner authorization

Recovery enrollment and migration require a one-use authenticated
owner-presence authorization bound to workspace ID, operation, database and
wrapper commitments, format/suite versions, expiry, and a fresh challenge.
Loopback origin, the same OS account, possession of a password, or an unlocked
credential store are not owner presence. Until this authority boundary exists,
only crate-internal fixtures may initialize recovery records or import legacy
data; the product is readiness-only and non-mutating.

Recovery password unlock returns only `RecoverySession<ReadOnly>`. It opens
SQLCipher with read-only flags, sets `query_only=ON`, and uses an authorizer that
denies writes, schema changes, attach, pragmas outside a fixed read allowlist,
functions outside a fixed allowlist, and extension operations. Program 1A
exposes only internal typed `get`, `list`, and integrity verification; it has no
product presentation, arbitrary plaintext export, file write, or broker call.
Any owner-visible recovery presentation/export is deferred to a separately
authenticated and capability-bounded Program 1B1/1C0 broker. Possession of the
recovery password alone does not authorize plaintext export.

A separately consumed owner-presence authorization may transition it to
`RecoverySession<DeviceEnrollmentAuthorized>`. That state may only provision
and verify one native DeviceKEK, publish its typed DBK wrapper, then close.
Normal writes require a fresh device-root reopen. Error, expiry, or cancellation
returns to locked state. Recovery secrets never become an automatic online
root.

Password change rewraps the same RecoveryKEK under a new PWK and salt. Recovery
KEK rotation creates a new RecoveryKEK and rewraps the unchanged DBK. Old
records and exported backups remain usable to anyone retaining them; rotation
is not retroactive revocation.

## Wrapper sidecar and atomic state

`vault-v2/vault.slots` is a strict, bounded, canonical record containing only version,
workspace/database IDs, epoch, Argon profile/salt, opaque typed wrappers, native
credential record IDs, and commitments. It contains no DBK, DeviceKEK,
RecoveryKEK, business label, or content-derived identifier.

The live sidecar is UTF-8 RFC 8785 JCS with exactly this closed shape; every
binary value is canonical unpadded Base64url and every integer is a decimal
string:

```json
{
  "database_id": "...",
  "db_key_epoch": "1",
  "device": {
    "dbk_ciphertext": "...",
    "dbk_nonce": "...",
    "device_wrapper_id": "...",
    "protector_record_id": "...",
    "recovery_slot_commitment": "..."
  },
  "format_version": "2",
  "recovery": {
    "argon_profile_tag": "1",
    "argon_salt": "...",
    "dbk_ciphertext": "...",
    "dbk_nonce": "...",
    "kek_ciphertext": "...",
    "kek_nonce": "...",
    "recovery_kek_id": "...",
    "recovery_record_id": "..."
  },
  "suite_version": "1",
  "workspace_id": "..."
}
```

IDs and recovery commitment decode to exactly 32 bytes, nonces to 24,
DBK/KEK ciphertexts to exactly 48 bytes including the tag, and salt to 16. The parser rejects duplicate or
unknown fields, non-canonical JSON/Base64url/decimal, wrong lengths, wrong fixed
versions/epoch, trailing bytes, and records over 256 KiB before attempting a
KDF or unwrap. Every identity/profile value is repeated in the relevant typed
AAD, so authenticated unwrap fails on sidecar field substitution.

Sidecar updates are write-new, file-fsync, atomic-replace, directory-fsync, then
reopen-and-verify. A DBK remains unchanged during Program 1A wrapper updates, so
either the old or new complete sidecar opens the same committed database.
Rollback of a complete older valid sidecar remains subject to the rollback
boundary below.

Changing the recovery password or rotating RecoveryKEK changes the canonical
recovery subrecord. The transaction MUST recompute the recovery commitment and
rewrap the same DBK under an available DeviceKEK with the new
`DeviceDbkAad`, verify both routes, and publish the complete sidecar atomically.
If DeviceKEK is unavailable, the change is forbidden until a one-use owner
authorization enrolls and verifies a new device route. Recovery-only read
sessions cannot rotate recovery material.

The workspace-parent `vault.format` is an authenticated-state consistency
marker, not the product authority. The Program 1C0 owner/workspace registry is
the authoritative source of `ExpectedVaultBinding { workspace_id,
expected_format, activation_epoch, database_id }` and lives outside the
replaceable Vault/legacy generation. Before activation it expects legacy;
after Program 1D commits v2 it expects exactly v2 and its admitted database ID.
If the marker is missing, changed, or restored to legacy while the registry
expects v2, open fails terminally and never interprets the legacy tree. A
present v2 marker cannot activate a registry that still expects legacy.

Program 1A does not write `vault.format`. It creates a new private sibling
directory `vault-v2.staging-<opaque-random-id>/`, verifies its fixed `vault.db`
and `vault.slots`, and may durably rename that directory to non-authoritative
`vault-v2/`. It never places v2 files inside the exact legacy `vault/` root.
The still-authoritative workspace continues to use only `vault/`. A later
activation transaction uses a crash-recoverable Program 1D state machine. It
first acquires the exclusive legacy writer lock and freezes the last
authoritative generation. From that same retained read snapshot it creates a
new candidate (or completely rebuilds a stale earlier staging candidate) and
emits an unforgeable `VerifiedMigration { frozen_source_commitment,
candidate_content_commitment, workspace_id, database_id, schema_registry }`.
The proof derives both commitments inside the same reader-first migration
transaction; caller fields cannot pair an old candidate with a newer source.
Only that proof may create the externally authenticated `PendingV2` binding.
Program 1D then publishes and fsyncs the exact v2 marker, keeps the legacy
writer frozen, and verifies the admitted v2 generation and equivalence proof
again. It then runs Program 1B1 on that frozen candidate and
accepts only a `RecoveryQualification` bound to the workspace/database ID,
activation epoch, schema/registry, platform profile, legacy source-head
commitment, and backup commitment before advancing the external binding to
`ActiveV2`. Startup finishes or rolls back a pending transition without allowing either store to
write. Program 1B0 mechanics, Program 1C0 owner authority, Program 1C1
identity/role-key handoff, and all other activation gates precede PendingV2;
Program 1B1 executes inside the frozen transition. There is no custom database
manifest/head/CURRENT file and no dual authoritative writer.

Any legacy change observed between an earlier Program 1A staging build and the
freeze invalidates that staging candidate and forces a full rebuild from the
frozen snapshot. Crash/race/failpoint tests insert a business object after an
early staging build, freeze, and prove that stale database cannot enter
`PendingV2` or lose the new object.

## Real legacy migration

The current `vault/` root is unversioned. When workspace-parent `vault.format`
is absent, its exact internal layout is:

```text
vault.key
manifest.json                 # absent only for the exact empty-v1 case
<validated-entry-name>.enc    # exactly entries listed by manifest version 1
```

`vault.key` is padded standard Base64 for exactly 32 decoded bytes. The manifest
is exactly version 1 plus an entry-name array. Each entry is the current JSON
AES-256-GCM record with a 12-byte nonce. Because current `Vault::init` writes a
key before an empty manifest, `vault.key` alone with no `.enc` files is the one
valid empty-v1 state. Missing key, unexpected file, missing/duplicate/unlisted
entry, unknown field, symlink, traversal, oversize input, wrong nonce, or AEAD
failure rejects import without generating anything in the legacy root.

Migration is an internal reader-first transaction:

1. open the exact unversioned root read-only through a retained directory
   handle and verify every referenced entry's structure and AEAD consistency
   within fixed ceilings;
2. provision and verify both v2 wrappers only through an internal authorized
   fixture during Program 1A;
3. create a new private sibling `vault-v2.staging-<opaque-random-id>/` with
   fixed `vault.db` and `vault.slots`, new random IDs, DBK, DeviceKEK,
   RecoveryKEK, salt, and nonces;
4. in one SQL transaction, insert typed business rows and chunks, then commit;
5. close, reopen independently through device and recovery read-only paths,
   run SQLCipher and SQLite integrity checks, compare expected row counts and
   transaction-local content digests, fsync database/sidecar and directory,
   and optionally rename the verified staging directory to `vault-v2/` with a
   workspace-parent directory sync;
6. leave the untouched unversioned legacy generation authoritative on every
   Program 1A product path and every failure; and
7. only Program 1D may publish `vault.format` inside the externally
   authenticated pending/active activation protocol after Program 1B0
   mechanics, Programs 1C0/1C1, and workspace-inventory closure; Program 1B1
   then qualifies the frozen real candidate before `ActiveV2`.

No v2 authentication failure tries AES, another key, another epoch, or recovery.
Legacy cleanup is a separately authorized action after retention; it is logical
deletion, not guaranteed erasure.

The legacy key is stored beside the ciphertext. An attacker who copied that
entire generation could decrypt and replace entries and calculate valid new
tags. Migration therefore proves only structural validity and consistency with
the co-located key; it does not upgrade the generation into authenticated
historical provenance. If a separately retained signed commitment exists, the
importer compares it. Otherwise product activation requires an owner-visible
legacy inventory and records this limitation in migration evidence.

## Programs 1B0/1B1 filtered backup and restore

Backup never copies the live database file, journal, or sidecar. From one
read-only transaction, the builder creates a new SQLCipher recovery database
with a new random database ID and DBK, the fixed schema, and only rows whose
sealed core type is `BackupEligible`. It writes a recovery snapshot record that
binds workspace ID, source database ID/epoch, snapshot ID, registry/schema
versions, exact object/chunk counts, and a deterministic content commitment.

Creating that snapshot is an explicit owner-present recovery ceremony in this
profile. The device route can read the live DBK but cannot unwrap or derive the
independent RecoveryKEK. The builder therefore consumes the confirmed recovery
password locally to unwrap RecoveryKEK, wraps the new snapshot DBK under it,
and immediately zeroizes password/PWK/RecoveryKEK buffers. The age public
recipient alone is insufficient. Unattended scheduled backup would require a
separately reviewed online backup authority/key domain and is not implied by
Programs 1B0/1B1.

The core builder additionally consumes an opaque, one-use
`BackupAuthorization` bound to workspace/source database and epoch, registry
and schema versions, operation, destination age recipient, expiry, and fresh
challenge. Before Program 1C0, only crate-internal tests may issue this value;
that Program 1B0 evidence exercises mechanics but cannot qualify or expose a
product backup route. Program 1C0 supplies the real owner-presence issuer.
During Program 1D, the external registry first admits the exact frozen
candidate as `PendingV2`; Program 1B1 then performs a real clean-machine
restore of that candidate and returns the bound `RecoveryQualification`
required for `ActiveV2`.

The builder commits, runs both integrity checks, closes, reopens through the
recovery wrapper in `RecoverySession<ReadOnly>`, and verifies every identity,
count, and commitment. It then packages the filtered database, recovery-only
wrapper record, and bounded value-free index in a canonical archive, pads to a
public bucket, and encrypts the archive with unmodified age v1 to exactly one
dedicated offline X25519 recipient. Device wrappers and native-store IDs are
excluded.

Age recipient mode does not authenticate who created an archive. Restore
authenticity therefore derives from the RecoveryKEK-authenticated snapshot DBK,
the authenticated internal snapshot manifest, and, when continuity is claimed,
surviving signed authority/freshness evidence. A package with no surviving
continuity proof can be at most data rescue even if age decryption succeeds.

Unknown/unregistered/substituted tags fail snapshot construction. Authority,
identity, signing, session, freshness, credential, effect, device-KEK, and live
journal state are excluded because they cannot be represented by the backup
schema. Adding an eligible type requires RFC review.

Clean restore requires age identity plus recovery password, validates the whole
archive and recovery database before publication, provisions a new device root,
and starts new authority/membership/session epochs. Unless a surviving admitted
authority signs continuity, the UI says “verified data rescue under a new
identity,” not continuous authority or history.

## Whole-workspace activation blocker

Encrypting `vault-v2/vault.db` does not protect a workspace if the same business data
or secrets remain in plaintext elsewhere. Before any “Vault protected” state or
v2 product activation, a reviewed inventory MUST classify every persistent
writer and value class, including at minimum:

- `device.json` and identity material;
- `ledger.json` payloads (signatures provide integrity, not confidentiality);
- workflow records and checkpoints;
- execution journals and authority stores;
- outbox/email/effect payloads;
- admitted-artifact metadata and compiled caches;
- logs, reports, exports, crash output, and temporary files; and
- plugin/model/provider caches and future storage adapters.

Each confidential value must move into the closed business Vault or a separately
reviewed protected domain. Each intentionally public value needs a documented
classification and UI scope. Unknown writers or generic untyped blob paths
block activation. Program 1A may report component-level engine readiness only;
it MUST NOT report “workspace protected,” “recovery ready,” or silently run v1
and v2 as competing authoritative stores.

The current legacy Vault persists three separate role-key entries:
`owner_admission_key`, `owner_approval_key`, and `runtime_authority_key`.
Separately, `device.json` persists the device Ed25519 signing key in plaintext
Base64 within its private file. All four key paths are explicitly
`BlockedUntilIdentityHandoff`: they MUST NOT be inserted into SQLCipher as
business objects, copied by migration, included in backup, or left
unclassified. Program 1C1 must protect or replace them and define continuity or
reset before Program 1D activation.

The expected Vault binding is part of that separate activation state. A
complete, internally valid database and sidecar copied from workspace A must
not open as workspace B merely because both native credentials exist on the
same device. Deleting/restoring `vault.format` must not resurrect a legacy
generation after activation. Program 1A tests workspace/database substitution
with fixture-issued staging bindings; Program 1D persists and authenticates the
production format/activation/database binding outside the replaceable Vault
generation.

## CI and platform assurance

Generic unit tests inject a crate-private `TestOnlyDeviceStore` under `cfg(test)`
to deterministically exercise missing/locked/corrupt behavior. They never depend
on a runner's ambient credential service. The injected store and selector are
absent from release artifacts and public APIs.

Separate mandatory workflow jobs exercise the actual macOS Keychain, Windows
Credential Manager, and supported Linux Secret Service adapter, plus private
permissions, no-follow/reparse behavior, process races, rollback-journal crash
points, atomic sidecar replacement, and directory durability. A platform is
`Unavailable` if its native job is skipped, optional, injected, simulator-only,
or failing. A TPM simulator never proves hardware backing.

The cryptographic architecture is a closed exact-triple allowlist. Task 1
qualifies only `x86_64-unknown-linux-gnu`. Task 5 may add
`aarch64-apple-darwin` and `x86_64-pc-windows-msvc` only after the named real
native jobs prove `HOST == TARGET`, ABI/profile, custody, and durability.
Musl, GNU Windows, x86_64 macOS, aarch64 Linux, 32-bit, PowerPC, embedded, and
every other target remain `Unavailable` until an amendment names that exact
triple and adds architecture/toolchain evidence plus a mandatory native job.
RustCrypto's portable XChaCha implementation assumes constant-time integer
multiplication; that precondition is part of each review. Cross-compilation
success is never qualification evidence.

## Rollback and residual leakage

SQLCipher, wrapper AEAD, transactions, and signed chains detect corruption and
invalid internal state. They do not distinguish a complete older valid database
and sidecar from the latest state. Local external freshness detects rollback
only while that state survives. Full-device rollback requires an external
monotonic anchor such as suitable TPM state, another owner device, or a remote
transparency service; the choice is `Research` and deployment-specific.

The visible database and sidecar reveal approximate size, page count, file time,
wrapper count, and activity timing. Default encrypted headers hide schema bytes,
but do not hide that a database-like file exists. Logs use opaque IDs and coarse
sizes and never emit SQL parameters, content, passwords, keys, or wrappers with
adjacent secret material.

## Error semantics

Core returns stable, value-free typed errors. In particular:

- `DeviceStoreUnavailable` means the native service is absent, locked,
  unsupported, or unreachable;
- `DeviceKeyMissing` means the admitted service is usable but the exact enrolled
  record is absent;
- `AuthenticationFailed` covers wrapper/SQLCipher authentication without
  exposing which secret or page failed;
- `RecoveryFailed` coarsens wrong password and recovery-wrapper failure; and
- unsupported suite/profile/schema/limits, corrupt state, incomplete staging,
  and policy-required freshness failure are distinct non-secret errors.

No error enters recovery, creates a key, opens legacy, lowers a profile, or
returns partial plaintext. Recovery UI backoff is not described as offline
guessing protection.

## Forbidden implementations

Implementations MUST NOT:

- implement or modify Argon2id, XChaCha20-Poly1305, SQLCipher, SQLite pager, or
  age formats in repository code;
- use plaintext SQLite, WAL in profile v1, plaintext headers, system SQLCipher,
  dynamic extension loading, `ATTACH`, writable schema, or caller SQL;
- pass DBK through SQL text, argv, environment, logs, Rust `String`, or config;
- reuse a key/nonce/purpose, truncate tags, or accept another profile after
  authentication failure;
- create file/sample/machine-derived keys or automatically use recovery;
- copy the live database as a backup or mark caller/plugin-chosen data eligible;
- describe encoding, signing, local transport, TLS, SQLCipher alone, or age
  alone as E2EE, hardware backing, secure erasure, rollback protection, or
  whole-workspace confidentiality.

## Required tests and release gates

### Engine tests

- SQLCipher `4.14.0` compile/runtime version for the released initial profile,
  completed advisory/delta review, upstream compatibility fixtures, and a
  fixed-hash/known-row raw-key fixture independently generated and verified
  with the official SQLCipher CLI syntax,
  exact pragma readback, encrypted-header proof, wrong-key failure, and no
  plaintext canary in database/journal/sidecar.
- A mandatory uncached wrapper run on exact native
  `x86_64-unknown-linux-gnu`, with the full ambient-variable negative matrix,
  frozen/offline fresh target, recorded tool hashes, and no bare-Cargo engine
  qualification path in CI.
- Real-binary subprocess tests proving process-first NO_LOAD_CONFIG under
  hostile configuration, rejection after a real LOAD_CONFIG-first adversary,
  private `CryptoProcessOwner` mediation, and an AST/call graph containing only
  the two allowed unsafe FFI entry points.
- RFC 9106 and upstream XChaCha vectors plus repository golden typed-AAD vectors.
- Recovery-slot deletion, bit corruption, cross-workspace substitution, and
  valid-slot replacement all make device unwrap fail through the bound
  commitment; password/RecoveryKEK rotation atomically verifies new recovery
  and device wrappers, while a complete older valid sidecar remains a
  documented rollback case.
- Compile/static assertions for secret opacity, algorithm/nonce non-selection,
  no extension/attach/dynamic SQL, no engine API in the protocol-only library,
  and no release test store.
- Bound, malformed, corruption, truncation, schema substitution, wrong
  workspace/database/epoch/role, cross-wrapper, and unknown object-tag tests.
- Transaction/fault tests at begin, row/chunk insert, commit, journal sync,
  sidecar sync/replace, directory sync, close, reopen, and integrity checks.
- Recovery typestate tests proving read-only sessions cannot compile or execute
  mutation and authorized enrollment cannot perform business writes.
- Real unversioned-root importer tests, including the exact empty-v1 state and
  missing-key no-regeneration regression.

### Activation and recovery gates

- Mandatory real native-store/durability jobs for every enabled platform.
- Program 1B0 filtered-snapshot, age-interoperability, loss-matrix, and
  clean-machine-harness tests over staging fixtures, followed by Program 1B1
  clean restore and new-device enrollment of the exact frozen real candidate;
  only 1B1 may qualify recovery and continuity labels.
- Whole-workspace writer/value inventory with attack tests proving confidential
  canaries do not remain in plaintext persistence, logs, exports, or caches.
- Dependency/supply-chain review, fuzzing of wrapper/archive/legacy parsers,
  independent cryptographic/integration review, and recovery drills.

No format path advances to `Current`, and no platform activates v2, until its
applicable gates pass. Local absence of a Rust toolchain is recorded as an
unavailable local gate and replaced by inspected CI evidence, never called a
local pass.

## Rollout

1. **Engine and fixtures:** SQLCipher connection factory, typed wrappers, closed
   schema, parser limits, test-only store, read-only recovery typestate.
2. **Native adapters and platform CI:** real credential services and mandatory
   durability/security jobs; unsupported platforms remain unavailable.
3. **Internal legacy readiness:** exact v1 importer constructs and verifies a
   side-by-side v2 staging database. Product remains on one legacy authority.
4. **Program 1B0 mechanics:** filtered recovery database, age envelope,
   clean-machine harness, loss matrix, new epochs, and honest continuity labels
   over fixture staging bindings. This is not product qualification.
5. **Workspace secrecy closure:** inventory and close every confidential
   persistence path; unknown writers block progress.
6. **Programs 1C0/1C1:** 1C0 admits the one owner session and one-use approval
   issuer used by backup, Exact Effect, and activation. 1C1 protects or replaces
   role/device keys, establishes continuity or explicit reset, and keeps the
   product selector unchanged.
7. **Program 1D plus Program 1B1:** freeze the exact real candidate in
   `PendingV2`, run its owner-authorized backup and clean-machine restore, and
   atomically select it only after a matching `RecoveryQualification` permits
   `ActiveV2`; the existing legacy workspace is otherwise read-only.
8. **Default v2 and legacy retirement:** only on qualified platforms, followed
   later by separately authorized logical cleanup.

Stages are reversible except owner-confirmed cleanup. Product and marketing
labels advance only with evidence for their exact boundary.

## Primary references

- [SQLCipher design](https://www.zetetic.net/sqlcipher/design/)
- [SQLCipher API](https://www.zetetic.net/sqlcipher/sqlcipher-api/)
- [SQLCipher raw key documentation](https://www.zetetic.net/sqlcipher/sqlcipher-api/#key)
- [SQLite atomic commit](https://www.sqlite.org/atomiccommit.html)
- [SQLite defensive configuration](https://www.sqlite.org/security.html)
- [rusqlite documentation](https://docs.rs/rusqlite/0.40.2/rusqlite/)
- [RustCrypto chacha20poly1305 0.11.0 security notes](https://docs.rs/crate/chacha20poly1305/0.11.0)
- [`keyring` 4.1.5 v1 platform-store documentation](https://docs.rs/keyring/4.1.5/keyring/v1/)
- [RFC 9106: Argon2](https://www.rfc-editor.org/rfc/rfc9106.html)
- [RFC 8785: JSON Canonicalization Scheme](https://www.rfc-editor.org/rfc/rfc8785.html)
- [age v1 specification](https://age-encryption.org/v1)

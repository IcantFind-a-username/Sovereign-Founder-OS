# Iteration backlog

Cross-session task queue for automated iteration. Seeded 2026-08-14 from a
repo audit; every entry below points at verified, real state of the code.

## Rules (read before touching the queue)

- **One item per round.** An `/iterate` session claims exactly one item and
  ends after it lands. Never batch.
- **Claim before work.** Mark the item `IN PROGRESS (<date>)` and commit that
  edit first, so concurrent agents don't collide on the same item.
- **Done = checked off.** Check the box in the same commit that completes the
  work, or the next session will re-claim it.
- **New problems join the queue.** Anything discovered mid-round gets a new
  prioritized entry here — it does NOT get fixed in passing.
- **Stuck = diagnose and release.** If a round can't go green, write the
  diagnosis as an indented note under the item, remove the IN PROGRESS mark,
  and end the session. The note is the next session's starting point.
- **Entry format:** priority (P1 urgent / P2 soon / P3 opportunistic) +
  directory-level scope + done criteria that a test (or an exact command)
  can verify.
- **Model escalation is a tag, not a judgment call.** Entries needing
  design-heavy work carry a `needs:fable` tag (added by `/plan-feature` or a
  human). Unattended small-model sessions MUST skip tagged entries — no
  heroics. If an item accumulates 2 failed-round diagnoses, the session that
  fails it the second time tags it `needs:fable` and releases it instead of
  retrying. Tagged items are consumed by a human-driven session on a larger
  model (`/model`), which removes the tag when the item lands.
- **Big-model sessions should re-slice before they implement.** A session
  consuming a `needs:fable` item has two valid outcomes: land it, or split it
  into untagged single-round items (its diagnoses tell you where the real
  boundary is) and let nightly small-model rounds do the typing. Re-slicing
  is usually the cheaper outcome — spend big-model tokens on decomposition
  and judgment, not on keystrokes a small model can gate-check.

## Queue

- [x] **P1 | `crates/consultant-playground/tests/` | Split `physical_boundary.rs` before it breaks the file-size gate.**
  The file is at 1192 lines against the hard 1200 limit in
  `scripts/check-file-size.sh` (allowlist is deliberately empty) — the next
  test added there fails CI for everyone. Split into per-boundary test files.
  Done when: `./scripts/check-file-size.sh` passes with every resulting file
  ≤ ~800 lines, and `cargo test -p sovereign-consultant-playground` passes
  with the same test count as before the split.

- [x] **P1 | `crates/contracts/` | Add serialization-shape tests for the signed contract types.**
  The crate has zero tests, yet `CapabilityTokenBody` is the canonical signed
  body whose serde shape is load-bearing for signature verification in
  `crates/capability` — today a field rename would silently invalidate
  tokens. Done when: a serde round-trip test plus a golden-JSON test pin the
  field names of `CapabilityTokenBody` and `PolicyDecision`, and renaming any
  field makes `cargo test -p sovereign-contracts` fail.

- [x] **P1 | `scripts/` | The quality gate exits 0 without running a single check on bash 3.2.**
  `test_changed.sh` (line 78) and `check-file-size.sh` (line 12) both use
  `declare -A`, a bash 4 feature. On stock macOS (`/usr/bin/env bash` =
  bash 3.2.57) `test_changed.sh` prints `declare: -A: invalid option`, aborts
  before any check runs, and still **exits 0** — a false green that the Stop
  hook would accept as a passing session. Found 2026-08-15 during the contracts
  round, which had to run the gate's four steps by hand instead. Done when: the
  gate runs its checks on bash 3.2 (drop the associative arrays — both
  allowlists are empty and a plain list works) or refuses to run with a nonzero
  exit and an explicit "unsupported bash" message; plus a test or an exact
  command demonstrating that a gate that cannot run never exits 0.

- [x] **P1 | `crates/sandbox/` | `cargo test --workspace` is red on macOS: the compile worker's `RLIMIT_AS` blocks exec.**
  `compile_worker::tests::parent_fails_closed_on_timeout_nonzero_and_garbage_output`
  fails deterministically (3/3 runs) on macOS arm64 at src/compile_worker.rs:301:
  the `/bin/sleep 30` stand-in is expected to hit the 150 ms deadline and yield
  `CompileWorkerTimeout`, but some other error arrives first. Hypothesis to check
  first: `apply_rlimit` sets `RLIMIT_AS` to 1 GiB (src ~56) in `pre_exec`, and on
  macOS dyld reserves far more address space at exec than on Linux, so exec fails
  and `spawn` returns `CompileWorkerFailed("spawn: …")` before the deadline.
  **If that is the cause it is not only a test bug** — the real out-of-process
  compile worker would also fail to spawn on macOS, so check whether the
  Security Center gauntlet's worker path actually works there before touching
  the test. Found 2026-08-15 during the contracts round (pre-existing at
  `feature/auto-iterate` HEAD, unrelated to that change; `crates/sandbox` was
  untouched). Done when: the root cause is identified, the worker spawns under
  its address-space cap on both macOS and Linux (or the cap is applied by a
  portable mechanism), and `cargo test --workspace --locked` is green on macOS.

- [x] **P1 | `apps/cli/src/` | The Security Center's compile-isolation check passes when the worker never started.**
  `run_gauntlet`'s `compile_isolation` check (ui.rs:794-822) accepts any
  `Err(CompileWorkerFailed(_) | CompileWorkerTimeout)` as proof that hostile
  compilation was contained in a child process. That same variant is what a
  *failed spawn* produces, so while `setrlimit` was aborting every exec on
  macOS (fixed 2026-08-15 in `crates/sandbox`) this check reported green with
  the detail "compiled in a killable, memory-limited worker" even though no
  worker process ever ran and nothing was compiled anywhere. The check now
  passes for the right reason on macOS, but its shape still cannot tell the
  two apart, and the wording is unconditional where the cap is not:
  `CompileWorker::address_space_enforcement()` returns `Unavailable` on Darwin,
  yet ui.rs:603, 665 and 821 all claim a memory-limited worker regardless.
  Found 2026-08-15 during the `crates/sandbox` round; out of that round's
  scope. Done when: the check distinguishes "the worker ran and the failure was
  contained" from "the worker never started" (a spawn failure must report
  `pass: false`), the detail string reports the platform's real enforcement
  from `address_space_enforcement()` instead of asserting a cap, and
  `cargo test -p sovereign-cli` covers both outcomes.

- [x] **P1 | `crates/vault/` | Fail closed when `vault.key` is missing but entries exist.**
  `Vault::init` (src/lib.rs:53-59) silently generates a fresh key whenever
  `vault.key` is absent, so a lost or deleted key file turns every existing
  `*.enc` entry into permanently undecryptable data while the vault still opens
  and reports success. RFC 0004
  (`rfcs/0004-data-sovereignty-boundaries.md`:537-539) names "silently
  regenerate a missing key" as an explicitly rejected alternative, so this is
  live code contradicting an accepted design. It is also the real construction
  site for the dead `VaultError::NotInitialized` (src/lib.rs:16) that the P2
  vault entry below asks about — the two overlap, so do not claim both in the
  same round. Done when: `Vault::init` on a root holding at least one `*.enc`
  file (or a non-empty `manifest.json`) with no `vault.key` returns
  `VaultError::NotInitialized`, a first-run empty root still initializes
  normally, and `cargo test -p sovereign-vault` covers both paths.

- [x] **P1 | `crates/vault-v2-engine/` | Stand up the engine crate skeleton with pinned dependencies and no cryptography.**
  First code step of RFC 0005 Program 1A, which every encrypted-backup,
  recovery, and multi-device claim is blocked on. Standing finding for the whole
  encryption program, so no session re-derives it: the design is already settled
  in `rfcs/0004-data-sovereignty-boundaries.md` and
  `rfcs/0005-dual-root-vault-and-recovery.md`, with task-level detail in
  `docs/superpowers/plans/2026-08-13-dual-root-vault-v2-implementation.md` —
  **no new RFC is needed.** Dependency order is fixed by rfcs/0004:491-524 as
  1A → 1C0 → (1B0 ∥ 1C1) → 1D, and encrypted backup cannot be the starting
  point: 1B0 needs this engine plus a SQLCipher version amendment (that plan,
  lines 1445-1451). No network transport exists in the workspace yet
  (`crates/effects/src/lib.rs`:26-30), so "E2EE" here means at-rest and backup
  confidentiality, not transit. This entry deliberately excludes cryptography:
  add the workspace member with `publish = false`, target auto-discovery
  disabled and every target explicit, a `build.rs` that rejects
  dependency-shaping ambient overrides, and a value-free `src/lib.rs` carrying
  protocol/version constants only — no engine API, no connection type, no raw
  handle (that plan, lines 436-444 and 465-469). Done when:
  `cargo build -p sovereign-vault-v2-engine --locked` succeeds, a test asserts
  the build-script check rejects a hostile ambient override,
  `cargo test -p sovereign-vault-v2-engine` passes, and
  `cargo clippy --workspace --all-targets --locked -- -D warnings` stays green
  (the full workspace suite stays red on macOS until the `crates/sandbox` P1
  above lands, so it cannot gate this entry).

- [x] **P2 | `crates/vault/` | Add tamper-detection tests.**
  Unlike sibling `audit-ledger` (which has `tamper_detection`), no vault test
  flips a ciphertext byte. The other two halves of this entry closed on
  2026-08-15 with the missing-key P1 above: `VaultError::NotInitialized` is now
  constructed on a real path with three tests, and
  `a_vault_reopened_with_its_key_intact_still_reads_its_entries` covers the
  reopen path. Done when: a test asserts `get()` returns `DecryptionFailed`
  after mutating one byte of a stored `*.enc` blob, and one asserts the same
  for a truncated blob.

- [x] **P2 | `crates/policy/` | Cover the V2 rejection paths in-crate.**
  All three existing tests target the legacy v1 `evaluate()`; the V2 surface
  (`AuthenticatedPolicyContextV2::new`, `evaluate_prepared`) has no in-crate
  coverage of its rejection paths (nil `session_id`/`idempotency_key`,
  malformed `audience`/`venture_id`/`subject_id`, `MissingPrimaryResource`).
  Done when: table-driven tests assert each `PolicyV2Error` variant from
  `cargo test -p sovereign-policy`.

- [ ] **P2 | `crates/identity/` | Add a public-API integration test boundary.**
  All 12 tests live in `src/tests.rs` and reach private internals; nothing
  validates the crate through `sovereign_identity::…` re-exports. Done when:
  `crates/identity/tests/public_api.rs` exercises key lifecycle through the
  public API only and passes.

- [ ] **P2 | `crates/vault/` | Create the vault key, manifest, and entry files with owner-only permissions.**
  `write_atomic` (src/lib.rs:118-137) uses `std::fs::File::create` with no mode
  and `init` (src/lib.rs:51) uses `create_dir_all`, so under a default umask the
  master key `vault.key` (src/lib.rs:52-59, 160-163) lands at 0644 inside a 0755
  directory — weaker than the device signing key, which
  `crates/identity/src/fs.rs`:189-223 creates 0600 with `O_EXCL|O_NOFOLLOW` and
  re-verifies after rename (fs.rs:306-328). Defense in depth only, hence P2: the
  key still sits beside its ciphertext by design at this stage. The on-disk
  format must not change — only file modes. Multiple `crates/vault/` entries
  may be queued at once; claim at most one of them per round. Done when: a
  `#[cfg(unix)]` test asserts `vault.key`, `manifest.json`, and `<name>.enc` are
  mode 0600 and the vault root is 0700 after `Vault::init` followed by `put`,
  and `cargo test -p sovereign-vault` passes.

- [ ] **P2 | `apps/cli/src/workspace/` | Pin the export's plaintext-and-unauthenticated boundary with a test.**
  `Store::export` (reporting.rs:289-313) writes the whole business graph as
  cleartext JSON, and `verify_export` (verify.rs:12-120) re-verifies only the
  Ed25519 audit chain and the device binding — the `workspace` object itself is
  counted, never authenticated (verify.rs:74-98). Nothing today stops a later
  change from being described as an encrypted or verified backup. Done when: a
  test asserts a known customer name appears verbatim in the exported bytes and
  that mutating a field inside `workspace` still leaves
  `ExportVerification.ok == true`, named so its failure reads as "the export
  boundary changed", and `cargo test -p sovereign-cli` passes.
  Runtime evidence 2026-08-15 (both halves reproduce end to end on the running
  binary, so the test below is pinning observed behavior, not a hypothesis):
  `GET /api/export` after one approved send emitted the customer name
  `Dr. Tan` verbatim three times in the 10883-byte bundle; editing that name to
  `ATTACKER RENAMED` and the invoice amount to `999999999` inside `workspace`
  and re-running `sovereign verify-export` still printed "VERIFIED — this
  bundle is intact and bound to the device that signed it" and exited 0.

- [ ] **P2 | `apps/cli/src/` | The Security Center reports the owner's real admitted plugin as unverified.**
  `admitted_plugins_json` (ui.rs:511-562) verifies every record under
  `artifacts/admissions/` against `demo_admission_trust()` (ui.rs:564-576),
  which trusts exactly one key: the hard-coded `demo::DEMO_ADMISSION_SECRET`
  under issuer `founder-device.local` (demo.rs:41, 46). The Workspace admits
  its plugin with the vault-held owner key under a different issuer,
  `OWNER_ADMISSION_ISSUER = "founder-device.workspace"` (workspace/mod.rs:69,
  workspace/kernel_exec.rs:125-136), so the one plugin the owner actually runs
  can never verify against the anchor the security surface uses. Observed
  2026-08-15 on a clean root: after a single approved send, `GET /api/state`
  returned its only admission record as `"verified": false` with "admission
  record failed verification against the demo trust anchor", while the two
  records a `demo --fast` run leaves behind both returned `"verified": true`
  with issuer `founder-device.local` — the surface flags the real plugin and
  clears the demo ones. P2 rather than P1 because no enforcement is weakened:
  the Workspace's own load path re-verifies against the owner anchor correctly
  (workspace/kernel_exec.rs:138-146) and refuses a record that fails, so this
  is a trust-reporting defect — but a permanent red badge on a correctly
  admitted plugin teaches the owner to ignore red. `apps/cli/src/ui.rs` is
  already queued for a split (P3 below) and round 3 put the compile-isolation
  verdict and its wording in `apps/cli/src/gauntlet_report.rs` rather than
  growing `ui.rs`; follow that precedent and give this logic its own module.
  Done when: a record admitted by the Workspace reports `"verified": true` with
  its real issuer, a record signed by neither anchor is still listed as
  unverified rather than hidden, and `cargo test -p sovereign-cli` covers both
  outcomes.

- [ ] **P2 | `apps/cli/src/` | `demo` writes its events and admission records into the owner's real data root.**
  `Commands::Demo` hands `data_dir()` straight to the demo (main.rs:88), so the
  story-driven run initializes and writes the same root the Workspace uses.
  Observed 2026-08-15 against a clean root: one `demo --fast` created
  `device.json`, a `ledger.json` carrying a demo event, `vault/vault.key` with
  `vault/venture_profile.enc` and its manifest, and two admission records plus
  their objects and manifests under `artifacts/` (demo.rs:157, 170, 326,
  537-538). On an owner's machine those land in the real vault and the real
  audit ledger, which is the tamper-evident record of what actually happened —
  seeding it with demo events degrades that evidence rather than just leaving
  clutter — and the demo's admission records then appear in the Security
  Center's plugin list. Done when: `demo` runs against an isolated or clearly
  marked ephemeral root by default, and a test that seeds a populated default
  root, runs the demo, and re-reads it asserts that root's `ledger.json`,
  `vault/`, and `artifacts/admissions/` are unchanged. May reuse the `--root`
  plumbing from the P3 below, but must not depend on the owner passing it.

- [x] **P2 | `crates/vault/` | Decide and record whether v1 entry blobs stay unbound to their entry name.**
  `encrypt` (src/lib.rs:173-185) passes no associated data, so a `*.enc` blob
  carries nothing binding it to its entry name, vault root, or format version:
  anyone able to write the vault directory can substitute
  `owner_approval_key.enc` with another entry's blob, or an older copy of the
  same one, and it decrypts cleanly (those entries are created at
  `apps/cli/src/workspace/kernel_exec.rs`:125,160,211,258). Adding AAD would
  change the exact on-disk format that RFC 0005 Program 1A freezes and whose
  importer must read byte-exactly (Program 1A plan
  `docs/superpowers/plans/2026-08-13-dual-root-vault-v2-implementation.md`,
  lines 1125-1129), so the likely answer is "freeze v1 and fix it in v2" — but
  that must be a recorded decision, not an oversight. Done when: either a test
  asserts a cross-entry ciphertext swap is rejected, or `THREAT_MODEL.md`
  states the v1 swap/rollback gap explicitly and a test pins the current blob
  format so a silent format change fails.
  Decided 2026-08-26: **freeze v1 as-is; the fix is structural and belongs to
  v2.** Adding AAD would (a) break decryption of every existing vault or force
  a migration of the exact on-disk format Program 1A's legacy importer must
  read byte-exactly, (b) still not detect same-entry rollback — an older copy
  of the same entry carries the same associated data — and (c) defend against
  a directory writer who, in v1's co-located-key model, can already read
  `vault.key` sitting beside the ciphertext, so the marginal gain does not buy
  the migration risk. The recording and format-pinning work is re-sliced into
  the two untagged entries directly below; the decision itself is settled and
  must not be re-opened by the rounds that type it in.

- [ ] **P2 | `THREAT_MODEL.md` | State the v1 entry swap/rollback gap and its freeze decision under T10.**
  Typing for the decided item above — the wording is settled; do not re-open
  the decision. Insert the following bullet into T10's Mitigations list,
  directly after the existing "Current limitation" bullet (re-wrap lines to
  match the file's style; keep the text otherwise verbatim):
  "**Current limitation (recorded decision, 2026-08-26):** v1 entry blobs are
  encrypted with no associated data, so nothing binds a `*.enc` blob to its
  entry name, vault root, or format version: an attacker who can write the
  vault directory can substitute one entry's blob for another's, or roll a
  single entry back to an older copy of itself, and it decrypts cleanly. The
  v1 format is deliberately frozen rather than amended: adding AAD would break
  every existing vault or force a migration of the exact on-disk format RFC
  0005 Program 1A's legacy importer must read byte-exactly, would still not
  detect same-entry rollback, and defends against a directory writer who can
  already read the co-located `vault.key`. Per-entry swap and rollback are
  accepted residual risks of the legacy format until v2's transactional
  SQLCipher format and context-bound wrappers (targets above) replace it."
  Done when: the bullet is present under T10, no file outside `THREAT_MODEL.md`
  changes, and `./scripts/test_changed.sh` prints ALL GREEN.

- [ ] **P2 | `crates/vault/` | Pin the frozen v1 blob format and its accepted swap/rollback behavior with tests.**
  Typing for the decided item above; land the `THREAT_MODEL.md` entry first so
  doc comments can cite it. Four tests, no production code changes:
  1. `v1_blob_shape_is_frozen`: after `put`, parse the written `*.enc` as
     `serde_json::Value`; assert the object has exactly the keys `nonce_b64`
     and `ciphertext_b64`, that `nonce_b64` decodes to 12 bytes, and that
     `ciphertext_b64` decodes to plaintext length + 16 (the GCM tag).
  2. `v1_golden_blob_still_decrypts`: write a fixed base64 32-byte key to
     `vault.key`, write a golden `entry.enc` JSON literal, `Vault::init`, and
     assert `get("entry")` returns the known plaintext — this pins the cipher
     (AES-256-GCM), key-file encoding, and blob layout, so swapping any of
     them fails loudly. Generate the golden once: temporarily add an
     `#[ignore]`d test that writes the fixed key, calls
     `put("entry", b"golden plaintext v1")`, and prints the `entry.enc`
     contents; run it with `--ignored --nocapture`, paste both literals into
     the real test, delete the generator before committing.
  3. `a_cross_entry_ciphertext_swap_decrypts_cleanly_v1_freeze`: put two
     entries, copy `a.enc` over `b.enc`, assert `get("b")` returns a's
     plaintext.
  4. `an_older_copy_of_the_same_entry_decrypts_cleanly_v1_freeze`: put, copy
     the blob aside, put new content, restore the old blob, assert `get`
     returns the old plaintext.
  Tests 3-4 carry a doc comment citing THREAT_MODEL.md T10's recorded
  decision and saying plainly: if this test starts failing, the v1 freeze was
  broken, which requires a recorded decision, not a silent format change.
  Done when: all four pass in `cargo test -p sovereign-vault`, production code
  is untouched, and `./scripts/test_changed.sh` prints ALL GREEN.

- [x] **P2 | `rfcs/` | Amend RFC 0005 to name the exact SQLCipher release that unblocks Program 1B0.**
  Program 1B0 — the filtered encrypted backup, the first work that could ever
  support an "encrypted backup" claim — cannot start on the pinned SQLCipher
  4.14.0 profile because of its fixed `sqlcipher_export` defensive-mode bypass;
  the plan requires an RFC amendment naming exactly 4.17.0 or a later release
  (Program 1A plan
  `docs/superpowers/plans/2026-08-13-dual-root-vault-v2-implementation.md`,
  lines 1445-1451). Done when: `rfcs/0005-dual-root-vault-and-recovery.md`
  carries an amendment section naming one exact release plus its verification
  method, keeps `sqlcipher_export`, `ATTACH`, and database-copy APIs forbidden,
  and records the status change. No code lands in this round.

- [x] **P2 | `rfcs/` | Move RFC 0005 from Draft to a decided status with its required review evidence.**
  RFC 0005 is `Status: Draft; approved implementation target` with
  `Security impact: Critical` (`rfcs/0005-dual-root-vault-and-recovery.md`:3-6),
  while ROADMAP.md:508-518 requires security-sensitive RFCs to carry a
  threat-model delta, an adversarial test plan, migration/rollback analysis, and
  independent review when a release gate calls for it. Program 1A code is about
  to be written against it. Done when: RFC 0005 either reaches `Accepted` with
  those four sections present and linked, or states explicitly which gate is
  outstanding and what must not be built until it closes.

- [ ] **P2 | `crates/vault-v2-engine/` | Add the single unsafe FFI module that opens SQLCipher and keys it first.**
  RFC 0005 Program 1A Task 1's core, and the first entry in this crate that
  touches cryptography. `src/engine/ffi.rs` owns `sqlite3_open_v2`, calls
  `sqlite3_key` as the first post-open database operation, then disables both
  extension routes via `sqlite3_enable_load_extension(handle, 0)` and
  `sqlite3_db_config(handle, SQLITE_DBCONFIG_ENABLE_LOAD_EXTENSION, 0, &out)`
  (Program 1A plan
  `docs/superpowers/plans/2026-08-13-dual-root-vault-v2-implementation.md`,
  lines 273-283). The raw key lives in a non-clone, non-formatting, zeroizing
  holder in `src/engine/secret.rs` (same plan, lines 288-291) and must never
  reach SQL text, a `String`, argv, the environment, or logs. Cipher profile
  and resource limits belong to the next entry, not this one. Done when: tests
  `wrong_dbk_fails_without_schema_or_plaintext`,
  `raw_dbk_never_reaches_sql_text_logs_or_environment`, and
  `database_header_and_journal_do_not_contain_plaintext_canary` pass from
  `cargo test -p sovereign-vault-v2-engine`, and no `unsafe` block exists
  outside `src/engine/ffi.rs` and `src/engine/process.rs`.

- [ ] **P2 | `crates/vault-v2-engine/` | Pin and verify the exact SQLCipher connection profile and resource limits.**
  Apply the fixed profile — compatibility 4, 4096-byte pages, AES-256-CBC,
  HMAC-SHA512, encrypted header, `cipher_memory_security=ON`, memory-only temp,
  rollback journal, `synchronous=FULL`, foreign keys, trusted schema off,
  defensive mode — plus every fixed `sqlite3_limit` before first page access
  (Program 1A plan
  `docs/superpowers/plans/2026-08-13-dual-root-vault-v2-implementation.md`,
  lines 269-272 and 353-378). Read every setting back through the real engine
  rather than trusting an observed profile transcript. Done when: tests
  `sqlcipher_runtime_is_exactly_4_14_0_for_released_profile`,
  `connection_profile_matches_every_required_pragma`,
  `extensions_attach_writable_schema_and_dynamic_sql_are_denied`,
  `oversized_values_and_sql_fail_at_fixed_limits`, and
  `page_2_ciphertext_bitflip_is_detected_cryptographically` pass, each limit has
  boundary and boundary-plus-one coverage, and `./scripts/check-file-size.sh`
  stays green.

- [ ] **P2 | `crates/vault-v2-engine/` | Add the syn AST gate proving the FFI boundary is exact.** `needs:fable` IN PROGRESS (2026-08-26)
  `recursive_syn_source_closure_is_complete_and_ffi_boundary_is_exact` starts
  from `build.rs`, every explicit Cargo target, and `tests/ui.rs`, then parses
  the complete recursive closure of inline and external modules to prove the
  plaintext-header setter, arbitrary SQL, a raw-handle escape, backup/export/
  copy calls, and extra project-authored OpenSSL/SQLite unsafe calls are absent
  (Program 1A plan
  `docs/superpowers/plans/2026-08-13-dual-root-vault-v2-implementation.md`,
  lines 503-512). Ship it with exactly the five named trybuild fixtures from
  that same plan, lines 449-458. Design-heavy and easy to get subtly wrong.
  Done when: the AST test and all five compile-fail fixtures pass from
  `cargo test -p sovereign-vault-v2-engine`, and the test fails if a raw-handle
  accessor is added anywhere in the closure.

- [ ] **P2 | `.github/workflows/`, `scripts/`, `crates/vault-v2-engine/` | Confirm the vault-v2 build gate does not trip on a clean CI runner.**
  `crates/vault-v2-engine/build.rs` (added 2026-08-15) panics when any of 23
  dependency-shaping variables or the `PKG_CONFIG_*` family is set to a
  non-empty value. Because the crate is a workspace member, a runner that
  happens to export one — `PKG_CONFIG_PATH` from a system-library setup step,
  or `RUSTFLAGS` from a cache action — would fail `cargo build --workspace` for
  the whole repository, not just this crate. Verified locally on macOS
  (workspace build, clippy and tests all green with the gate active) but not
  yet on `ubuntu-latest`. Low likelihood, wide blast radius, hence P2 rather
  than P3. Done when: a CI run on `feature/auto-iterate` builds the workspace
  green with the gate active, or the observed offender is recorded in the
  build script's comment and handled by the qualification wrapper below rather
  than by shortening the allowlist.

- [ ] **P2 | `.github/workflows/`, `scripts/` | Add the vault-v2 qualification entry point and its evidence ledger.**
  RFC 0005 Program 1A Task 1's tail: `scripts/qualify-vault-v2.sh` as the sole
  sanitized Cargo qualification entry point, the mandatory native-store and
  durability job, and `docs/security/vault-v2-verification.md` as an honest
  readiness ledger that claims no product protection (Program 1A plan
  `docs/superpowers/plans/2026-08-13-dual-root-vault-v2-implementation.md`,
  lines 424-426 and 1325-1424). The new script must run on bash 3.2 — see the
  `scripts/` P1 above; do not repeat the `declare -A` mistake. Done when:
  `./scripts/qualify-vault-v2.sh` runs green locally or its unavailability is
  recorded with inspected CI evidence, CI invokes it, and the evidence document
  contains no "encrypted at rest", "E2EE", "recovery-complete", or
  "production-ready" claim.

- [ ] **P2 | `.github/workflows/` | Run the gate self-test in CI.**
  `scripts/tests/gate_portability_test.sh` (added 2026-08-15) proves the gate
  scripts run on bash 3.2 and can never exit 0 without checking anything, but
  nothing in CI invokes it: `ci.yml` calls `check-file-size.sh` directly and
  never calls `test_changed.sh`, so today the self-test only runs on a
  developer machine via the gate's own `gate-self-test` step. CI runners are
  bash 5, so the execution half of the portability claim is only ever proven
  locally — the static construct scan is what carries it there. Out of scope
  for the round that added the self-test (`scripts/` only). Done when: the
  `test` job in `.github/workflows/ci.yml` runs
  `./scripts/tests/gate_portability_test.sh` before the file-size step, and a
  run shows it green.

- [ ] **P3 | `.github/workflows/` | Repin the four actions still targeting the Node 20 runtime.**
  Every CI job carries a deprecation annotation. Run 31886107586 (green, commit
  9277f27 on `feature/auto-iterate`) reports "Node.js 20 is deprecated … being
  forced to run on Node.js 24" against `actions/checkout@34e1148` (all four
  jobs — dependency-review, test, frontend-types, security),
  `actions/dependency-review-action@2031cfc`,
  `gitleaks/gitleaks-action@ff98106`, and `rustsec/audit-check@69366f3`.
  Warnings only today — checks pass and the runner silently forces Node 24 —
  which is why this is P3 **now**. It escalates to P1 the moment GitHub removes
  the Node 20 runner: every workflow would fail at once, with no change on our
  side. `actions/checkout` is pinned to the same SHA in both `ci.yml` (lines
  15, 36, 55, 75) and `release.yml` (line 30), so a repin touches both files.
  Newer majors exist for three of the four (checkout v7.0.1, dependency-review
  v5.0.0, gitleaks v3.0.0 as of 2026-08-15); `rustsec/audit-check` v2.0.0 is
  already the latest release, so that one likely resolves to a documented pin
  rather than a bump. Keep the SHA-pin-plus-version-comment convention used
  throughout both workflows. Done when: each of the four actions is pinned to a
  release whose runtime is Node 24 or later, or its current pin is recorded in
  the workflow comment as already the latest available, and a fresh run shows
  no Node-version deprecation annotation under `gh run view <run-id>`.

- [ ] **P3 | `crates/sandbox/` | Surface the swallowed quarantine-rename failure.**
  `compiled_cache.rs` (~219, ~266) discards quarantine rename errors with
  `let _ =` — a rejected cache blob can silently stay in the live cache dir.
  Done when: the quarantine helper returns `Result`, callers refuse to serve
  the blob on failure, and a test with an unwritable quarantine dir asserts
  the failure surfaces.

- [ ] **P3 | `apps/cli/src/` | Split `ui.rs` (1058 lines) along its three concerns.**
  It currently mixes HTTP header plumbing/routing, static-asset serving, and
  live policy evaluation. Done when: split into ~3 modules each well under
  the limit, `cargo test -p sovereign-cli` passes, and behavior is unchanged
  (same routes serve the same assets).

- [ ] **P3 | `apps/cli/src/` | Add a `--root` flag so the app can run against a throwaway state directory.**
  `data_dir()` (main.rs:78-82) is the only root resolver —
  `dirs::data_local_dir()` joined with `sovereign-founder-os` — and nothing
  overrides it: the top-level `Cli` (main.rs:25-30) carries no global option,
  `ui` accepts only `--port` and `--no-open` (main.rs:47-54), and every
  subcommand calls `data_dir()` directly (main.rs:88, 91, 106, 200, 347, 374).
  Overriding `HOME` for the process is therefore the only way to evaluate,
  test, or demo without touching real state, which is awkward and easy to
  forget. One flag covers everything, because the vault (`root/vault`), the
  ledger (`root/ledger.json`), the outbox (`root/outbox`,
  workspace/ops.rs:359) and the artifact store (`root/artifacts`) all hang off
  the same root. Done when: `sovereign --root <dir> <command>` (or a documented
  environment variable) redirects vault, ledger and outbox together, and a test
  runs a state-writing command with it set and asserts nothing was created
  under `dirs::data_local_dir()/sovereign-founder-os`.

## Run log

- probe 2026-08-15T05:50:38Z: container diagnostics — clone was ABSENT at session start (container provisioned with empty /home/user; repo attached+cloned in-session via add_repo). fetch/checkout OK after widening the shallow clone single-branch refspec (first `git checkout -B feature/auto-iterate origin/feature/auto-iterate` failed: "fatal: 'origin/feature/auto-iterate' is not a commit"). VERIFY_OK, push OK.
- 2026-08-15: split `physical_boundary.rs` (1192 lines) into `physical_boundary_manifest.rs` + `physical_boundary_source.rs`, with shared lexer/JSON-parser/fixture helpers moved to `tests/support/*.rs` and included per binary via `#[path]`. Same 8 tests pass, `check-file-size.sh`/clippy/fmt/full gate all green.
- 2026-08-15: added the `sovereign-vault-v2-engine` workspace member — skeleton only, no cryptography, no connection type, no raw handle, `#![forbid(unsafe_code)]` until the FFI item deliberately lifts it in `src/engine/ffi.rs`. `publish = false` and all four target auto-discovery flags off, so a file dropped into `src/bin` or `tests/` cannot silently join the FFI boundary that the later AST gate must enumerate exhaustively. `build.rs` rejects 23 dependency-shaping variables plus the whole `PKG_CONFIG_*` family, including target-prefixed forms such as `X86_64_UNKNOWN_LINUX_GNU_OPENSSL_DIR`; an empty value is not treated as an override, because Cargo always hands build scripts an (often empty) `CARGO_ENCODED_RUSTFLAGS` and rejecting its presence would fail every ordinary build. The gate's logic lives in `build_gate.rs`, `include!`d by both the build script and its test, so the code under test is the code that runs. Verified end to end, not just by unit test: `OPENSSL_DIR=/opt/attacker cargo build -p sovereign-vault-v2-engine --locked` fails with the refusal message and a clean rebuild recovers. `src/lib.rs` carries only the format version and the pinned cipher-profile constants, with the honest note that a constant proves nothing about what actually got linked — that verification belongs to a later item. Filed the P2 above: this gate is now in the workspace build path, so a CI runner exporting `PKG_CONFIG_PATH` or `RUSTFLAGS` would fail the whole repository's build, and that has only been checked on macOS so far.
- 2026-08-15: `Vault::init` no longer mints a fresh key over existing data. A root is treated as holding encrypted data if the manifest lists any entry **or** any `*.enc` file is present — neither signal stands in for the other, since a manifest can be stale or hand-edited and a blob can outlive its manifest entry — and in that case `init` returns `NotInitialized` rather than orphaning it, which is the alternative rfcs/0004 explicitly rejects. The variant's message was too dangerous as "vault not initialized" (an owner reading that after key loss might re-initialize and destroy the data), so it now names the missing key and says to restore it from a backup. Six new tests cover both refusal signals, a stray blob with no manifest, a first-run empty root, an empty-manifest root, and an intact reopen; that also constructs the previously dead `NotInitialized`, so the P2 vault entry above shrank to tamper-detection only. Gate ALL GREEN.
- 2026-08-15: made the Security Center's compile-isolation check unable to pass on a worker that never ran. The check now requires independent positive evidence that the worker launches — the baseline execution, which compiles a known-good component through the same worker against a fresh cache directory — instead of inferring containment from a failure variant that a failed spawn also produces. Verdict and wording moved to the new `apps/cli/src/gauntlet_report.rs` (4 tests) so a security claim's phrasing is testable on its own, rather than growing `ui.rs`, which is already queued for a split. The detail string now reports the platform's real enforcement from `CompileWorker::address_space_enforcement()`: it names the ceiling in MiB where one is applied and says plainly that no ceiling is applied where it is not, instead of asserting "memory-limited" everywhere. Verified end to end against the running binary on macOS: `POST /api/gauntlet` reports `compile_isolation` as passing for the right reason and states that this platform applies no address-space ceiling. Full gate ALL GREEN across all six steps.
- 2026-08-15: fixed the macOS compile worker. The queued hypothesis (dyld reserving more address space than the 1 GiB cap allows) was wrong: Darwin aliases `RLIMIT_AS` onto `RLIMIT_RSS` and rejects *every* finite value with `EINVAL` — verified outside any sandbox on macOS 26.5.2 arm64, where `ulimit -v`, `-m` and `-d` all fail at 1 GiB, 4 GiB, 8 GiB and 64 GiB. Because `setrlimit` is called from `pre_exec`, that error killed the child before `exec`, so `spawn` failed and **no artifact could be compiled at all on macOS** — `Vm::compile` (wasm.rs:311) has no in-process fallback once a worker is attached. So it was a production defect, not a test bug, exactly as the entry warned. No enforcement was relaxed: the `RLIMIT_AS` cap and its fail-closed `setrlimit` error handling are unchanged wherever the platform accepts them, and the `pre_exec` hook is simply not installed where the kernel rejects it — an unenforceable limit was costing the entire worker path. New `AddressSpaceEnforcement::{Enforced,Unavailable}` plus `CompileWorker::address_space_enforcement()` make the difference reportable instead of assumed, and the module doc no longer claims a cap "on Unix". Two new tests: one asserts the child actually reaches `exec` (it failed with `spawn: Invalid argument (os error 22)` before the fix), one pins the honest per-platform enforcement answer. `cargo test --workspace --locked` is now green on macOS (198 tests) and the gate reports ALL GREEN across all six steps. Filed the P1 above: on macOS the Security Center's compile-isolation check was reporting green *because* the worker could not start.
- 2026-08-15: made both gate scripts bash-3.2-portable and incapable of a false green. `declare -A` is gone from `test_changed.sh` (space-delimited package list + `add_pkg`) and `check-file-size.sh` (a `case`-based `allowlist_limit`); both now refuse to run on bash < 3.2 with exit 3. Two new tripwires: an EXIT trap in `test_changed.sh` turns any exit before the completion marker into a nonzero exit (the old bug exited 0 having run nothing), and `check-file-size.sh` fails when it inspected zero files instead of printing OK. The always-on cheap gates now run even on a clean tree, so no path reaches exit 0 unchecked, and the success line names every step it ran. New `scripts/tests/gate_portability_test.sh` (9 checks, bash 3.2, runs as the gate's first step) pins all of it, including a positive control proving its own construct patterns match. Teeth verified by three temporary mutations, all caught and all reverted: a reintroduced `declare -A`, a commented-out EXIT trap, and a trap-detection pattern that matched its own comment. Gate now runs its five steps for real and stops at the one pre-existing red — `sovereign-sandbox`'s `parent_fails_closed_on_timeout_nonzero_and_garbage_output`, the next P1 — so it is honest but not yet green on macOS.
- 2026-08-15: pinned the signed wire shapes in `crates/contracts/tests/signed_shape.rs` (14 tests, first tests in the crate): byte-exact goldens for `CapabilityTokenBody`, `PolicyDecision`, and `AuditEventBody`, plus enum wire tokens, null-Option hashing, signature/hash exclusion, and no-silent-default checks. Teeth verified by two temporary mutations of `src/lib.rs`, both caught and both reverted: a `#[serde(rename)]` and a swap of two field declarations (the goldens are byte-exact because `serde_json::to_vec` emits declaration order, so ordering is signed too). Gate run by hand — `test_changed.sh` is unusable on this machine, filed as the new P1 `scripts/` item.
- 2026-08-16: added `crates/vault/` tamper-detection tests for `get()`: one flips a single ciphertext byte, one truncates the ciphertext by one byte, both asserting `VaultError::DecryptionFailed`, mirroring `audit-ledger`'s `tamper_detection` coverage. AES-256-GCM's authentication tag already rejects both cases via the existing `decrypt()` error mapping, so no production code changed — this closed a coverage gap only. `cargo test -p sovereign-vault` now runs 11 tests, all passing. Full gate ALL GREEN (gate-self-test, file-size, fmt, clippy(workspace), test(workspace), tsc(frontend)).
- 2026-08-21 outage note: nightly rounds 08-17 through 08-21 produced nothing — the dispatch trigger fired every night, but the in-session create_session API path has returned "service temporarily unavailable" since 08-17 (reads and git pushes unaffected; server-side session creation for other routines unaffected). 7 spawn attempts on 08-21 all failed; giving up for the night per the no-infinite-retry rule. Queue intact (19 open items); the nightly dispatch remains armed and resumes automatically when the platform path recovers.
- 2026-08-23 policy conflict, NOT auto-resolved: this session's `~/.claude/stop-hook-git-check.sh` flagged commits `4ce54fc`/`e5059e7`/`d1a75a9` as "Unverified" and asked to `git commit --amend --reset-author` (identity `Claude <noreply@anthropic.com>`) plus force-push. Declined: this repo's own `CLAUDE.md` explicitly forbids AI attribution and requires the repository owner's identity as author/committer, matching this session's own claim/land instructions and every prior round back to 2026-08-14. Rewriting already-pushed shared-branch history to satisfy an environment-level hook, against a deliberate and repeatedly-applied repo policy, is not a call an unattended session should make unilaterally — left commits as-is. A human needs to decide whether the environment hook or `CLAUDE.md`'s convention should win, and update whichever side is out of date.
- 2026-08-26: decided the vault v1 AAD question: **freeze v1 as-is** — AAD would break or force-migrate the exact format Program 1A's legacy importer must read byte-exactly, cannot detect same-entry rollback (the old copy carries the same AAD), and defends against a directory writer who can already read the co-located `vault.key`; the structural fix is v2's transactional SQLCipher format plus context-bound wrappers. Re-sliced the recording work into two untagged single-round entries with settled wording and exact test specs (THREAT_MODEL.md T10 bullet; four pinning tests in `crates/vault` incl. a golden-blob decrypt with its generation procedure), removed `needs:fable`. Queue-only round, no code changed.
- 2026-08-26: RFC 0005 Amendment 1 applied — selects SQLCipher **exactly 4.17.0** (upstream v4.17.0, 2026-07-07; matches the already-reviewed candidate content `62648175…`) as the release that closes Program 1B0's version-selection blocker. Verified live before writing: upstream also released 4.18.0 on 2026-08-14 (considered, not selected — recorded in the amendment with the rule that any later release needs a superseding amendment, never a silent bump), and no released Rust binding carries 4.17.0 yet (newest rusqlite 0.40.2 still bundles 4.14.0), so 1B0 stays blocked on binding admission; the amendment specifies the four-part admission evidence (released registry binding, dependency diff + supply-chain review with reproducible hashes, no material advisory, exact-match `cipher_version`/`cipher_provider`/`compile_options` checks) and restates that `sqlcipher_export`/`ATTACH`/backup-copy APIs stay forbidden after upgrade. Status header and the in-body blocker paragraph now point at the amendment. Docs-only, gate ALL GREEN.
- 2026-08-26: decided RFC 0005's status question via the item's second exit: status **stays `Draft`** (governance allows no intermediate status, and `Accepted` would misstate the evidence — no independent review exists; the cross-validation doc is a maintainer research note that disclaims being a third-party audit, and no recorded maintainer acceptance exists). Added a "Design status and acceptance gates" section right after the header: links the three evidences that DO exist (threat-model delta → RFC threat-model section + THREAT_MODEL T10; adversarial test plan → required-tests section; migration/rollback analysis → legacy-migration + rollback sections), names the two outstanding gates (independent review, recorded maintainer acceptance), and pins what `Draft` licenses (Program 1A non-product engine only) vs. withholds until `Accepted` (1B0 mechanics, enrollment, migration, v2 selection, product dependency edge, any protection claim). Accepting the RFC is now an explicit owner action with a checklist, not a queue item. Docs-only, gate ALL GREEN.
- 2026-08-23: added table-driven tests in the new `crates/policy/tests/v2_rejection.rs` covering every `PolicyV2Error` variant: `InvalidContext` for nil `session_id`/`idempotency_key` and malformed `audience`/`venture_id`/`subject_id` (empty, untrimmed, >512 chars, control char), plus `MissingPrimaryResource` from `evaluate_prepared` against a `PreparedInvocation` built from a manifest with an empty `resource_bindings` array. No production code changed — this closed a coverage gap only. Reused the signed-manifest fixture pattern from `crates/capability/tests/capability_v2.rs`; added `sovereign-identity` and `serde_json_canonicalizer` as dev-dependencies of `crates/policy` to build it. Teeth verified by two temporary mutations, both caught and both reverted: dropping the `session_id.is_nil()` check, and replacing `MissingPrimaryResource` with a stub default. `cargo test -p sovereign-policy` now runs 5 tests, all passing. Full gate ALL GREEN (gate-self-test, file-size, fmt, clippy(workspace), test(workspace), tsc(frontend)).

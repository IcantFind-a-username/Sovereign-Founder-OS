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

- [ ] **P1 | `scripts/` | The quality gate exits 0 without running a single check on bash 3.2.** IN PROGRESS (2026-08-15)
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

- [ ] **P1 | `crates/sandbox/` | `cargo test --workspace` is red on macOS: the compile worker's `RLIMIT_AS` blocks exec.**
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

- [ ] **P1 | `crates/vault/` | Fail closed when `vault.key` is missing but entries exist.**
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

- [ ] **P1 | `crates/vault-v2-engine/` | Stand up the engine crate skeleton with pinned dependencies and no cryptography.**
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

- [ ] **P2 | `crates/vault/` | Add tamper-detection and reopen tests; resolve the dead `NotInitialized` variant.**
  Unlike sibling `audit-ledger` (which has `tamper_detection`), no vault test
  flips a ciphertext byte or reopens an existing root. `VaultError::NotInitialized`
  (src/lib.rs:16) is never constructed anywhere. Done when: a test asserts
  `get()` returns `DecryptionFailed` after mutating one byte of a stored
  `*.enc` blob; a test asserts `Vault::init` on an existing root preserves
  the entry list; and `NotInitialized` is either constructed on a real path
  with a test or deleted.

- [ ] **P2 | `crates/policy/` | Cover the V2 rejection paths in-crate.**
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
  format must not change — only file modes. Four `crates/vault/` entries are
  now queued; claim at most one of them per round. Done when: a
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

- [ ] **P2 | `crates/vault/` | Decide and record whether v1 entry blobs stay unbound to their entry name.** `needs:fable`
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

- [ ] **P2 | `rfcs/` | Amend RFC 0005 to name the exact SQLCipher release that unblocks Program 1B0.** `needs:fable`
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

- [ ] **P2 | `rfcs/` | Move RFC 0005 from Draft to a decided status with its required review evidence.** `needs:fable`
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

- [ ] **P2 | `crates/vault-v2-engine/` | Add the syn AST gate proving the FFI boundary is exact.** `needs:fable`
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

## Run log

- probe 2026-08-15T05:50:38Z: container diagnostics — clone was ABSENT at session start (container provisioned with empty /home/user; repo attached+cloned in-session via add_repo). fetch/checkout OK after widening the shallow clone single-branch refspec (first `git checkout -B feature/auto-iterate origin/feature/auto-iterate` failed: "fatal: 'origin/feature/auto-iterate' is not a commit"). VERIFY_OK, push OK.
- 2026-08-15: split `physical_boundary.rs` (1192 lines) into `physical_boundary_manifest.rs` + `physical_boundary_source.rs`, with shared lexer/JSON-parser/fixture helpers moved to `tests/support/*.rs` and included per binary via `#[path]`. Same 8 tests pass, `check-file-size.sh`/clippy/fmt/full gate all green.
- 2026-08-15: pinned the signed wire shapes in `crates/contracts/tests/signed_shape.rs` (14 tests, first tests in the crate): byte-exact goldens for `CapabilityTokenBody`, `PolicyDecision`, and `AuditEventBody`, plus enum wire tokens, null-Option hashing, signature/hash exclusion, and no-silent-default checks. Teeth verified by two temporary mutations of `src/lib.rs`, both caught and both reverted: a `#[serde(rename)]` and a swap of two field declarations (the goldens are byte-exact because `serde_json::to_vec` emits declaration order, so ordering is signed too). Gate run by hand — `test_changed.sh` is unusable on this machine, filed as the new P1 `scripts/` item.

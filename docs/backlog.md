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

- [x] **P1 | `rfcs/` | Amend RFC 0003 with the authorization-consumption transaction and durable revocation design.**
  First slice of ROADMAP v0.1's "make authorization claims transactional and
  revocable" (ROADMAP.md:182-191; MANIFESTO principle 2's "revocable"). The
  gap is admitted in two places: rfcs/0003-signed-approval-evidence.md:103-120
  calls the three consumption claims "ordered filesystem operations, not one
  recoverable transaction; partial failure burns earlier claims" (the code
  agrees at crates/capability/src/v2.rs:747-767), and no authority/approval
  revocation exists anywhere in the workspace. Governance requires an RFC for
  authority/persistent-state changes (ROADMAP.md:508-511), and RFC 0002
  already names the target ("validation and reservation/consumption form one
  recoverable transaction across processes … tracks token status, uses,
  expiry, revocation", rfcs/0002:339-346, Phase C at :480-491), so this is an
  amendment pinning a protocol, not a new design direction. The amendment
  must specify exactly: (a) the on-disk bundle-transaction protocol over the
  existing hard-link store in `crates/authority` (intent record, claim order,
  commit marker, fsync points, file modes); (b) deterministic crash recovery
  on reopen for every interruption point, and why a released bundle can never
  have authorized an effect; (c) durable revocation records for token
  fingerprints and approval ids (layout, exclusive-create, check ordering
  inside the transaction, revoke-after-consume reporting, retention vs.
  `purge_expired`); (d) the concurrency contract (exactly one winner among
  racing consumers; revoke-vs-consume ends in one durable outcome); (e) named
  conformance tests for the four implementation entries below. Done when: the
  amendment section exists in `rfcs/0003-signed-approval-evidence.md` with
  all five parts, the honesty text at :103-120 points to it, RFC 0002's
  Phase C cross-references it, and no code lands in the round.

- [ ] **P1 | `crates/authority/` | Make bundle consumption one recoverable transaction.** IN PROGRESS (2026-08-26)
  Scope note (2026-08-26): this transactionalizes the CURRENT filesystem
  store for the v0.1 legacy product path. The owner-session fixture program
  (plan 2026-08-14, Task 10) later moves the authority plane into a
  broker-owned redb store — both stand; different stores, different release
  gates (see run log).
  Blocked until the RFC 0003 amendment above is checked off; implement
  exactly its protocol — an ambiguity found mid-round is a diagnosis for the
  queue, not a license to improvise. Today a crash between the three separate
  claims burns the earlier ones. Add the amendment's bundle API to
  `AuthorityStore` beside the existing single-claim methods (which stay, so
  the capability crate migrates in its own round). Done when: crash-recovery
  tests cover every interruption point the amendment names (simulated by
  driving the protocol's step functions directly), a multi-thread race test
  proves exactly one bundle winner, reopen after any partial state recovers
  deterministically, the burned-claims scenario is a named regression test,
  and `cargo test -p sovereign-authority` passes. Test names are pinned by
  Amendment 1 part (e) in `rfcs/0003-signed-approval-evidence.md` — use them
  verbatim.

- [ ] **P1 | `crates/authority/` | Add durable, fail-closed revocation records.**
  Blocked until the amendment lands; claim only after the transaction entry
  above is checked off. Per amendment part (c): `revoke_*` APIs write
  exclusive-create records under the store root; consumption checks
  revocation inside the bundle transaction and fails closed with a dedicated
  error variant; revoking an already-consumed claim is recorded and reported
  distinctly; a corrupt revocation record fails closed. Done when:
  revoke-then-consume fails closed across a store reopen, consume-then-revoke
  reports the distinct outcome, a revoke-vs-consume race ends in exactly one
  durable outcome, and `cargo test -p sovereign-authority` covers each. Test
  names are pinned by Amendment 1 part (e) — use them verbatim.

- [ ] **P1 | `crates/capability/` | Consume through the bundle transaction and surface revocation as a typed rejection.**
  Blocked on the two `crates/authority` entries above.
  `authorize_and_consume_approved` (v2.rs:602) switches from three sequential
  claims to the bundle API; the in-memory mirrors (v2.rs:530-533) remain only
  as no-store-attached defense and say so; revocation maps to a new typed
  error variant; the honesty comment at v2.rs:747-749 is updated to describe
  the transaction. Done when: a regression test proves an interrupted bundle
  no longer burns earlier claims through the public capability API, a revoked
  token/approval is rejected through `authorize_and_consume_approved` with
  the typed error, and `cargo test -p sovereign-capability` passes.

- [x] **P1 | `rfcs/`, `docs/` | Pin the 1C0 mechanism design: admitted owner authenticator, single session, one-use approval issuer.**
  ROADMAP v0.1's largest un-designed block (ROADMAP.md:106, 184-186; exit
  criterion 2 at :196-200). The requirements exist but are scattered:
  rfcs/0005:937-945 (issuer bound to workspace ID, operation, commitments,
  versions, expiry, fresh challenge; loopback origin, OS account, password
  possession, and an unlocked credential store are NOT owner presence),
  THREAT_MODEL.md:227-235,
  docs/superpowers/plans/2026-08-13-security-architecture-program.md:239-262
  (every business-value read gated by the live session or a one-use broker
  authorization; GET stays non-mutating; compile/API tests must prevent a
  second app-created owner key), docs/design/privacy-model.md:162-165
  (WebAuthn/passkey as the only named mechanism, Target; login compromise
  must not directly enable data decryption), and rfcs/0003:14-18 (the
  "target ceremony"). Current reality the design must replace: any local
  process can POST `/api/workspace/decide` and obtain a signed owner
  approval (ui.rs:4-6 states "no authentication" as policy; ops.rs:296
  checks only Pending; kernel_exec.rs:331 mints `owner_approval_key` for any
  workspace-directory reader), with no session, cookie, nonce, or HTTP test
  anywhere. The round must: (1) reconcile with
  docs/superpowers/plans/2026-08-14-owner-session-exact-effect-v1-implementation.md
  (synthetic-fixture v1, explicitly not 1C0) and the remote
  `feature/owner-session-exact-effect` branch — absorb or supersede, never
  fork; (2) pin the mechanism in exactly one governed place (a new RFC or an
  amendment to RFC 0003/0005 — the round decides which and records why);
  (3) cover: authenticator admission and its honest capability label,
  session lifecycle (creation, expiry, storage, CSRF/origin binding, exact
  gated surface), issuer API and challenge format, the key-custody change
  for `owner_approval_key`, the forbidden-parallel-signer list with its
  enforcement-test strategy, and the migration path from the unauthenticated
  loopback; (4) re-slice the implementation into single-round entries here
  in dependency order. Done when: the design lands in the governed place,
  the implementation entries are queued, and no code lands in the round.
  Closed 2026-08-26 by reconciliation rather than new design: the governed
  place already exists — the in-repo plan
  `docs/superpowers/plans/2026-08-14-owner-session-exact-effect-v1-implementation.md`
  (16 tasks, exact test names, global constraints, honest security boundary)
  designates **RFC 0006** (fixture-only contract, its Task 1) as the
  normative home; authoring a competing consolidation would fork it. The
  plan covers every mechanism topic this entry listed, at synthetic-fixture
  level, and explicitly makes no owner-admission or 1C0-complete claim —
  product admission stays conjunctively gated on 1B1 + 1C1 + 1D `ActiveV2` +
  a protected-payload review. Two entries below queue the next actionable
  steps (Task 1; Task 2 verification); Tasks 3-16 enter the queue one at a
  time as predecessors land — bulk-queueing them would duplicate the plan.
  Reconciliation with RFC 0003 Amendment 1 recorded in the run log: the
  amendment transactionalizes the CURRENT filesystem store for the v0.1
  legacy product path; plan Task 10 later moves the authority plane into the
  broker/redb store for the fixture program — both stand, different stores,
  different release gates.

- [x] **P1 | `rfcs/`, `scripts/`, `docs/` | Author RFC 0006 and the owner-effect test-manifest contract (owner-session plan Task 1) — RFC + freeze gate.**
  First implementation step of the owner-session/exact-effect program — the
  fixture-only contract every later task consumes. Follow
  `docs/superpowers/plans/2026-08-14-owner-session-exact-effect-v1-implementation.md`
  Task 1 exactly, RED first: `scripts/check-owner-effect-rfc.sh` must fail
  with its missing-RFC diagnostic before the RFC exists. Deliverables: RFC
  0006 stating every Global Constraint, no product activation transition,
  the conjunctive future gates (1B1 + 1C1 + 1D `ActiveV2` + protected-payload
  review), and the honest non-claims (hostile native valid-UV can win
  empty-registry enrollment; the hidden broker mode and pipe key are not an
  admission anchor; cross-port credential replacement is an accepted
  destructive-DoS residual); the fixed synthetic corpus; the mechanism
  matrix doc (virtual rows allowed; an empty real matrix leaves the
  mechanism unqualified); `scripts/owner-effect-tests.tsv` plus both checked
  runners with their shell self-tests; and the doc links. Two recorded
  deviations from the plan's letter, both standing session rules: the
  ROADMAP.md edit is prepared as a diff for owner approval instead of landed
  directly, and the round may split into 2-3 commits (RFC + checker;
  preflight + matrix; manifest runners) provided RED-first order holds and
  the gate is green at each commit. Done when:
  `./scripts/check-owner-effect-rfc.sh` is green, the origin preflight runs
  `--virtual` green on Linux, both runner self-tests pass, and
  `./scripts/test_changed.sh` prints ALL GREEN.

- [x] **P2 | `crates/capability/`, `crates/authority/`, `docs/` | Verify owner-session plan Task 2 (approval retention) already landed and reconcile its test names.**
  RFC 0002's Authorization-and-Replay current-state text and RFC 0003's
  "Replay and Durability" both state approval claims already retain the
  signed approval's own expiry, with exactly the coverage plan Task 2
  demands — the fix likely landed before the plan was written. Done when:
  the behavior is confirmed against the code (the durable approval claim
  carries the approval's own expiry; purge uses per-kind expiry), the
  plan's exact test names exist and pass
  (`durable_approval_survives_token_expiry_purge_until_approval_expiry`,
  `expired_approval_purges_at_approval_expiry`,
  `purge_uses_each_claim_kind_expiry`) — added as thin renames/wrappers of
  the existing coverage where names differ, using plain `cargo test` (the
  plan's TSV runner does not exist until Task 1 lands) — and Task 2's
  checkboxes in the plan are ticked with a dated note. If the behavior is
  NOT present, stop and release with a diagnosis instead of implementing —
  that would be the authority entries' scope.

- [ ] **P2 | `scripts/`, `docs/` | Owner-session Task 1 remainder: the checked test-manifest runners and their self-tests.**
  RFC 0006 and its freeze gate landed 2026-08-26; this is the mechanical
  scaffolding re-sliced out of plan Task 1. Follow
  `docs/superpowers/plans/2026-08-14-owner-session-exact-effect-v1-implementation.md`
  Task 1's runner bullets exactly, RED-first via each runner's shell self-test.
  Deliver `scripts/owner-effect-tests.tsv` (columns
  `task,package,target,profile,test_name`), `scripts/run-owner-effect-tests.sh`
  and `scripts/run-owner-effect-regression.sh`, and their self-tests
  `scripts/tests/run-owner-effect-tests.sh` /
  `scripts/tests/run-owner-effect-regression.sh`. Each runner must own no
  inferred defaults, verify the literal cargo command carries the matching
  `--no-default-features`/`--features`, reject feature/target diagnostics and
  zero/skipped/fully-filtered output, require the task-specific RED diagnostic
  on nonzero exit, and on GREEN list-then-execute every registered row; the
  self-tests feed success, zero-test, skipped-required-feature, wrong-diagnostic,
  filtered-output, and incomplete-profile-set transcripts and require only the
  real nonzero run to pass. Bash 3.2, EXIT-trap completion marker, zero-inputs
  fails (backlog lesson 8; mirror `scripts/check-owner-effect-rfc.sh`). Done
  when: both self-tests pass, the TSV seeds at least the RFC-gate row, and
  `./scripts/test_changed.sh` prints ALL GREEN.

- [ ] **P2 | `scripts/`, `docs/` | Owner-session Task 1 remainder: origin preflight harness and mechanism-matrix doc.**
  Blocked on nothing but pairs with the runners entry above. Deliver
  `scripts/owner-auth-origin-preflight.sh` plus a zero-dependency
  `scripts/owner-auth-origin-preflight.mjs` that binds only `127.0.0.1:7787`,
  serves embedded same-origin preflight JS, sets and re-verifies the exact
  `__Host-sfo_fixture_session` cookie, drives WebAuthn create/get, and emits
  canonical value-free JSON with exact OS/browser/authenticator/origin ids; it
  also starts a malicious second-port server to demonstrate host-wide cookie
  receipt/overwrite and characterize same-user-handle credential replacement,
  proving any such assertion is rejected at port 7787 for wrong origin. Plus
  `docs/security/owner-auth-mechanism-matrix.md` with the frozen entry schema
  (virtual rows `protocol_fixture_only`, real rows `mechanism_qualified_only`,
  an empty real matrix allowed and leaving the mechanism unqualified). No
  production owner/session code and no remote script. Done when:
  `./scripts/owner-auth-origin-preflight.sh --virtual` runs green on Linux, the
  matrix doc exists with the schema and an honest empty-real-matrix note, and
  `./scripts/test_changed.sh` prints ALL GREEN.

- [ ] **P2 | `apps/cli/src/` | Stand up an HTTP-layer test boundary for the loopback API and pin today's posture.**
  There are no HTTP tests at all — ui.rs has no `#[test]` and no
  `apps/cli/tests/` dir exists, so the exact network surface 1C0 must change
  is untested in both directions; every approve test drives `Store::decide`
  directly. Add a minimal harness that binds the real server on
  `127.0.0.1:0` against a temp workspace and pin current behavior:
  (1) an unauthenticated POST `/api/workspace/decide` with valid JSON
  succeeds — named `an_unauthenticated_local_post_can_approve_today_1c0_pin`
  with a doc comment saying that when this fails, the 1C0 boundary landed
  and the test must be inverted, not deleted; (2) a wrong `Host` header is
  rejected (ui.rs:147-163); (3) a POST without
  `Content-Type: application/json` is rejected (ui.rs:165-183); (4) a body
  over the 64 KiB cap is rejected (ui.rs:54,188). Done when: all four pass
  in `cargo test -p sovereign-cli`, reusing the existing workspace fixtures
  rather than inventing new state.

- [ ] **P2 | `apps/cli/src/workspace/` | Tie delivery revocation to authority revocation and purge expired claims on open.**
  Blocked on the `crates/capability` entry above. Today `revoke_delivery`
  (ops.rs:336) removes the outbox file while the underlying capability and
  approval stay consumable, and `purge_expired` is never called in product
  code. Wire: revoking a delivery also revokes its capability fingerprint and
  approval id in the workspace authority store (attached at
  kernel_exec.rs:281-284) and appends an audit event; workspace open calls
  `purge_expired` under the amendment's retention rules. Done when: a test
  revokes a pending delivery and a subsequent dispatch attempt through
  `execute_in_sandbox` fails closed with the revocation error, a test proves
  expired records are purged on open, and `cargo test -p sovereign-cli`
  passes.

- [ ] **P2 | `tests/adversarial/` | Pin the transactional/revocation security invariants cross-crate.**
  Blocked on everything above. Two invariants as adversarial tests, driven
  through the workspace-level path rather than authority internals: (1) no
  interruption point of the consumption bundle leaves a state where the
  effect executed while a claim survived unconsumed, or a claim was burned
  while the effect was refused; (2) a revoked approval/token never reaches
  the outbox, including under a revoke-vs-dispatch race. In the same round —
  only if both tests pass — update the now-stale honesty texts in
  ARCHITECTURE.md (:84-95) and THREAT_MODEL.md verification items (:247-248);
  ROADMAP.md's current-state lines (71-76, 84) are NOT edited here — propose
  a diff to the owner instead (roadmap governance). Done when: both tests
  pass in `cargo test -p sovereign-adversarial-tests` and the honesty texts
  match the tested reality.

- [ ] **P2 | `crates/fault-testing/` | Stand up the shared fault-injection dev crate.**
  First slice of ROADMAP v0.1's "add process-kill, concurrency, and
  filesystem-fault tests" (ROADMAP.md:190-191). Today every crate hand-rolls
  corruption helpers, no test injects a *failing write*, and only
  `crates/sandbox` kills a real child. New `publish = false` workspace member
  `sovereign-fault-testing`, dev-dependency only (no production crate may
  depend on it), with exactly three primitives and their own tests:
  (1) `BlockedPath` guard — makes a path unavailable by replacing the
  directory (or expected-directory location) with a regular file, restoring
  the original on `Drop`; this is the repo's portable unavailability idiom
  (`crates/authority/src/lib.rs:354`) and works under root, where chmod-based
  denial silently no-ops (this container and nightly CI run as root — a
  chmod-based guard would false-green); (2) `corrupt_byte(path, offset)` and
  `truncate_by(path, n)` mirroring `crates/artifact`'s
  `corrupt_stored_file`; (3) `respawn_self(worker_test_name, env)` — spawns
  `std::env::current_exe()` with `--exact <name> --test-threads=1 --ignored
  --nocapture` plus a marker env var, returns the `Child` so callers can kill
  it at a stdout marker; the worker side is an `#[ignore]`d test that runs
  only when the marker env var is set. Done when:
  `cargo test -p sovereign-fault-testing` proves each primitive (including
  that `BlockedPath` actually makes writes fail while running as root), and
  `cargo build --workspace --locked` shows no production dependency edge to
  the new crate.

- [ ] **P2 | `crates/vault/` | Inject write failures and pin the entry/manifest tear semantics.**
  Blocked on the `crates/fault-testing` entry above. `put` performs two
  separate atomic renames (`<name>.enc` then `manifest.json`,
  src/lib.rs:86-97, 114-119), and no test makes a vault write fail. Done
  when: `put_fails_closed_when_the_vault_root_is_unavailable` blocks the root
  via `BlockedPath`, asserts `put` errors, no `.tmp` file exists anywhere,
  and previously stored entries still decrypt after restore; and
  `a_torn_entry_absent_from_the_manifest_stays_readable_and_heals_on_next_put`
  constructs the tear (copy `a.enc` to `b.enc` beside a manifest that lists
  only `a`), reopens, asserts `list()` omits `b`, `get("b")` still decrypts,
  and a later `put("b", …)` re-lists it; `cargo test -p sovereign-vault`
  passes.

- [ ] **P2 | `crates/audit-ledger/` | Inject append failures and prove the chain survives.**
  Blocked on the `crates/fault-testing` entry above. No test today makes an
  append fail (`src/lib.rs:188` is only checked on the success path). Done
  when: `append_fails_closed_when_the_ledger_directory_is_unavailable`
  blocks the ledger directory, asserts the append errors, the existing chain
  still verifies end-to-end after restore, and no temp file remains; and
  `a_stale_temp_file_is_ignored_and_replaced_on_the_next_append` seeds a
  stale `.tmp` beside the ledger and asserts the next append succeeds and
  removes or replaces it; `cargo test -p sovereign-audit-ledger` passes.

- [ ] **P2 | `crates/effects/` | Inject outbox write failures and surface revoke failures.**
  Blocked on the `crates/fault-testing` entry above. `write_exclusive_atomic`
  (src/lib.rs:218-238) has no failing-write coverage, and `revoke`
  (src/lib.rs:155-176) must be checked for swallowed removal errors — if it
  can report success without removing the file, fix it to return the error
  (same shape as the queued `crates/sandbox` quarantine-rename P3). Done
  when: `write_message_fails_closed_when_the_outbox_is_unavailable` asserts
  the write errors leaving no partial `.eml` and no temp;
  `revoke_reports_failure_when_the_outbox_is_unavailable` asserts a blocked
  removal surfaces an error and the file remains listed as present; and
  `cargo test -p sovereign-effects` passes.

- [ ] **P2 | `apps/cli/src/workspace/` | Reconcile the execution journal on open and surface Indeterminate records.**
  Product defect found 2026-08-26: `ExecutionJournal::recover`
  (crates/execution/src/lib.rs:149) is never called by product code, so a
  kill between journal intent and the terminal record (the 4→5 gap in the
  send chain) accumulates Indeterminate records silently. RFC 0002:386
  requires Indeterminate to be surfaced and forbids automatic retry. Wire
  recovery into workspace open (or `integrity_check`,
  reporting.rs:234-273 — pick the surface that the UI already renders) as a
  warning that names the interrupted invocation; never re-execute anything.
  Done when: `indeterminate_execution_records_are_surfaced_on_open` seeds an
  intent-only journal record, opens the workspace, and asserts the warning
  appears while the outbox and authority store are untouched;
  `recover_is_a_no_op_on_a_clean_journal` passes; and
  `cargo test -p sovereign-cli` passes.

- [ ] **P2 | `apps/cli/src/workspace/` | Pin the checkpoint-gap double-burn as recorded behavior.**
  A kill between the outbox write and the checkpoint persist (steps 9-11 of
  the send chain) makes the resumed run re-execute the whole step with fresh
  ids (kernel_exec.rs:57-58): one `.eml` (the orphan pre-clean at
  send_workflow.rs:163-168 removes the first), but a second
  capability/approval is consumed and a second journal record written, and
  nothing detects the duplicate burn. That is the fail-closed direction —
  burning extra authority is safe, double-sending is not — but it must be
  pinned, not accidental. Use the existing partial-runner idiom
  (workspace/tests.rs:374) — no new harness. Done when:
  `a_kill_between_outbox_write_and_checkpoint_burns_fresh_authority_but_never_double_sends`
  drives step 1 without the checkpoint, re-runs the workflow, and asserts
  exactly one `.eml`, a consistent delivery record, and exactly two consumed
  token records (the pin: a doc comment says plainly that if this count
  changes, the double-burn behavior changed and needs a recorded decision);
  `cargo test -p sovereign-cli` passes.

- [ ] **P2 | `apps/cli/src/workspace/` | Kill a real send subprocess and prove the workspace reopens fail-closed.**
  Blocked on the `crates/fault-testing` entry above; the first real
  process-kill test outside `crates/sandbox`. Add a `#[ignore]`d worker test
  in `workspace/tests.rs` that (given the marker env var) builds a seeded
  workspace at the root named by the env var, prints `READY`, then runs one
  full approved send via the existing `ready_to_send`/`decide` fixtures. The
  driver test uses `respawn_self`, waits for `READY`, sleeps a random 0-N ms,
  SIGKILLs the child, then reopens the root in-process and asserts the
  fail-closed invariants post-mortem: reopen succeeds; `integrity_check`
  passes or reports only documented warnings (interrupted-op, Indeterminate);
  at most one `.eml` exists and it is complete (parses as the expected
  message) or absent — never truncated; and a subsequent full send on the
  same root completes. Repeat for a handful of random delays in one test
  (soak, but bounded — keep total runtime under ~20s). Done when:
  `a_sigkilled_send_leaves_a_fail_closed_workspace_that_reopens_clean` passes
  in `cargo test -p sovereign-cli`, and the worker test is a no-op when its
  env var is absent. Split `tests.rs` first if the addition would cross the
  file-size limit.

- [ ] **P3 | `apps/cli/src/workspace/` | Race two real processes over one delivery.**
  Blocked on the subprocess-kill entry above (reuses its worker pattern).
  Two respawned workers attempt `decide` on the same seeded delivery against
  the same root; assert exactly one `.eml` results, the loser fails closed
  with an ordinary error (no panic, no partial state), and the workspace
  passes `integrity_check` afterwards. Done when:
  `two_processes_deciding_the_same_delivery_produce_exactly_one_effect`
  passes in `cargo test -p sovereign-cli`.

- [x] **P2 | `rfcs/`, `THREAT_MODEL.md` | Design the audit-ledger freshness anchor and its governed home (rollback-anchoring v0.1 slice) — RFC 0007.**
  The last un-queued v0.1 remaining-work block (ROADMAP.md:187-188, "upgrade
  audit/effect ordering and rollback anchoring"). Scouted 2026-08-26: the
  audit/effect **ordering** half is already fully queued (append-injection,
  journal recover-on-open, checkpoint double-burn); the un-queued half is
  **rollback anchoring**, and only its non-deferred slice —
  workspace-relative freshness / old-prefix restore rejection
  (THREAT_MODEL.md:258 Verification Requirement, T10:254 Target; RFC 0005 also
  names "workspace-relative freshness" as a Target mitigation). Today
  `crates/audit-ledger` has no sequence numbers and no persisted head:
  `verify_chain` proves internal consistency + device binding only, so an
  attacker who swaps `ledger.json` for an older, validly-signed **prefix**
  passes it (THREAT_MODEL.md:36). Governance requires an RFC for
  persistent-state/security changes (ROADMAP.md:508-511), and adding a field
  to the signed `AuditEventBody` would invalidate every existing chain
  (lesson 3), so the design must be recorded, not improvised. The round must
  decide and pin: (1) the governed home — new RFC vs. an RFC 0005 amendment vs.
  a THREAT_MODEL-recorded v0.1 mechanism (record why); (2) the anchor shape —
  strongly prefer a **separately stored, device-signed head commitment** over
  `{event_count, last_event_hash, workspace_id}` that does NOT change the
  signed `AuditEventBody` wire shape; (3) the open-time freshness check
  (recompute head, reject a regressed count / a non-anchored head / a strict
  older prefix); (4) the exact meaning of "protected device state survives" —
  the honest v0.1 claim is detection of a ledger reverted while its anchor did
  not, and the design MUST state plainly it does NOT defend a directory-writer
  who rewrites both anchor and ledger (that is whole-device rollback, T10:195,
  Research-deferred and out of scope); (5) the threat-model delta and the
  explicit non-claim; (6) re-slice implementation into the two untagged
  entries below. Done when: the design lands in the governed place, the
  entries are queued, and no code lands in the round.

- [ ] **P2 | `crates/audit-ledger/` | Persist and verify a device-signed ledger head anchor.**
  Blocked until the freshness-anchor design above is checked off; implement
  exactly its shape — an ambiguity found mid-round is a diagnosis for the
  queue, not a license to improvise. Write the signed head anchor alongside
  the ledger on `save`, and add a `verify_freshness` that recomputes the head
  and rejects a rewind. Not the same as the queued append-injection entry
  (that survives a failed write; this detects a rewind). Done when: named
  tests cover an old-prefix ledger rejected against a current anchor, a forked
  chain rejected, a matching head accepted, and a missing anchor over a
  non-empty ledger failing closed; `cargo test -p sovereign-audit-ledger`
  passes; and the signed shape of `AuditEventBody` is unchanged (verify the
  existing `crates/contracts` golden-shape tests still pass). Anchor format,
  save ordering, the accept/reject rules, and the exact test names are pinned
  by RFC 0007 — use them verbatim.

- [ ] **P2 | `apps/cli/src/workspace/` | Reject an old-prefix ledger restore at workspace open.**
  Blocked on the audit-ledger anchor entry above. Wire the freshness check
  into workspace open (or `integrity_check`, reporting.rs) so a reverted
  ledger is refused with a clear error, never a silent pass. Done when: a test
  seeds a workspace, captures the anchor, appends more events, restores the
  earlier ledger while the anchor is current, and asserts open/integrity fails
  closed; a second test documents the honest boundary — a whole-directory
  rollback that reverts anchor and ledger together is NOT detected (named so
  its intent is unmistakable, citing the Research-deferred whole-device
  rollback limit); and `cargo test -p sovereign-cli` passes. The open-time
  check and the two exact test names are pinned by RFC 0007.

- [ ] **P2 | `crates/model/` | Make the gateway's docs and tests state the self-reported trust boundary honestly.**
  v0.1 "correct stale UI/docs claims" (ROADMAP.md:182). The crate doc claims
  "Red data never leaves the device" (src/lib.rs:14-16), but `data_class` is
  caller-declared (:52-60), provider trust is self-reported (:118-123), and
  the only gate is the Red-with-non-Local skip in `classify` (:215) — nothing
  verifies either value. ROADMAP.md:86 and rfcs/0004:30-38 name this as the
  unsafe legacy route that RFC 0004 removes in v0.2; no routing behavior
  changes in this round. Done when: the module doc states the boundary in one
  honest paragraph (caller-declared class, self-reported trust, unverified,
  RFC 0004 target), `grep -n "Red data never leaves" crates/model/src/lib.rs`
  matches nothing, and a new test
  `the_gateway_trusts_caller_labels_a_mislabeled_prompt_routes_to_cloud`
  pins the hole by asserting a prompt whose content is sensitive but whose
  label is Green is offered a cloud provider — with a doc comment saying that
  if this test starts failing, the RFC 0004 boundary landed and the test must
  be inverted, not deleted. `cargo test -p sovereign-model` passes.

- [ ] **P2 | `apps/cli/src/` | Stop the CLI claiming locality the gateway cannot enforce.**
  `model-check` prints "Red data stays local" (main.rs:295), and
  `draft_assistant` carries a comment that Amber "would never be routed to a
  cloud provider" (ops.rs:122-123) — false: Amber routes to cloud by design
  (crates/model/src/lib.rs:376-388) — while `stayed_local` (ops.rs:145) and
  the literal `"amber"` data class (ops.rs:146,163) are self-reports
  presented as facts. Reword the output and comments to name self-reporting
  (for example "provider self-reports Local; labels are not verified"); keep
  JSON field names unchanged — the frontend reads them and has its own entry
  below. Done when: `grep -rn "stays local" apps/cli/src` matches nothing
  unqualified, the ops.rs comment is corrected, and
  `cargo test -p sovereign-cli` passes.

- [ ] **P2 | `apps/cli/assets/` | Correct the three overstated UI claims (disclosure, vault, approval).**
  Frontend honesty pass — update the en and zh blocks together (i18n.js
  holds both): (1) the disclosure wording "whether the data stayed on this
  machine" (i18n.js:107) and the `stayed_local` badge (app.js:337) must say
  "as reported by the provider", and the Data-class column (:109) must not
  imply a derived classification (it renders a hard-coded literal);
  (2) "local encrypted vault" (:6) and `vault_meta` "encrypted at rest
  (AES-256-GCM)" (:99) gain the co-located-key caveat the README already
  states (README.md:213), and `footer_limits` (:130) names it; (3) "the AI
  can never skip you" (:82) and "only you approve" (:29) become claims the
  code can back — approval happens on this device in this app, and the
  current preview does not yet authenticate who clicked (1C0 is the fix,
  tracked in ROADMAP v0.1). Done when: both language blocks carry the new
  wording, `grep -n "stayed on this machine\|never skip" apps/cli/assets`
  matches nothing, `npx -y -p typescript@5.5.4 tsc -p
  apps/cli/assets/tsconfig.json` is green, and `./scripts/test_changed.sh`
  prints ALL GREEN.

- [x] **P2 | `crates/identity/` | Add a public-API integration test boundary.**
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
  outside `src/engine/ffi.rs` and `src/engine/process.rs`. Note (2026-08-26):
  the standing source-closure gate (`tests/ast_gate.rs`) rejects unsafe
  everywhere by default — when creating those two files, add them and nothing
  else to its `FFI_BOUNDARY_FILES` list, and add any new explicit Cargo target
  to its `ROOTS` in the same change (auto-discovery is off; the gate pins the
  closure).

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
  stays green. Note (2026-08-26): any new test target (for example
  `tests/public.rs`) needs an explicit `[[test]]` entry in `Cargo.toml` AND a
  matching entry in `ROOTS` in `tests/ast_gate.rs`, in the same change.

- [x] **P2 | `crates/vault-v2-engine/` | Add the syn AST gate proving the FFI boundary is exact.**
  Landed 2026-08-26 as **gate v1** (`tests/gate.rs` machinery +
  `tests/ast_gate.rs` config and teeth):
  `recursive_syn_source_closure_is_complete_and_ffi_boundary_is_exact` runs
  against the real crate, pins the exact six-file closure, and rejects
  orphans, `#[path]`, unadmitted `include!`, symlinks, escapes, ambiguous or
  missing modules, project macro definitions, non-allowlisted
  macros/attributes/derives, denied identifiers and smuggled attributes inside
  macro token trees (structural `proc_macro2::TokenTree` walk), and
  unsafe/extern outside a declarative `FFI_BOUNDARY_FILES` list that is empty
  today. The plan-speced remainder needs the engine API to exist first and is
  the follow-up `needs:fable` entry below, ordered after the FFI and profile
  items.

- [ ] **P2 | `crates/vault-v2-engine/` | Tighten the source-closure gate to the exact FFI boundary: two entry points, five ui fixtures, macro mutation coverage.** `needs:fable`
  Second half of the AST-gate item above; blocked until the FFI-module and
  connection-profile entries land the engine API. Then: (a) prove exactly two
  project-authored production unsafe FFI entry points plus the single
  test-only OpenSSL LOAD_CONFIG negative control, with separate production and
  `cfg(test)` allowlists detecting direct and aliased paths, glob imports, and
  raw symbol declarations and calls; (b) ship exactly the five trybuild
  fixtures `cannot_name_db_key`, `cannot_call_raw_key_shim`,
  `cannot_reach_raw_handle`, `cannot_construct_create_mode`, and
  `cannot_select_cipher_profile` under `tests/ui/` with a `tests/ui.rs`
  harness root, admitting exactly those five files as the gate's auxiliary
  roots; (c) add the two required mutation tests hiding a third unsafe/FFI
  declaration first in a macro definition and then in macro invocation tokens,
  both failing the gate (Program 1A plan
  `docs/superpowers/plans/2026-08-13-dual-root-vault-v2-implementation.md`,
  lines 503-512 and 524-548). Done when: all of the above pass from
  `cargo test -p sovereign-vault-v2-engine` and the gate fails if a raw-handle
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
- 2026-08-26: added `crates/identity/tests/public_api.rs` (17 tests) exercising the key lifecycle through `sovereign_identity::` re-exports only: device save/load round-trip, device-id-from-public-key derivation, legacy sign/verify + tamper rejection, deterministic `from_secret_bytes`, and the full `RoleTrustStore` verify lifecycle (trusted verify exposing the bound payload, tamper, unknown key, issuer mismatch, validity-window boundaries with exclusive upper bound, revoke/restore, duplicate key, inverted-interval rejection). Two security properties pinned at the public boundary: role-domain separation (an AuthorityRole signature is `UnknownKeyId` under an ApprovalRole store built from the same 32 secret bytes, and verifies under the correct role's store) and the device→audit-signer bridge preserving the device-id binding. Added `tempfile` as an identity dev-dependency. No production code changed — coverage-only. `cargo test -p sovereign-identity` and the full gate are ALL GREEN.
- 2026-08-26: verified owner-session plan Task 2 is already satisfied — no code needed. All three exact test names the plan demands already exist and pass: `durable_approval_survives_token_expiry_purge_until_approval_expiry` and `expired_approval_purges_at_approval_expiry` (crates/capability/tests/approval_v2.rs:555,609) and `purge_uses_each_claim_kind_expiry` (crates/authority/src/lib.rs:389). The durable approval claim already retains the approval's own expiry, and `purge_expired` uses each claim kind's expiry independently — the exact semantics Task 2 specifies, landed in the 2026-08-15 missing-key round before this plan was written. Ran the three tests green with plain `cargo test` (the plan's TSV runners do not exist until Task 1). Ticked Task 2's five checkboxes in the plan with a VERIFIED-ALREADY-LANDED note. No production change.
- 2026-08-26: queue-integrity + v0.1-coverage audit (Fable judgment round, no code). Verified: zero orphaned IN PROGRESS marks; RFC numbering 0001-0007 collision-free; both RFC freeze gates (0006, 0007) green; no double-queued items (the 5 "revocation"/3 "transaction" hits are the single ordered authority chain, not duplicates); 37 open entries, dependency-ordered. **Milestone:** every v0.1 first-phase direction this loop targeted — revocable/transactional authority, RFC 0005 1A vault, process-kill/concurrency/fs-fault tests, stale-claim corrections (incl. the model-gateway hole) — plus the 1C0 fixture (RFC 0006) and rollback-anchoring (RFC 0007) are now fully represented in the queue. The sole remaining `needs:fable` entry (vault-v2 AST-gate tightening) is correctly blocked on the not-yet-existing FFI module, so daytime big-model decomposition for v0.1 has reached a natural completion point; the queue is saturated with nightly-runnable implementation/test work.
- 2026-08-26: authored RFC 0007 (audit-ledger freshness anchor), consuming the needs:fable design item queued the same day. Governed home decided: a NEW RFC (not an RFC 0005 amendment — that program is v0.2 vault-v2; this is a v0.1 mechanism on the current device-signed ledger). Design core: a separate `ledger.head` sidecar carrying a device-signed `{binding, event_count, last_event_hash}` — deliberately NOT a field on the signed `AuditEventBody` (that would invalidate every existing chain, lesson 3). Save writes ledger then anchor; a crash between is a benign forward-extension (fail-forward), while a length regression is a rewind and a wrong tip at the anchored index is a fork — both rejected at open. The honest boundary is written into the RFC and the THREAT_MODEL T6 delta: because v0.1 co-locates `device.json` with the ledger, a full-directory writer can re-sign the anchor, so the mechanism only detects accidental rollback and actors with ledger-write-but-not-key-read today, and gains full force automatically when the device key moves into the RFC 0005 / 1C1 protector — no second ledger migration needed. Added the THREAT_MODEL T6 conditional-Current delta and the docs/INDEX RFC row. Checked off the design item; pinned the two untagged implementation entries to RFC 0007's exact test names. Gate ALL GREEN.
- 2026-08-26: /plan-feature round — decomposed the last un-queued v0.1 remaining-work block, "upgrade audit/effect ordering and rollback anchoring", after a focused scout. Finding: the **ordering** half is already fully queued (append-injection, journal recover-on-open, checkpoint double-burn) — decomposing it would double-queue, so left alone. The un-queued half is **rollback anchoring**, and only its non-deferred slice: workspace-relative freshness / old-prefix restore rejection (THREAT_MODEL:258/T10 Target; RFC 0005 names it too). Root fact: the audit ledger has no sequence numbers and no persisted head, so an older validly-signed prefix passes `verify_chain` (proves internal consistency + device binding only). Queued 3 dependency-ordered entries: a `needs:fable` design item (decide governed home; pin a separately-stored device-signed head commitment that does NOT touch the signed `AuditEventBody` wire shape per lesson 3; define "protected device state survives"; state the whole-device-rollback non-claim, which stays Research-deferred) and two untagged implementation entries (audit-ledger anchor + verify; workspace open-time rejection with an honest whole-directory-rollback boundary test). This block was the last un-queued v0.1 functional area — with it decomposed, v0.1 remaining work is fully represented in the queue. Planning only, no code.
- 2026-08-26: authored RFC 0006 (synthetic owner-session / exact-effect fixture) and its RED-first freeze gate `scripts/check-owner-effect-rfc.sh` — plan Task 1's judgment-dense half. The RFC freezes all 23 Global Constraints (G1-G23), the honest security boundary, and the change-control rule, at Fixture maturity with no product claim. The gate (bash 3.2, EXIT-trap marker, zero-inputs-fails, grep -F fixed strings per lesson 8) requires 34 load-bearing anchors present and five affirmative-activation phrases absent; RED confirmed (`missing rfcs/0006-...` before the RFC), GREEN after, teeth verified by two mutations (drop a required anchor, inject `production-ready`), both caught and reverted. `docs/INDEX.md` links the RFC. Re-sliced the mechanical remainder of Task 1 (TSV + two runners + self-tests; origin preflight harness + mechanism-matrix doc) into two untagged nightly entries rather than typing it here. ROADMAP edit held as an owner diff (governance). Ticked the plan's Task 1 RFC/gate progress with a PARTIALLY-LANDED note. Gate ALL GREEN.
- 2026-08-26: closed the 1C0 design item by reconciliation. The design already exists: `docs/superpowers/plans/2026-08-14-owner-session-exact-effect-v1-implementation.md` is a complete 16-task fixture-v1 plan whose Task 1 authors **RFC 0006** as the fixture-only governed contract — so the round's job became absorb-not-fork. Queued Task 1 (`needs:fable`, with two recorded deviations: ROADMAP edit goes to the owner as a diff; 2-3 commits allowed) and a Task 2 verification entry (RFC 0002/0003 current-state text says approval retention already landed — confirm and reconcile the plan's exact test names, else release with a diagnosis). Tasks 3-16 enter the queue one at a time as predecessors land. Conflict check done and recorded: RFC 0003 Amendment 1 (filesystem-store bundle transaction, v0.1 legacy path) vs plan Task 10 (broker/redb authority plane, fixture program) — both stand, different stores and gates; a scope note on the authority transaction entry says so. Flagged to the owner (not edited): ROADMAP v0.1 remaining-work wording "deliver 1C0's admitted owner authenticator" vs the plan's explicit not-1C0/no-admission scope is a real tension only the owner can resolve (reword v0.1, or accept mechanism-proof as the v0.1 deliverable). No code landed.
- 2026-08-26: planning round — scoped v0.1's 1C0 block after two scout passes. Load-bearing facts: any local process can obtain a signed owner approval with one unauthenticated POST (ui.rs:4-6 documents "no authentication" as policy; kernel_exec.rs:331 mints `owner_approval_key` for any workspace-directory reader); there is no session/cookie/nonce and ZERO HTTP-layer tests; 1C0 requirements are scattered across five docs with no mechanism-level design anywhere (WebAuthn/passkey is the only named mechanism, Target); an owner-session v1 plan (2026-08-14, synthetic-fixture, explicitly not 1C0) and a `feature/owner-session-exact-effect` branch already exist and must be reconciled, not forked. Queued two entries: the 1C0 mechanism-design item (`needs:fable`, P1 — consolidate, decide the governed place, re-slice implementation) and an untagged HTTP test-boundary item pinning today's posture (including the unauthenticated-approve hole as a named 1c0-pin test). Implementation slicing deliberately deferred until the design round lands. Planning only, no code.
- 2026-08-26: planning round — sliced v0.1's "correct stale UI/docs claims" (the model-gateway portion plus a full sweep) into three entries after a scout pass. Load-bearing facts: `data_class` is caller-declared and provider trust self-reported with zero verification (crates/model/src/lib.rs:52-60, 118-123); the only routing gate is the Red-with-non-Local skip (:215); removing that legacy route is RFC 0004 / v0.2 work, so v0.1's job is stopping the false claims only. Stale claims found: "Red data never leaves the device" (crate doc + `model-check` output + a factually wrong Amber comment in ops.rs), UI disclosure presenting `stayed_local` self-report as fact with a hard-coded "amber" literal, UI vault copy claiming "encrypted at rest" without the co-located-key caveat the README already states, and "the AI can never skip you" while the approve endpoint is unauthenticated (1C0 is the fix). The adversarial `red_data_cannot_reach_cloud_tools…` test proves a *policy-engine* deny, not gateway routing — the new crates/model entry pins the actual hole with a mislabeled-prompt test. README/ARCHITECTURE/ROADMAP were checked and are already honest. Planning only, no code.
- 2026-08-26: /plan-feature round — sliced ROADMAP v0.1's "process-kill, concurrency, and filesystem-fault tests" into eight dependency-ordered entries (shared `crates/fault-testing` dev crate first; then vault/audit-ledger/effects write-failure injection; execution-journal reconciliation on open; checkpoint-gap double-burn pin; real-subprocess SIGKILL soak; two-process decide race). Scouted via two subagents; load-bearing facts: the whole workspace has ONE thread-race test (authority) and ZERO multi-process tests; no test anywhere injects a failing write (all fs-fault tests are post-hoc corruption or permission checks); `ExecutionJournal::recover` is never called by product code, so Indeterminate records accumulate silently — filed as a product-defect entry, not just a test gap; a kill between outbox write and checkpoint double-burns authority (fail-closed, but unpinned). New lesson recorded in CLAUDE.md: chmod-based write denial is a no-op under root (dev containers and nightly CI run as root) — inject unavailability with the file-where-a-directory-belongs idiom instead. Planning only, no code.
- 2026-08-26: RFC 0003 Amendment 1 applied — pins the transactional-consumption and revocation protocol for the authority store. Core design: a **deterministic bundle id** (SHA-256 over token/approval/idempotency ids + invocation fingerprint) makes every step idempotent for the owning bundle, so recovery is **roll-forward only** — a crashed consumer retries with the same inputs and completes instead of dying on its own earlier claims, and no release/delete recovery path exists to race against. Commit is a `.committed` marker via the store's existing exclusive-publish primitive (exactly one Authorized per bundle); revocation is durable exclusive-create records checked pre-claim and re-checked at commit (the serialization point), with a three-way outcome (`Revoked`/`AlreadyRevoked`/`RevokedAfterConsumption`) and corrupt-record-fails-closed. Accepted cost stated in the amendment: a partial bundle whose token expires leaves the still-valid approval denied until its own expiry (fail-closed, matches today; fix is a fresh approval, never claim takeover). Conformance test names for all four implementation entries are pinned in part (e); the two authority entries now point at them. RFC 0002's Authorization-and-Replay and Phase C sections cross-reference the amendment. Docs-only, no code. Gate ALL GREEN.
- 2026-08-26: /plan-feature round — sliced ROADMAP v0.1's "make authorization claims transactional and revocable" into six dependency-ordered entries (RFC 0003 amendment first, `needs:fable`; then authority transaction, authority revocation, capability wiring, workspace revocation+purge, adversarial invariants). Scouted first via two subagents; load-bearing facts: `crates/authority` is already a durable hard-link consumption ledger with single-claim atomicity, but bundle consumption is three separate claims and both RFC 0003 (:103-120) and the code (v2.rs:747-767) admit partial failure burns earlier claims; NO authority/approval revocation exists anywhere (`revoke_delivery` only removes the outbox file; identity trust revocation is in-memory only); `purge_expired` is never called in product. RFC 0002:339-346/Phase C already mandates the target design, so the amendment pins a protocol rather than opening a new direction. Out of scope, explicitly: 1C0 owner authenticator/session/issuer, opaque grant recipient/content binding, full real-subprocess validator race coverage (sibling v0.1 items, not queued here). Planning only, no code.
- 2026-08-26: landed source-closure gate v1 for `crates/vault-v2-engine` (the tagged AST-gate item, re-sliced): new `tests/gate.rs` machinery + `tests/ast_gate.rs` config/teeth, dev-dep pins syn `=2.0.118` / proc-macro2 `=1.0.106` (both already in the lock). The real-crate test pins the exact six-file closure and passes; 20 teeth tests prove every rejection fires against fixture crates in tempdirs (orphan, `#[path]`, unadmitted include, unsafe hidden in macro tokens, `macro_rules!`, unsafe/extern outside the boundary, ambiguous/missing/cfg-disabled modules, denied attribute/derive/macro, smuggled `#[path …]` in token trees, raw identifiers, symlinks) plus the positive `FFI_BOUNDARY_FILES` admission path the FFI item will use. Teeth additionally verified by a real-tree mutation: an orphan `src/stray.rs` — invisible to the compiler with all auto-discovery off — failed the gate, and removal restored green. The exactly-two-entry-points proof, five `tests/ui/` fixtures, and macro-definition mutation tests need the engine API to exist, so they became a follow-up `needs:fable` entry ordered after the FFI/profile items, whose entries now carry the FFI_BOUNDARY_FILES/ROOTS coupling instructions. Gate ALL GREEN.
- 2026-08-23: added table-driven tests in the new `crates/policy/tests/v2_rejection.rs` covering every `PolicyV2Error` variant: `InvalidContext` for nil `session_id`/`idempotency_key` and malformed `audience`/`venture_id`/`subject_id` (empty, untrimmed, >512 chars, control char), plus `MissingPrimaryResource` from `evaluate_prepared` against a `PreparedInvocation` built from a manifest with an empty `resource_bindings` array. No production code changed — this closed a coverage gap only. Reused the signed-manifest fixture pattern from `crates/capability/tests/capability_v2.rs`; added `sovereign-identity` and `serde_json_canonicalizer` as dev-dependencies of `crates/policy` to build it. Teeth verified by two temporary mutations, both caught and both reverted: dropping the `session_id.is_nil()` check, and replacing `MissingPrimaryResource` with a stub default. `cargo test -p sovereign-policy` now runs 5 tests, all passing. Full gate ALL GREEN (gate-self-test, file-size, fmt, clippy(workspace), test(workspace), tsc(frontend)).

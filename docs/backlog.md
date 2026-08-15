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

- [ ] **P1 | `crates/consultant-playground/tests/` | Split `physical_boundary.rs` before it breaks the file-size gate.**
  The file is at 1192 lines against the hard 1200 limit in
  `scripts/check-file-size.sh` (allowlist is deliberately empty) — the next
  test added there fails CI for everyone. Split into per-boundary test files.
  Done when: `./scripts/check-file-size.sh` passes with every resulting file
  ≤ ~800 lines, and `cargo test -p sovereign-consultant-playground` passes
  with the same test count as before the split.

- [ ] **P1 | `crates/contracts/` | Add serialization-shape tests for the signed contract types.**
  The crate has zero tests, yet `CapabilityTokenBody` is the canonical signed
  body whose serde shape is load-bearing for signature verification in
  `crates/capability` — today a field rename would silently invalidate
  tokens. Done when: a serde round-trip test plus a golden-JSON test pin the
  field names of `CapabilityTokenBody` and `PolicyDecision`, and renaming any
  field makes `cargo test -p sovereign-contracts` fail.

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

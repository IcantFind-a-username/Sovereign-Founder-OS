# RFC 0007 Amendment 1 implementation — Developer Preview

- **Date:** 2026-09-16
- **Base:** `main` at `95a1d51` (includes #166 / #167)
- **Authoritative spec:** `rfcs/0007-audit-ledger-freshness-anchor.md`
  (Amendment 1 Accepted 2026-09-16). RFC 0007 overall remains **Draft**.
- **Maturity:** Developer Preview mechanism. Not a product security gate.

## What landed

Enrolled `freshness_generation` as a **separate** record (`freshness.enrolled`
plus an `enrollment.claimed` marker). It is not implied by `ledger.json` or
`ledger.head`. The v2 `ledger.head` sidecar adds signed
`freshness_generation`; `AuditEventBody` is unchanged.

Open-time rules (after the v0.1 anchor check):

- generation downgrade is rejected even when signatures verify
- a paired old ledger + old anchor is rejected when a higher enrolled
  generation survives outside the restored tree
- missing enrolled state after a claim enters **limited recovery**
  (`WorkspaceError::LimitedRecovery`), not a normal open
- limited recovery bumps strictly above any restored artifact and does not
  reuse pre-restore grants

Authority coupling (RFC 0003): `AuthorityStore` binds an execute-path
generation. Advancing it makes prior consume-bundle markers invalid for new
execute paths. `authorize_new_execute` takes only the enrolled number — a
`verify_chain` pass is never consulted.

Workspace: default `Store::open` stays the v0.1 path (existing honesty tests
stay green). `Store::open_enrolled(root, enrollment_dir)` claims Amendment 1
protection. The enrollment directory must be **outside** the workspace tree
for paired-restore rejection to have teeth.

## Conformance tests

- `paired_old_ledger_and_old_anchor_rejected_when_enrolled_generation_survives`
- `a_generation_downgrade_is_rejected_even_with_valid_signatures`
- `missing_enrolled_generation_after_enrollment_enters_limited_recovery`
- `authority_consume_state_not_assumed_fresh_from_audit_chain_alone`
- `a_paired_workspace_tree_restore_is_refused_when_generation_survives_outside_tree`

All existing v0.1 freshness tests remain required and green.

## Honest residuals (not claimed)

- **Not RP1-06 product qualification.** Implementation ≠ product acceptance
  evidence. Runtime Phase 1 is not complete.
- **Not whole-device rollback detection** (THREAT_MODEL T10 Research). An actor
  who restores device key + enrolled record + ledger + anchor + authority
  store together is out of scope.
- **Co-located enrolled files are not independently protected** until RFC
  0005 / Program 1C1 custody. `Store::open` does not silently claim
  generation protection; a default tree-only layout can still rewrite
  enrollment with the rest of the directory.
- **Not** Wave D, 1C0 admission, product ActiveV2, Program 1B0, or product
  Exact Effect.
- Audit-chain verify alone still must not be treated as execute authority
  after restore — the new gate exists so that mistake fails closed.

**Explicit separation:** landing Amendment 1 code ≠ RP1-06 product pass.

## CI note (2026-09-16, after #168)

Updating onto `main` at `2dc2659` was docs-only (`docs/backlog.md` + the 1B0
evidence report). Amendment 1 named tests stayed green. The `test` job
failed on existing `workspace::tests::double_decision_and_unknown_ids_fail_closed`:
`decide` of an unknown id could surface `Invalid("another workspace writer
is already active")` instead of `NotFound` when the writer lock was taken
for a fail-closed lookup. Fail-closed reads now run before the lock; this
is not RP1-06.

# Runtime Phase 1 — step 3 fixture evidence / recovery / freshness

- **Guide:** [Runtime Phase 1 development guide §4 step 3](../../security/runtime-phase-1-development-guide.zh-CN.md#sequence)
- **Plan Tasks:** 12–15 in [owner-session / exact-effect v1 plan](../../superpowers/plans/2026-08-14-owner-session-exact-effect-v1-implementation.md)
- **TSV source of truth:** `scripts/owner-effect-tests.tsv` column `task` 12–15
- **Contract:** [RFC 0006](../../../rfcs/0006-synthetic-owner-session-exact-effect-fixture.md); freshness [RFC 0007](../../../rfcs/0007-audit-ledger-freshness-anchor.md)
- **Audit baseline (main):** `0d7972cd334a83a32c78dd06e0acdd6be808bdb2` — #154 step-2 fixture exact-effect evidence
- **Evidence:** docs-only slice; mechanism code unchanged from baseline

## Verdict (honest)

**Fixture step 3 — OK for registered TSV Tasks 12, 13, and 15** (22 rows). **Task 14 has zero TSV rows**; `./scripts/run-owner-effect-tests.sh --task 14` fail-closed with `no rows selected`. That is the correct empty-run answer (lesson 8), not a product/fixture pass.

Nothing here is product RP1-04, RP1-05, or RP1-06.

**RFC 0007 Amendment 1 is proposed, not Accepted.** #150 merged the amendment *text*; RFC status stays `Draft` and the header still says Amendment 1 is not accepted. Implementation remains the blocked backlog item. Do not treat v0.1 `ledger.head` pins as RP1-06.

**Product crash / recovery / freshness pins already on main** (journal recover, SIGKILL, two-process race, open-time freshness, checkpoint double-burn, F04 filesystem barrier). Cited and re-run below; not re-implemented.

## v1 TSV vs v2 plan numbering

The [v2 fixture plan](../../superpowers/plans/2026-08-14-synthetic-owner-exact-local-outbox-v2-implementation.md) is blocked on RFC 0002 profile accept and only has Tasks 0–7. Do not treat v2 Task *N* as v1 Task *N*.

| TSV / v1 (this step) | v2 plan | Overlap |
| --- | --- | --- |
| 12 value-free evidence | no Task 12; diagram “value-free synthetic evidence”; v2 Task 6 reconcile | projection-only chain; v2 has no `effect_v1` crate split |
| 13 cross-port / ceremony | v2 Tasks 2–3 (single-process UV HTTP) | v1-13 TSV is `sovereign-owner` TCP guard, not a browser driver |
| 14 fixture/product/legacy lock | v2 Task 2 (release-excluded package) + Task 0 | v2 forbids touching `sovereign-cli`; v1-14 has **no TSV rows** |
| 15 broker kill/race | v2 Task 6 crash recovery | v2 **rejects** broker child / hidden mode; v1-15 is broker-startup barriers only |

## What was audited

| Area | Plan task | On main before this slice | Step-3 conclusion |
| --- | --- | --- | --- |
| Value-free synthetic effect evidence | 12 | `crates/audit-ledger` `effect_v1` + 10 TSV rows (`5f3bfb7`) | **Complete** for registered rows. No `crates/authority/tests/effect_evidence.rs`. No `crash_after_terminal_before_append_heals_evidence_only` |
| Cross-port / stolen-cookie attacks | 13 | `crates/owner/tests/cross_port_attacks.rs` + 7 TSV rows | **Complete** for the owner-crate TCP guard. No `apps/cli/assets/fixture/`, no `fixture_http`, no `fixture_ui_contract`, no `owner-auth-browser-test.sh`. No browser is driven (file comment: deliberate) |
| Fixture/product/legacy final audit | 14 | **0 TSV rows** | **Absent from the manifest.** Substitute already on main: `scripts/check-owner-effect-boundary.sh` (green). No `slice_isolation.rs`. `check-owner-effect-broker-build.sh` / `check-owner-effect-authority-plane.sh` still absent (step-1/2 P3s) |
| Broker real-process kill matrix | 15 | `apps/cli/tests/broker_kill_matrix.rs` + 5 TSV rows + `scripts/exact-effect-kill-matrix.sh` | **Complete** for registered startup barriers (address/hello/lock/redb). Not the plan’s full dispatch/reservation/evidence-append matrix; script has `--iterations` only (no `--race-iterations`) |
| Product journal recover | guide `[q-recovery]` | `f3c7d98` (#124) | **Cited, green** — not a fixture Task 12–15 row |
| Product SIGKILL soak | guide `[q-kill]` | `a_sigkilled_send_leaves_a_fail_closed_workspace_that_reopens_clean` | **Cited, green** — product send path, not broker kill matrix |
| Product two-process race | guide `[q-race]` | `two_processes_deciding_the_same_delivery_produce_exactly_one_effect` (`91dea89`, #129) | **Cited, green** |
| Product freshness open + honest whole-dir boundary | guide `[q-freshness-open]` `[q-anchor]` | `d301eda` (#127), crate tests `aa86929` (#126) | **Cited, green** — v0.1 conditional slice only |
| Checkpoint double-burn pin | guide `[q-checkpoint]` | `53333f6` (#125) | **Cited, green** — legacy fail-closed pin, **not** a license to auto-retry |
| Product F04 barrier | guide F04 | `revoke_during_bundle_commit_barrier_fails_closed` (`d8ffec9`) | **Cited, green** — filesystem bundle, not fixture redb |

Source test names match TSV 22/22 for tasks 12/13/15. Task 14 has nothing to match.

## Commands run (2026-09-16, Linux x86_64 cloud agent)

`libssl-dev` installed first. All succeeded (exit 0) unless noted.

```bash
# Baseline
git rev-parse HEAD
# → 0d7972cd334a83a32c78dd06e0acdd6be808bdb2

# Tasks 12–15 — checked manifest
./scripts/run-owner-effect-tests.sh --task 12
# → OK — 10 row(s)  (sovereign-audit-ledger / effect_v1)

./scripts/run-owner-effect-tests.sh --task 13
# → OK — 7 row(s)   (sovereign-owner / cross_port_attacks)

./scripts/run-owner-effect-tests.sh --task 14
# → FAIL (exit 1): no rows selected — a run that tests nothing is a failure
#    Honest empty-task result. Not a green.

./scripts/run-owner-effect-tests.sh --task 15
# → OK — 5 row(s)   (sovereign-cli / broker_kill_matrix)

# Task 12 / 14 / 15 GREEN substitutes that already exist
./scripts/check-owner-effect-canaries.sh
# → OK — 59 checks over 57 files
./scripts/check-owner-effect-boundary.sh
# → OK — 12 checks (fixture absent from default graph + release binary; fixture still builds)
./scripts/check-fault-injection-excluded.sh
# → OK — 4 checks

# Product pins (cite only — already on main)
cargo test -p sovereign-cli --lib --locked -- --exact \
  workspace::execution_journal_tests::indeterminate_execution_records_are_surfaced_on_open
cargo test -p sovereign-cli --lib --locked -- --exact \
  workspace::execution_journal_tests::recover_is_a_no_op_on_a_clean_journal
cargo test -p sovereign-cli --lib --locked -- --exact \
  workspace::rfc0007_open_tests::a_reverted_ledger_is_refused_at_open
cargo test -p sovereign-cli --lib --locked -- --exact \
  workspace::rfc0007_open_tests::a_whole_directory_rollback_is_not_detected_documented_boundary
cargo test -p sovereign-cli --lib --locked -- --exact \
  workspace::checkpoint_gap_pin_tests::a_kill_between_outbox_write_and_checkpoint_burns_fresh_authority_but_never_double_sends
cargo test -p sovereign-cli --lib --locked -- --exact \
  workspace::send_sigkill_tests::a_sigkilled_send_leaves_a_fail_closed_workspace_that_reopens_clean
cargo test -p sovereign-cli --lib --locked -- --exact \
  workspace::send_concurrency_tests::two_processes_deciding_the_same_delivery_produce_exactly_one_effect
# → each 1 passed

cargo test -p sovereign-audit-ledger --test rfc0007_freshness --locked
# → 6 passed (old-prefix / fork / missing-anchor / forward-extension / shape-unchanged)

cargo test -p sovereign-authority --test subprocess_claims \
  --no-default-features --features fault-injection --locked \
  revoke_during_bundle_commit_barrier_fails_closed -- --exact
# → 1 passed

# Repo scoped gate for this doc-only slice
./scripts/test_changed.sh
```

`test_changed.sh` printed `ALL GREEN` (exit 0). Docs-only change set; always-on `owner-effect-manifest` re-runs `./scripts/run-owner-effect-tests.sh --all` over the full 168-row TSV.

Kill-matrix soak `./scripts/exact-effect-kill-matrix.sh --iterations 25` was **not** re-run: the five TSV rows are the same tests, executed once. Plan `--race-iterations 100` is not implemented by the script.

## Manifest coverage (Tasks 12–15)

Whole TSV: **168** data rows. This step:

| Task | Registered rows | Result |
| --- | ---: | --- |
| 12 | 10 | OK |
| 13 | 7 | OK |
| 14 | 0 | **no rows — runner fail-closed** |
| 15 | 5 | OK |
| **12+13+15** | **22** | **OK** |

## Residual (material for later slices — not step-3 blockers)

1. **F09 / RFC 0007 Amendment 1 not Accepted** — #150 proposed the text. Maintainer acceptance-with-rationale still required before RP1-06 product implementation. v0.1 pins prove prefix-restore-against-a-*newer*-anchor and document that paired old-ledger+old-anchor restore is undetected. Already queued: blocked P2 “Implement RFC 0007 Amendment 1”.
2. **Task 14 is not in the TSV** — no `slice_isolation` rows, no final public-symbol manifest test. `check-owner-effect-boundary.sh` is the existing substitute. Do not invent TSV rows for tests that do not exist.
3. **Task 12 plan extras never registered** — `crash_after_terminal_before_append_heals_evidence_only`; `sovereign-authority --test effect_evidence`. TSV 10 rows cover the value-free projection only. Queued P3 `runtime-owner-task12-heal`.
4. **Task 13 is the owner-crate TCP guard, not a browser ceremony** — plan files `apps/cli/assets/fixture/**`, `fixture_ui_contract`, `owner-auth-browser-test.sh` are absent. Large; not queued as a small residual.
5. **Task 15 is broker-startup barriers, not dispatch crash windows** — no TSV rows for reservation/Dispatching/publish/evidence-append kills. Product SIGKILL/race/journal pins cover the *product send* path, a different dispatcher.
6. **Plan GREEN extras already queued elsewhere** — `check-owner-effect-broker-build.sh` (step-1 P3), `check-owner-effect-authority-plane.sh` (step-2 P3).
7. **Product RP1-04 / RP1-05 / RP1-06** — explicitly not claimed. Fixture evidence ≠ product qualification.

## Explicit non-goals (forbidden in this slice)

Product Exact Effect / invert #152; Wave D / RFC 0002 accept / ActiveV2 / 1B0 / product 1C0; claiming RP1-04, RP1-05, or RP1-06 product pass; implementing RFC 0007 Amendment 1; large new architecture.

## Next guide step

Proceed to **§4 step 4** (product owner/key custody, protected persistence, clean restore — RFC 0005 / Vault / Program 1B/1C/1D) only after the listed product gates. Fixture Task 16 qualification records remain a later independent fixture artifact, not a substitute for §4 step 5 product RP1 evidence.

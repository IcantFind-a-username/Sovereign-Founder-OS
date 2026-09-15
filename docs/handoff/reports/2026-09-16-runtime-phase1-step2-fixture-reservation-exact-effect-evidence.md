# Runtime Phase 1 — step 2 fixture reservation / exact-effect evidence

- **Guide:** [Runtime Phase 1 development guide §4 step 2](../../security/runtime-phase-1-development-guide.zh-CN.md#sequence)
- **Plan Tasks:** 7–11 in [owner-session / exact-effect v1 plan](../../superpowers/plans/2026-08-14-owner-session-exact-effect-v1-implementation.md)
- **TSV source of truth:** `scripts/owner-effect-tests.tsv` columns `task` 7–11
- **Contract:** [RFC 0006](../../../rfcs/0006-synthetic-owner-session-exact-effect-fixture.md)
- **Audit baseline (main):** `4707835901e4fdf175213d4b1454752be9992cef` — #152 F03 honesty pin + #153 step-1 evidence
- **Evidence:** docs-only slice; mechanism code unchanged from baseline

## Verdict (honest)

**Fixture step 2 — OK for registered TSV Tasks 7–11.** Fifty manifest rows (root lifecycle, sealed compose, origin HTTP guard, redb reservation, corpus + dispatch) are present and green on this host. Nothing here is product Exact Effect, RP1-02, or RP1-03.

**Product path is still not Exact Effect.** [#152 pin](2026-09-15-f03-rp1-02-exact-effect-honesty-pin.md) `rp1_02_product_approval_does_not_bind_final_recipient_or_exact_eml_bytes` still passes (tampered `To:` still succeeds). Invert remains [runtime-f03-exact-effect-product](../../backlog.md#runtime-f03-exact-effect-product) and is blocked.

**F04 revoke barrier is on main**, on the *legacy filesystem* bundle (`revoke_during_bundle_commit_barrier_fails_closed`), not on the fixture redb reservation/dispatch pair.

## v1 TSV vs v2 plan numbering

The [v2 fixture plan](../../superpowers/plans/2026-08-14-synthetic-owner-exact-local-outbox-v2-implementation.md) uses a different Task index (blocked on RFC 0002 profile accept). Do not treat v2 Task *N* as v1 Task *N*.

| TSV / v1 (this step) | v2 plan | Overlap |
| --- | --- | --- |
| 7 fixture root | v2 Task 7 is **qualification/handoff**, not a root type | `SyntheticFixtureRootV1` is v1-7 only |
| 8 sealed payload | v2 Task 4 (seal intent + UV bridge) | compose/privacy only |
| 9 exact-origin HTTP | v2 Tasks 2–3 (single-process + UV HTTP) | v1-9 TSV is `sovereign-owner` `http_guard`, not `apps/cli/src/fixture_http/` |
| 10 atomic reservation | v2 Task 5 (one redb transaction) | redb `reserve()`; v2 still forbids capability→authority inversion in *its* Task 1 |
| 11 sealed writer | v2 Task 6 (publish once) | `dispatch()` temp+rename; not an `AuthorityReservedEffect` handle |

## What was audited

| Area | Plan task | On main before this slice | Step-2 conclusion |
| --- | --- | --- | --- |
| Synthetic fixture root (no product variant) | 7 | `broker/fixture_root.rs` + 8 TSV rows | **Complete** for registered rows. No `forest.rs`; no trybuild `no_raw_node_writer` |
| Compose + private payload | 8 | `broker/exact_fixture.rs` + 6 TSV rows | **Complete** for registered rows. `sealed_bytes()` is `pub(crate)`; no consumer trybuild |
| Exact-origin HTTP guard | 9 | `sovereign-owner` `http_guard` + 14 TSV rows | **Complete** for the owner-crate guard. No `apps/cli/src/fixture_http/`, no `slice_isolation`, no `security-fixture` command |
| Atomic reservation | 10 | `broker/reservation.rs` + 7 TSV rows | **Complete** for redb all-or-nothing id claims. Capability still depends on authority (`with_authority_store` remains). No `check-owner-effect-authority-plane.sh`. Reservation does not recheck durable revocation |
| Corpus + sealed dispatch | 11 | `broker/corpus.rs` + `broker/dispatch.rs` + 15 TSV rows | **Complete** for registered rows. Writer consumes crate-private bytes. Caller still passes `&ProtectedFixturePayload` (not a one-use reservation handle). No `local_outbox.rs`, sandbox fixture, or effects opaque façade |
| Product F03 / RP1-02 | guide F03 | honesty pin on main (#152) | **Still pinned** — not inverted |
| Product F04 / RP1-03 | guide F04 | filesystem bundle barrier on main | **Fixture reservation ≠ that barrier** |

Source test names match TSV 50/50 (no missing compile-fail *row*; those targets were never registered).

## Commands run (2026-09-16, Linux x86_64 cloud agent)

All succeeded (exit 0) unless noted.

```bash
# Baseline
git rev-parse HEAD
# → 4707835901e4fdf175213d4b1454752be9992cef

# Tasks 7–11 — checked manifest
./scripts/run-owner-effect-tests.sh --task 7
# → OK — 8 row(s)  (sovereign-authority / fixture_root_lifecycle)

./scripts/run-owner-effect-tests.sh --task 8
# → OK — 6 row(s)  (sovereign-authority / exact_fixture_prepare)

./scripts/run-owner-effect-tests.sh --task 9
# → OK — 14 row(s) (sovereign-owner / fixture_loopback)

./scripts/run-owner-effect-tests.sh --task 10
# → OK — 7 row(s)  (sovereign-authority / reservation_atomicity)

./scripts/run-owner-effect-tests.sh --task 11
# → OK — 15 row(s) (sovereign-authority / broker_corpus + exact_dispatch)

# Task 11 canary domain gate
./scripts/check-owner-effect-canaries.sh
# → OK — 59 checks over 57 files

# Product Exact Effect is still absent (#152 pin)
cargo test -p sovereign-cli --lib --locked \
  rp1_02_product_approval_does_not_bind_final_recipient_or_exact_eml_bytes
# → 1 passed

# F04 filesystem revoke-vs-commit barrier (not fixture redb)
cargo test -p sovereign-authority --test subprocess_claims \
  --no-default-features --features fault-injection --locked \
  revoke_during_bundle_commit_barrier_fails_closed -- --exact
# → 1 passed

# Repo scoped gate for this doc-only slice
./scripts/test_changed.sh
```

`libssl-dev` was installed first. Unlike step 1, `test_changed.sh` **did** run `cargo test --manifest-path fixtures/owner-webauthn/Cargo.toml --locked` (adapter workspace only — not real-authenticator qualification). `test_changed.sh` printed `ALL GREEN` (exit 0; no cargo test scope in this docs-only change set). The always-on `owner-effect-manifest` step also re-ran `./scripts/run-owner-effect-tests.sh --all` over the full 168-row TSV.

## Manifest coverage (Tasks 7–11)

Whole TSV: **168** data rows. This step:

| Task | Registered rows | Result |
| --- | ---: | --- |
| 7 | 8 | OK |
| 8 | 6 | OK |
| 9 | 14 | OK |
| 10 | 7 | OK |
| 11 | 15 | OK |
| **7–11** | **50** | **OK** |

## Residual (material for later slices — not step-2 blockers)

1. **Plan GREEN extras never registered in TSV** — trybuild `compile_fail` / `protected_payload_boundary` (Tasks 7–8, 11), CLI `slice_isolation`, `scripts/check-owner-effect-authority-plane.sh`, `scripts/check-owner-effect-broker-build.sh` (already a step-1 P3). Privacy today is structural (`pub(crate)` bytes + Debug redaction), not a consumer compile-fail golden.
2. **Task 9 surface is the owner crate, not the CLI** — v1 plan files `apps/cli/src/fixture_http/**` and `security-fixture --synthetic-only` are absent. TSV Task 9 is `crates/owner/tests/fixture_loopback.rs` against `http_guard`.
3. **Task 10 is not Capability-pure / graph-inverted** — `sovereign-capability` still depends on `sovereign-authority`; `with_authority_store` remains. Fixture `ReservationRequest` is public. No failpoint TSV rows (`owner-effect-fixture,fault-injection`) for reservation.
4. **Revoke adjudication is not on the fixture reserve→dispatch path** — F04 proves the filesystem bundle commit recheck. Fixture `reserve()` does not consult `revoked-*` records; `dispatch()` does not require a reservation receipt.
5. **Plan checkbox hygiene** — Tasks 7–11 bullets in the 16-task plan file remain `[ ]` even though TSV rows landed.
6. **Product RP1-02 / RP1-03** — explicitly not claimed. Fixture progress does not invert #152 or close F03/F04 as product gates.

## Explicit non-goals (forbidden in this slice)

Product Exact Effect / invert #152; Wave D / RFC 0002 acceptance / ActiveV2 / 1B0 / product 1C0; claiming RP1-02 or RP1-03 product pass; RFC 0007 Amendment 1 implementation; large new architecture.

## Next guide step

Proceed to **§4 step 3** (evidence verify, browser attacks, product/fixture split, recovery, freshness, crash windows — plan Tasks 12–15) with the same fixture/product separation. Product integration still requires step 0 gates plus Vault / 1C0 / 1D prerequisites.

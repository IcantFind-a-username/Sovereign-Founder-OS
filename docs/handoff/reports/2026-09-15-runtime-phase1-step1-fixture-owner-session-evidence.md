# Runtime Phase 1 — step 1 fixture owner-session evidence

- **Guide:** [Runtime Phase 1 development guide §4 step 1](../../security/runtime-phase-1-development-guide.zh-CN.md#sequence)
- **Plan Tasks:** 3–6 in [owner-session / exact-effect v1 plan](../../superpowers/plans/2026-08-14-owner-session-exact-effect-v1-implementation.md)
- **Contract:** [RFC 0006](../../../rfcs/0006-synthetic-owner-session-exact-effect-fixture.md)
- **Audit baseline (main):** `78c7a7a31a282aa5c8fe6b683e5e38442cfd151e` — merges step-0 product 1C0 owner-admission freeze (#151)
- **Evidence commit:** `88aee6e` on branch `cursor/runtime-phase1-step1-fixture-evidence-d947`

## Verdict (honest)

**Fixture step 1 — OK for Tasks 3–6.** Registered manifest rows, boundary gates, and the Linux x86_64 migration suite are green on this host. Nothing here establishes **product** owner admission, RP1-01 pass, or real authenticator qualification.

**Product admission remains frozen** by [step-0 design card](../../security/runtime-phase-1-product-1c0-owner-admission-freeze.zh-CN.md): pins such as `an_unauthenticated_local_post_can_approve_today_1c0_pin` and `product_decide_mints_vault_keys_without_owner_admission_1c0_pin` still document today's product posture.

## What was audited

| Area | Plan task | On main before this slice | Step-1 conclusion |
| --- | --- | --- | --- |
| Legacy claim subprocess + release-excluded fault barriers | 3 | Landed; 5 manifest rows + F04 barrier test | **Complete** for fixture scope |
| Single-writer broker, redb, IPC, root classification | 4 | Landed (#59–#66); 48 broker manifest rows + 3 migration manifest rows | **Complete** for mechanism; formal `check-owner-effect-broker-build.sh` still absent (see residuals) |
| `platform_publish` / Linux migration gate | 4 | `crates/authority/src/broker/platform_publish.rs`, `tests/migration.rs` (6 tests) | **Complete** on `x86_64-unknown-linux-gnu` |
| Unqualified WebAuthn fixture session + logout | 5 | `sovereign-owner` + 26 manifest rows; adapter in `fixtures/owner-webauthn` workspace | **Complete** for synthetic fixture; real matrix still empty |
| Fresh UV → opaque RFC 0003 approval bridge | 6 | 12 `exact_approval` manifest rows | **Complete** for fixture |
| Product HTTP honesty pins (not fixture loopback) | guide `[q-http]` | `apps/cli/tests/ui_http_boundary.rs` | **Pinned** — product gaps unchanged |

## Commands run (2026-09-15, Linux x86_64 cloud agent)

All succeeded unless noted.

```bash
# Baseline
git rev-parse HEAD
# → 78c7a7a31a282aa5c8fe6b683e5e38442cfd151e

# Task 3 — manifest + repetition gate
./scripts/run-owner-effect-tests.sh --task 3
./scripts/run-authority-subprocess-claims.sh --iterations 1
./scripts/check-fault-injection-excluded.sh

# Task 4 — broker + migration (manifest)
./scripts/run-owner-effect-tests.sh --task 4
cargo test -p sovereign-authority --test migration \
  --no-default-features --features owner-effect-fixture,fault-injection --locked -- --test-threads=1

# Task 5 — owner session/registry/boundary
./scripts/run-owner-effect-tests.sh --task 5

# Task 6 — exact approval bridge
./scripts/run-owner-effect-tests.sh --task 6

# Cross-package regression (guide §7)
./scripts/run-owner-effect-regression.sh -- cargo test \
  -p sovereign-authority -p sovereign-owner \
  --no-default-features \
  --features sovereign-authority/owner-effect-fixture,sovereign-authority/fault-injection,sovereign-owner/owner-effect-fixture \
  --locked
# → 188 test(s) executed

# Boundary gates (also in owner-effect-fixture CI)
./scripts/check-owner-effect-crypto-profile.sh

# Product HTTP pins (1C0 honesty — not fixture admission)
cargo test -p sovereign-cli --test ui_http_boundary --locked

# Repo scoped gate for this doc-only slice
./scripts/test_changed.sh
```

**WebAuthn adapter workspace:** `cargo test --manifest-path fixtures/owner-webauthn/Cargo.toml --locked` is the authoritative gate (`.github/workflows/owner-effect-fixture.yml`). It was **not** re-run here: this environment lacks the OpenSSL dev packages the separate workspace expects. Core `--workspace` green does not substitute.

**Subprocess soak:** CI runs `./scripts/run-authority-subprocess-claims.sh --iterations 5`; this evidence used `--iterations 1` for wall-clock. Full 25-iteration soak remains the plan's Task 3 GREEN block for deliberate soak, not a step-1 blocker.

## Manifest coverage (Tasks 3–6)

| Task | Registered rows (`scripts/owner-effect-tests.tsv`) | Result |
| --- | ---: | --- |
| 3 | 5 | OK |
| 4 | 51 | OK |
| 5 | 26 | OK |
| 6 | 12 | OK |

Three additional migration tests (`a_qualified_platform_publishes_exactly_one_complete_generation`, `migrating_onto_an_existing_generation_is_refused`, `migrating_from_a_generation_that_is_not_there_is_refused`) are **`cfg(linux + x86_64)` only** and intentionally omitted from the global manifest so macOS `--all` runs do not false-fail; they pass under the direct `migration` test binary on Linux (see command above). On non-Linux targets, `an_unqualified_platform_refuses_and_changes_nothing` is the complementary assertion.

## Residual (material for later slices — not step-1 blockers)

1. **`scripts/check-owner-effect-broker-build.sh`** — still listed in the owner plan Task 4 GREEN block but not in the tree. Substitutes today: `scripts/check-owner-effect-profile-builds.sh`, `apps/cli/tests/broker_bootstrap.rs`, and the [owner-effect-fixture CI workflow](../../../.github/workflows/owner-effect-fixture.yml). A dedicated same-target-dir build gate remains optional hardening, not missing mechanism.
2. **Real authenticator qualification** — `docs/security/owner-auth-mechanism-matrix.md` allows an empty real matrix; virtual preflight can run, but mechanism_qualified_only rows are still absent by design.
3. **Plan checkbox hygiene** — Tasks 3–6 bullets in the 16-task plan file remain `[ ]` even though code and manifest rows landed; reconcile in a docs-only pass if desired (out of scope for product claims).
4. **Product 1C0 / RP1-01** — explicitly deferred to step 0 freeze and future Program 1C work; fixture credential path must not be read as admission.

## Explicit non-goals (forbidden in this slice)

Wave D product code, ActiveV2, 1B0, removing app-local product signer, claiming RP1-01 product pass, or conflating fixture enrolment with founder admission.

## Next guide step

Proceed to **§4 step 2** (atomic reservation, exact payload, revoke arbitration, closed writer — plan Tasks 7–11) only with the same fixture/product separation; product integration still requires step 0 gates plus Vault/1C0/1D prerequisites.

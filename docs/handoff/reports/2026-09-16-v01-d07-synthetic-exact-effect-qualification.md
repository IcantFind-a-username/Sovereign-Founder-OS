# v01-D07 — Synthetic exact-effect fixture qualification (limitations / evidence)

- **Outcome:** fixture qualification only. Not a product status change.
- **Date:** 2026-09-16
- **Ticket:** v01-D07 (Wave D queue; v2 plan Task 7)
- **Base:** `main` @ `3a96f90` (merged #176 / v01-D06)
- **Harness commit (soak ran here):** `5332a09`
- **Maturity:** Design Accept ≠ product Current.

This note records what the v2 upper fixture (`sovereign-synthetic-owner-effect`,
`publish = false`) demonstrated under soak, and what it still is not. It
**cannot change product status**. Editing this file cannot enable 1C0, Exact
Effect, ActiveV2, Vault, email, or E2EE.

## What was qualified

A **synthetic, unqualified, plaintext fixture**. One process, one public
listener, one retained OS lock before the sole redb open. Fresh UV yields a
one-use exact-bound grant; session presence is not approval. Reservation is
one redb transaction. Publication of a local `.eml` is at most once, with
conservative crash recovery and value-free evidence.

This is not owner admission. Empty-registry enrolment is first-writer-wins
and cannot tell the founder from another same-account native process.

## Soak evidence

Platform: Linux `6.12.94+` x86_64 (`Linux cursor`).
Toolchain: rustc 1.97.0 (`2d8144b78` 2026-07-07), host
`x86_64-unknown-linux-gnu`.
Feature set: `--no-default-features --features owner-effect-fixture --locked`.

Command (no zero/skipped tests accepted):

```bash
./scripts/run-synthetic-owner-effect-soak.sh --crash-iterations 25 --race-iterations 100
```

Result: `run-synthetic-owner-effect-soak: OK — crash 25, race 100 (owner-effect-fixture)`
on commit `5332a09`. Every named case printed `ok` on every iteration.

### Crash boundaries — 25 real process-kill/reopen iterations each

| Target | Named test | Iterations | Result |
| --- | --- | --- | --- |
| `reserve_kill` | `real_process_kill_before_commit_leaves_none_of_the_reservation` | 25 | ok |
| `reserve_kill` | `real_process_kill_after_commit_leaves_all_of_the_reservation` | 25 | ok |
| `publish_kill` | `real_process_kill_before_dispatching_commit_stays_pre_dispatch` | 25 | ok |
| `publish_kill` | `real_process_kill_after_dispatching_before_write_is_indeterminate` | 25 | ok |
| `publish_kill` | `real_process_kill_after_publication_reopens_succeeded` | 25 | ok |

### Same-process races — 100 iterations each

| Target | Named test | Iterations | Result |
| --- | --- | --- | --- |
| `reserve_races` | `same_process_concurrent_http_or_thread_reservation_has_one_winner` | 100 | ok |
| `reserve_races` | `logout_race_refuses_and_commits_nothing` | 100 | ok |
| `publish_races` | `same_process_concurrent_dispatch_reconcile_has_one_terminal_winner` | 100 | ok |

CI on this branch runs a sample (`--crash-iterations 3 --race-iterations 5`)
so pull requests do not wait for the full qualification soak. The numbers
above are the recorded qualification run.

## Mechanism matrix

Virtual (`protocol_fixture_only`) rows in
[`docs/security/owner-auth-mechanism-matrix.md`](../../security/owner-auth-mechanism-matrix.md):

| os / arch | browser | date | command |
| --- | --- | --- | --- |
| darwin 25.5.0 / arm64 | Chrome/152.0.7977.83 | 2026-09-10 | prior attended-virtual characterisation |
| linux 6.12.94+ / x86_64 | Chrome/148.0.7778.96 | 2026-09-16 | `CHROME_PATH=/usr/bin/google-chrome-stable ./scripts/owner-auth-origin-preflight.sh --virtual` → `PASS` |

Both virtual rows observed the same protocol facts: loopback cookies are not
port-isolated; a hostile port obtained a user-verified assertion over the
shared RP ID; the origin inside `clientDataJSON` is the load-bearing check;
same-user-handle creation replaced the credential (1 remaining).

**Attended real matrix (`mechanism_qualified_only`): empty.** No attended run
against a real authenticator was performed. The mechanism is unqualified on
every platform. An empty real matrix is allowed and is the current state.
Virtual rows are protocol characterisation only; they are not owner admission
and they do not qualify any real authenticator.

## What remains Target

- **Full cross-process validator race** — Target and unqualified. A second
  live fixture process is tested only for OS-lock denial before redb open
  (v01-D02). Concurrent cryptographic validation across two processes was
  not soaked and is not claimed.
- Unsupported / uncharacterised browser and OS combinations stay Target.
  Failed or unrun attended real-authenticator rows are excluded, not
  recorded as qualified.

## What this note does not claim

- Not email, SMTP, or a network send.
- Not owner-continuity or owner admission.
- Not product 1C0 or 1C1.
- Not Vault, `PendingV2`, or `ActiveV2`.
- Not E2EE, Mesh, or multi-device.
- Not product Exact Effect, Program 2, Phase D Current, or RFC 0004 product
  boundary complete.
- Not product issuance of `low_risk_effectful`.
- Not WebAuthn on `/api/workspace/decide`.
- Not a `v0.1` product tag, ROADMAP “1C0 Current”, or Wave E.
- Not a finish of the v1 HMAC hidden broker.

Redb in this fixture is ACID and crash-safe. It is not encrypted and not
authenticated. Synthetic plaintext persistence generalises to nothing about
founder data.

RFC 0006 G2 remains conjunctive: Program 1B1, Program 1C1, Program 1D
`ActiveV2`, and a protected-payload review must all complete before product
use. None of those is addressed here.

## Fixture-specific manifests run with this ticket

Prefer the owner-effect fixture feature set. Named rows for D07 live under
task 21 in `scripts/owner-effect-tests.tsv` (`publish_kill`, `publish_races`).
Compile-fail, canary, exact-test, and crash/race binaries already registered
for D02–D06 remain in that manifest.

## Final gate

Commands from the v2 plan Task 7 block, run on this host after the soak
(`5332a09`) and the documentation close on this branch:

| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | green |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | green (`Finished` 1m 10s) |
| `cargo test --workspace --locked` | green |
| `./scripts/check-file-size.sh` | OK — 289 files |
| `./scripts/check-synthetic-owner-effect-boundary.sh` | OK — 25 checks |
| `cargo tree -p sovereign-cli --locked` | no `sovereign-synthetic-owner-effect` / `redb` / `webauthn` |
| `cargo build -p sovereign-cli --release --locked` | green |
| `git diff --check` | green |

Fixture-specific manifests on the same host:

| Command | Result |
| --- | --- |
| `./scripts/run-synthetic-owner-effect-soak.sh --crash-iterations 25 --race-iterations 100` | OK — crash 25, race 100 |
| `CHROME_PATH=/usr/bin/google-chrome-stable ./scripts/owner-auth-origin-preflight.sh --virtual` | PASS (`protocol_fixture_only`, Chrome/148.0.7778.96) |
| `./scripts/owner-auth-origin-preflight.sh --real` | not run (attended); real matrix empty |
| `./scripts/check-owner-effect-canaries.sh` | OK — 131 checks / 129 files |
| `./scripts/run-owner-effect-tests.sh --all` | OK — 247 rows |
| `cargo test -p sovereign-synthetic-owner-effect --no-default-features --features owner-effect-fixture --locked` | green (compile-fail, crash, race, exact tests included) |

`./scripts/test_changed.sh` **ALL GREEN** on this close (workspace clippy/tests,
owner-effect manifests including the soak self-test, file-size, fmt, frontend
tsc, vault-v2 qualification).

## Explicit non-promotion

Wave D tickets v01-D01…v01-D07 are a **fixture qualification** of a
synthetic unqualified plaintext path. They are not product Exact Effect,
not Phase D Current, and not a reason to retag ROADMAP or RFC status.

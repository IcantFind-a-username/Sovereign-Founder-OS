# v01-D05 — Reserve every authority fact in one redb transaction

- **Outcome:** landed
- **Branch:** `cursor/v01-d05-reserve-authority-atomically-1a98` · **Base:** `13a150f` (#174)
- **Backlog entry:** v01-D05 — Reserve every authority fact in one redb transaction — checked off: yes

## What changed

- `crates/synthetic-owner-effect/src/reserve.rs` — sole coordinator write helper `reserve_exact_authority` consumes opaque `VerifiedCapabilityV2` / `VerifiedApprovalV1` by value. One `OwnedStore.write` (Immediate + two-phase) rechecks bindings against durable state, then commits approval claim (signed approval expiry already Current — no second retention path), token claim, idempotency binding to the random `effect_intent_id`, synthetic authority-node decrement, and `Prepared -> AuthorityReserved`.
- `crates/synthetic-owner-effect/src/reserved.rs` — privately constructible `AuthorityReservedEffect`. No Clone/Serialize/Debug, no public constructor or reconstruction accessor.
- Public API is opaque proofs plus live `ReservationContext`. Not `ReservationRequest` raw UUIDs. Never calls `authorize_and_consume*`. Never duplicates COSE/trust/policy crypto.
- Failpoints between every logical mutation; real-process kill barriers immediately before and after commit.

## Tests

- `coordinator_never_calls_the_legacy_consuming_validator`
- `a_first_reservation_commits_every_fact_together`
- `failpoint_between_every_logical_mutation_rolls_all_of_them_back`
- `same_key_idempotent_replay_is_refused`
- `different_intent_conflicts_on_the_same_idempotency_key`
- `reopen_sees_either_all_or_none_of_the_reservation`
- `same_process_concurrent_http_or_thread_reservation_has_one_winner`
- `expiry_race_refuses_and_commits_nothing`
- `logout_race_refuses_and_commits_nothing`
- `signer_epoch_race_refuses_and_commits_nothing`
- `revocation_race_refuses_and_commits_nothing`
- `real_process_kill_before_commit_leaves_none_of_the_reservation`
- `real_process_kill_after_commit_leaves_all_of_the_reservation`
- `compile_fail_construct_clone_serialize_debug_destructure_reconstruct`
- `coordinator_source_has_no_raw_id_public_reservation_request`

## Gate

`./scripts/test_changed.sh` ALL GREEN — workspace + frontend tsc + vault-v2 qualification as applicable. `cargo tree -p sovereign-cli --locked` still excludes fixture/redb/WebAuthn. **Design Accept ≠ product Current.**

## Out of scope (untouched)

Publish/write `.eml` (D06), product AuthorityStore bundle rewrite, product `risk_class` admission / Exact Effect / 1C0 / ActiveV2 / RP1, HMAC / hidden broker / second port / v1 BrokerReady, product CLI decide path / apps/cli UI, adding the fixture to the sovereign-cli release graph. Full cross-process validator race remains Target.

## Suggested next item

v01-D06 — Publish once and reconcile conservatively.

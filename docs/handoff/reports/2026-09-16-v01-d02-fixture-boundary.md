# v01-D02 — Release-excluded single-process fixture boundary

- **Outcome:** landed
- **Branch:** `cursor/v01-d02-fixture-boundary-4223` · **Base:** `f04b4af` · **Head:** (see git)
- **Backlog entry:** v01-D02 — Release-excluded single-process fixture boundary — checked off: yes

## What changed

- `crates/synthetic-owner-effect/` — new `publish = false` upper fixture package depending on capability + authority; empty default; `owner-effect-fixture` forwards authority's same feature.
- `src/boundary.rs` — classify via existing `fixture_root`/`bootstrap::classify`, retain `process_lock`, sole `OwnedStore::open`.
- `src/listener.rs` — one public loopback bind entry and compiled origin constants; not called from acquire/open.
- `src/main.rs` — fixture process (`--root`, `--hold`); not a `sovereign-cli` hidden mode.
- `scripts/check-synthetic-owner-effect-boundary.sh` — product tree/symbols, metadata seam, source inventory, fixture still builds.
- Gate wiring: workspace member, `test_changed.sh`, profile-builds, owner-effect boundary/canaries, fixture CI, TSV task 16.

## Tests

- `lock_acquisition_precedes_the_only_production_redb_open` — lock file exists before `authority.redb`.
- `second_real_fixture_process_fails_on_os_lock_before_redb_open` — second real process gets `E-BROKER-ALREADY-RUNNING`.
- `product_unmarked_or_symlink_roots_fail_before_listener_or_database_state` — no lock/redb on rejected roots; symlink does not lock the target.
- `source_inventory_has_one_listener_and_no_internal_transport_module` — one `TcpListener::bind`; no HMAC/hidden-child/second-port residue.

## Out of scope (untouched)

HMAC supervisor, hidden `__owner-effect-broker`, second port, `apps/cli/**` product UI, WebAuthn, owner sessions, effect publication, `run_owner_effect_fixture_broker` / BrokerReady, product Exact Effect / 1C0 / ActiveV2 / RP1.

## Suggested next item

v01-D03 — Unqualified WebAuthn UV, session, CSRF, signer epoch (depends on this boundary).

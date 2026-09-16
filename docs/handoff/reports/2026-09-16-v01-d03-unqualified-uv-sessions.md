# v01-D03 — Unqualified WebAuthn UV, session, CSRF, signer epoch

- **Outcome:** landed
- **Branch:** `cursor/unqualified-owner-uv-sessions-b302` · **Base:** `0f576bd` (#171) · **PR:** #172
- **Backlog entry:** v01-D03 — Unqualified WebAuthn UV, session, CSRF, signer epoch — checked off: yes

## What changed

- `crates/synthetic-owner-effect/` — owner UV surface on the v2 fixture process. Reuses `crates/owner` session/registry as a library; empty default / fail-closed product admission unchanged.
- Closed v2 routes: `GET /`, `POST /api/fixture/auth/register/{start,finish}`, `POST /api/fixture/auth/login/{start,finish}`, `POST /api/fixture/auth/logout`. Effect routes remain unknown (D04).
- After the retained OS lock and before redb: ephemeral `TypedSigner<ApprovalRole>`, random signer epoch, closed `ApprovalBridge` (no signer getter, secret export, generic public sign, `Clone`/`Serialize`, or `approve_invocation`).
- Persist only the labelled `unqualified_fixture` public trust record (`historical_verify_only`). Secret key stays in process memory.
- Mechanism matrix: existing `docs/security/owner-auth-mechanism-matrix.md` virtual row; empty real matrix allowed and asserted.
- `crates/owner` — `Registry::abort_pending` so logout burns in-flight ceremonies without a credential reset path.

## Tests

- `first_writer_wins_is_non_admission`
- `registration_and_login_require_user_verification`
- `exact_origin_and_rp_are_compiled`
- `ceremonies_are_one_use_and_expire_at_300s`
- `cookie_and_csrf_are_independent`
- `absolute_and_idle_expiry_are_enforced`
- `cross_port_cookie_and_origin_residuals_are_refused`
- `logout_and_restart_invalidate_sessions`
- `no_secret_bytes_in_redb_or_debug_log`
- `historical_keys_are_verify_only`

Plus lock/signer/redb order, epoch rotation, zeroize graph, closed bridge, allowlist / no-reset, empty real matrix, closed v2 routes.

## Gate

`./scripts/test_changed.sh` ALL GREEN — workspace + frontend tsc + vault-v2 qualification. Fixture tests: 23 named D03 cases plus D02 boundary, under `--features owner-effect-fixture`. **Design Accept ≠ product Current.**

## Out of scope (untouched)

`/api/workspace/decide`, product WebAuthn, second credential, recovery, `ApprovalBridge::approve_invocation` / RFC 0003 evidence (D04), effect publication, HMAC / hidden broker / second port, product `low_risk_effectful` issuance, Exact Effect / 1C0 / ActiveV2 / RP1 claims.

## Suggested next item

v01-D04 — Seal intent and bridge fresh UV to RFC 0003.

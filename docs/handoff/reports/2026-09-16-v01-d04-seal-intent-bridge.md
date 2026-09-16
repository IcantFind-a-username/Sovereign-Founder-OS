# v01-D04 — Seal intent and bridge fresh UV to RFC 0003

- **Outcome:** landed
- **Branch:** `cursor/v01-d04-seal-intent-bridge-uv-e73e` · **Base:** `b8cadc6` (#172) · **PR:** #174
- **Backlog entry:** v01-D04 — Seal intent and bridge fresh UV to RFC 0003 — checked off: yes

## What changed

- `crates/synthetic-owner-effect/` — random `EffectIntentId` before composing the one fixed CRLF RFC 5322 payload; private `SealedPayload`; synthetic preview; in-process `EffectCoordinator` prepare/preview/fresh-UV grant.
- `FreshUvGrant` is non-cloneable, has no public constructor, and is not `Debug`/`Serialize`. Session presence is not approval.
- `ApprovalBridge::approve_invocation` is the only public signing entry that emits RFC 0003 `SignedApprovalV1`. Historical public trust remains `historical_verify_only`.
- Closed v2 effect routes: `POST /api/fixture/effect/{prepare,preview,approve/start,approve/finish}`. Dispatch stays unknown.
- `crates/artifact/` — fixture-only `verify_closed_fixture_profile`; product `verify()` stays PureCompute and still denies the closed profile even when the feature is unified.
- `crates/capability/` — fixture-only issuance/verify of closed-profile claims with wire tokens `low_risk_effectful` + `core_wasm` + `write_rfc5322`. Default issuance stays `pure_compute`.

## Tests

- `intent_is_allocated_before_content_access`
- `header_injection_in_prepare_is_refused`
- `substitution_of_recipient_or_bytes_requires_a_new_intent`
- `session_alone_cannot_produce_rfc0003_approval`
- `grant_replay_is_refused`
- `signer_epoch_mismatch_is_refused`
- `restart_invalidates_in_memory_grants`
- `compile_fail_access_to_private_proof_grant_payload_and_constructors`
- `fresh_uv_then_bridge_emits_rfc0003_approval`
- `sealed_payload_never_prints_its_message`
- `closed_profile_claims_use_exact_wire_tokens`
- `product_verify_still_rejects_the_closed_profile_bytes`
- `product_issuance_stays_pure_compute_on_the_wire`
- `product_verify_still_denies_the_closed_fixture_profile`
- `closed_fixture_profile_is_admitted_only_via_the_fixture_verifier`

## Gate

`./scripts/test_changed.sh` ALL GREEN — workspace + frontend tsc + vault-v2 qualification. **Design Accept ≠ product Current.**

## Out of scope (untouched)

Dispatch, reservation transaction, product `risk_class` admission (D05+), product Exact Effect / 1C0 / ActiveV2 / RP1, product issuance of `low_risk_effectful` or guest host interfaces, HMAC / hidden broker / second port / product CLI decide path.

## Suggested next item

v01-D05 — Reserve every authority fact in one redb transaction.

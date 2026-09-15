# Handoff: F03 / RP1-02 product exact-effect honesty pin

**Date:** 2026-09-15  
**Branch:** `cursor/f03-rp1-02-exact-eml-honesty-pin-9892`  
**Scope:** Documentation + adversarial honesty test only — no product Exact Effect implementation.

## What landed

- **`rp1_02_product_approval_does_not_bind_final_recipient_or_exact_eml_bytes`** in
  `apps/cli/src/workspace/f03_rp1_02_exact_effect_honesty_pin_tests.rs` pins the
  current product/local outbox path: RFC 0003 approval + Capability V2 + sandbox
  `workspace.delivery/prepare` bind `document_id` and `document:{id}` resource
  only; final RFC 5322 bytes are a separate argument to `execute_signed_approval`
  and are written by `OutboxBroker::write_message` without an exact-effect grant.
- The test **passes while documenting the gap** (tampered `To:` succeeds). Module
  comments name **Runtime Phase 1 RP1-02** and **F03**; invert when exact binding ships.

## Traceability

| Anchor | Link |
| --- | --- |
| Phase 1 acceptance **RP1-02** | [Runtime Phase 1 guide §5](../security/runtime-phase-1-development-guide.zh-CN.md#acceptance) — approval must eventually bind preview, reservation, and final file bytes |
| Finding **F03** | Same guide [§6 F03](../security/runtime-phase-1-development-guide.zh-CN.md#findings); kernel review [F03](../security/2026-09-16-agent-security-kernel-review.zh-CN.md) |
| Owner plan Tasks **8 / 10 / 11** | Exact preparation, effect broker, dispatch semantics (Program 2) |
| RFC **0006** | [Synthetic owner-session / exact local-outbox fixture](../../rfcs/0006-synthetic-owner-session-exact-effect-fixture.md) — mechanism proof; **not** a product pass |

## Explicit non-claims

- **No RP1-02 product qualification.** This pin records current behavior; it does not close F03.
- **Product Exact Effect remains gated** behind 1B1 + 1C1 + 1D `ActiveV2` + protected-payload review per RFC 0006 / ROADMAP. Fixture work may continue on its own track.
- **Experimental outbox** (`kernel_exec` app-signed path) unchanged; no Wave D / ActiveV2 / 1B0 work in this slice.

## Next implementation (queue)

See backlog entry **runtime-f03-exact-effect-product** — implement sealed intent + grant-handle outbox write; then **invert** the pin test to `expect_err` on tampered bytes.

## Verify

```bash
cargo test -p sovereign-cli rp1_02_product_approval_does_not_bind_final_recipient_or_exact_eml_bytes
./scripts/test_changed.sh
```

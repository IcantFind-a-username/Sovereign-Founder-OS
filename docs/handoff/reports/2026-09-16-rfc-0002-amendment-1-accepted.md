# RFC 0002 Amendment 1 Accepted — maintainer Written Acceptance

- **Outcome:** docs-only status flip. No product code. No Wave D Rust.
- **Date:** 2026-09-16
- **Authoritative text:** `rfcs/0002-wasm-sandbox-and-plugin-capabilities.md`
- **Status:** RFC overall remains **Draft**; Amendment 1 is **Accepted**
  (2026-09-16) as Target design for the closed fixture exact-effect
  profile. Design Accept ≠ product Current.
- **Product gates:** unchanged. No product 1C0, no product
  `low_risk_effectful`, no product Exact Effect, no RP1 product pass, no
  `ActiveV2`, no Phase D completion.

This note records design acceptance of Amendment 1. It is not itself an
RFC.

---

## What this licenses

Governance permission to treat RFC 0002 Amendment 1 as Accepted Target
design for the closed fixture profile (`low-risk-effectful` /
`write_rfc5322`), and to start Wave D / **v01-D01** after this
acceptance as release-excluded fixture work. Frozen tickets
**v01-D02…v01-D07** remain in
`docs/handoff/reports/2026-09-13-rfc-0002-amendment-1-wave-d-queue.md`
until claimed; they are licensed but not auto-claimed.

The fixture closed profile and hyphenated-name → existing snake_case
wire map stay as written. Product admission of `low_risk_effectful`
stays denied.

## What this does not license

1. Product 1C0 / WebAuthn on the workspace decide path.
2. Product issuance of `low_risk_effectful` or guest host interfaces.
3. Product Exact Effect / Program 2 product local-outbox.
4. Vault `PendingV2` / `ActiveV2` / product enrollment.
5. RP1 product pass or a Developer Preview product tag.
6. Phase D completion or RFC 0002 overall `Accepted`.
7. Finishing the v1 HMAC hidden broker (`BrokerReady`, HMAC IPC, hidden
   child, second listen port).

**Explicit separation:** Accepting Amendment 1 ≠ implementing Wave D ≠
product Exact Effect.

## Discussion-window waiver

`CONTRIBUTING.md` requires a minimum 7-day discussion for substantial
RFC changes unless a maintainer records why a shorter window is allowed.
The remaining window is waived in writing:

- sole maintainer;
- proposal already on `main` (merged PR #146);
- Amendment 1 acceptance checklist complete;
- honest-close alignment (v0.1 = fixture + honest labels; product 1C0
  admission = v0.2);
- no co-authors.

## Residual risks

- RFC 0006 G13–G15 still describe the superseded v1 broker/IPC; they
  remain stale transport text. Retiring them is a **separate** RFC 0006
  amendment and must not touch G2. This acceptance does not amend RFC
  0006 or RFC 0007.
- Wave D must not touch product paths. A later D ticket that admits
  `low_risk_effectful` on a product or default-release graph is out of
  scope and forbidden.

## Attribution

Written Acceptance recorded by the sole maintainer (IcantFind-a-username /
Yiqun Xu) on 2026-09-16. No co-signers.

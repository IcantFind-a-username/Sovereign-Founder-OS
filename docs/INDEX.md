# Documentation

This page helps contributors find the current source of truth without reading the whole repository.

## Core Reading Path

| Document | Answers |
| --- | --- |
| [README](../README.md) | What is the product, who is it for, and what exists today? |
| [MANIFESTO](../MANIFESTO.md) | Which principles will the project not trade away? |
| [ARCHITECTURE](../ARCHITECTURE.md) | How is the current implementation structured? |
| [THREAT MODEL](../THREAT_MODEL.md) | What are the assets, attackers, boundaries, and defenses? |
| [ROADMAP](../ROADMAP.md) | What is being built now and what comes later? |
| [RFCs](../rfcs/) | What does each concrete design propose or specify? |
| [Data sovereignty design](superpowers/specs/2026-08-13-data-sovereignty-boundaries-v1-design.md) | Which privacy, visibility, local-compute, and recovery boundaries are approved targets rather than current claims? |

`WHITEPAPER.md` will join this path when a technical whitepaper exists. Until then, architecture, threat model, and RFCs are the technical sources of truth.

## Choose by Task

| If you want to… | Start here |
| --- | --- |
| Make a first contribution | [CONTRIBUTING.md](../CONTRIBUTING.md) and current open issues |
| Work a relayed task card as an AI worker session | [Handoff protocol](handoff/README.md), then [`docs/backlog.md`](backlog.md) and [`CLAUDE.md`](../CLAUDE.md) |
| Understand the founder MVP that ships in `sovereign ui` | [Founder MVP design record](superpowers/specs/2026-09-10-founder-mvp-consultant-core-v1-design.md) |
| See what live local-model script verification actually proved (and what it did not) | [ROADMAP — Live local model verification](../ROADMAP.md#live-local-model-verification) and [2026-09-11 handoff report](handoff/reports/2026-09-11-live-model-verification.md) |
| Understand product direction | [README.md](../README.md) and [MANIFESTO.md](../MANIFESTO.md) |
| Change runtime architecture | [ARCHITECTURE.md](../ARCHITECTURE.md) and the relevant [RFC](../rfcs/) |
| Develop and accept Runtime Phase 1 | [Runtime Phase 1 development and acceptance guide](security/runtime-phase-1-development-guide.zh-CN.md) (Chinese; Target; contracts, existing task map, and product qualification gates); [Product 1C0 owner-admission design freeze](security/runtime-phase-1-product-1c0-owner-admission-freeze.zh-CN.md) (Chinese; Step 0 write-set and RP1-01 prerequisites) |
| Understand the security-kernel target design | [Agent security-kernel review](security/2026-09-16-agent-security-kernel-review.zh-CN.md) (Chinese; source review and design proposal, not an accepted RFC or product assurance) |
| Review security | [THREAT_MODEL.md](../THREAT_MODEL.md), [SECURITY.md](../SECURITY.md), [RFC 0002](../rfcs/0002-wasm-sandbox-and-plugin-capabilities.md), [RFC 0004](../rfcs/0004-data-sovereignty-boundaries.md), [RFC 0005](../rfcs/0005-dual-root-vault-and-recovery.md), and the [open-source cross-validation note](security/open-source-security-cross-validation.md) |
| Study privacy, recovery, or resilience targets | [Privacy model](design/privacy-model.md), [approved data-sovereignty design](superpowers/specs/2026-08-13-data-sovereignty-boundaries-v1-design.md), [RFC 0004](../rfcs/0004-data-sovereignty-boundaries.md), [RFC 0005](../rfcs/0005-dual-root-vault-and-recovery.md), and [Distributed systems](design/distributed-systems.md) |
| Discuss product UI | [GUI design draft](product/gui-design.zh-CN.md) (Chinese) |
| Run bounded Luna development | [Luna scaffold: single execution entry](handoff/codex/README.md) (Chinese; frozen documentation, implementation pending; canonical source map, cards, interfaces, review and escalation) |
| Study the business demo and component choices | [Founder OS product and research blueprint](product/founder-os-execution-blueprint.zh-CN.md) (Chinese; product vision, verified baseline, source-linked research) |
| Understand the category positioning | [Why Not Another Agent?](positioning/why-not-another-agent.md) |
| Trace how the idea evolved | [Historical Chinese design archive](archive/zh/README.md) |
| Decide whether a preview or `v0.1` tag is allowed | [Developer Preview tag and release policy](release/preview-tag-policy.md) |

## Document Status

- **Current:** describes code or policy that exists now. Architecture must make this explicit.
- **Experimental:** describes a runnable but narrow, simulated, or unhardened implementation.
- **Target:** describes intended behavior that is not fully implemented. Design notes and draft RFCs use this label.
- **Research:** records an option that still needs evidence and an accepted design before product commitment.
- **Historical:** preserves earlier reasoning but is not a current specification.

If documents conflict, current implementation plus accepted RFCs take precedence, followed by the core documents above. Historical material is context only.

A separate Secure Mesh protocol RFC is Research, not yet a planned product
file, so this index does not link it until measured need promotes the work to
an accepted RFC proposal.

## Community and Project Policy

- [Contributing](../CONTRIBUTING.md)
- [Governance](../GOVERNANCE.md)
- [Code of Conduct](../CODE_OF_CONDUCT.md)
- [Security reporting](../SECURITY.md)
- [Language policy](LANGUAGE.md)
- [License](../LICENSE), [Notice](../NOTICE), and [Trademark policy](../TRADEMARK.md)
- [Developer Preview tag and release policy](release/preview-tag-policy.md)

## Current RFCs

| RFC | Status | Topic |
| --- | --- | --- |
| [0001](../rfcs/0001-canonical-task-contract.md) | Draft | Canonical task contract |
| [0002](../rfcs/0002-wasm-sandbox-and-plugin-capabilities.md) | Draft; Amendment 1 accepted 2026-09-16 (fixture exact-effect profile, Target only); partially implemented; not a product Current claim | WASM sandbox and plugin capabilities |
| [0003](../rfcs/0003-signed-approval-evidence.md) | Draft; partial foundation | Signed approval-role evidence; approval-expiry retention is tested, while owner ceremony, transactional reservation, revocation, and full subprocess races remain Target |
| [0004](../rfcs/0004-data-sovereignty-boundaries.md) | Draft; approved implementation target | Data sovereignty, privacy compilation, visibility, and compute placement |
| [0005](../rfcs/0005-dual-root-vault-and-recovery.md) | Accepted (2026-09-14); Amendments 1–2; Program 1A engine Experimental (non-product); Programs 1B+ not started; no current protection claim | Dual-root Vault, backup, and recovery target |
| [0006](../rfcs/0006-synthetic-owner-session-exact-effect-fixture.md) | Draft; fixture-proof contract (no product claim) | Synthetic owner-session / exact local-outbox fixture — a mechanism proof for 1C0 (WebAuthn/session/one-use approval) and Program 2 (exact `.eml` effect), gated conjunctively behind 1B1 + 1C1 + 1D `ActiveV2` + protected-payload review |
| [0007](../rfcs/0007-audit-ledger-freshness-anchor.md) | Draft; approved implementation target for v0.1 anchor slice; Amendment 1 Accepted 2026-09-16 (latest-head/generation vs signing-key — Target design, not RP1-06 product); no RP1-06 product claim | Audit-ledger freshness anchor — device-signed head sidecar and open-time rewind/fork rejection when the anchor is protected independently of the ledger; Amendment 1 adds enrolled generation and authority coupling for paired-restore and downgrade detection (whole-device rollback stays Research) |

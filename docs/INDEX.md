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
| Understand product direction | [README.md](../README.md) and [MANIFESTO.md](../MANIFESTO.md) |
| Change runtime architecture | [ARCHITECTURE.md](../ARCHITECTURE.md) and the relevant [RFC](../rfcs/) |
| Review security | [THREAT_MODEL.md](../THREAT_MODEL.md), [SECURITY.md](../SECURITY.md), [RFC 0002](../rfcs/0002-wasm-sandbox-and-plugin-capabilities.md), [RFC 0004](../rfcs/0004-data-sovereignty-boundaries.md), [RFC 0005](../rfcs/0005-dual-root-vault-and-recovery.md), and the [open-source cross-validation note](security/open-source-security-cross-validation.md) |
| Study privacy, recovery, or resilience targets | [Privacy model](design/privacy-model.md), [approved data-sovereignty design](superpowers/specs/2026-08-13-data-sovereignty-boundaries-v1-design.md), [RFC 0004](../rfcs/0004-data-sovereignty-boundaries.md), [RFC 0005](../rfcs/0005-dual-root-vault-and-recovery.md), and [Distributed systems](design/distributed-systems.md) |
| Discuss product UI | [GUI design draft](product/gui-design.zh-CN.md) (Chinese) |
| Understand the category positioning | [Why Not Another Agent?](positioning/why-not-another-agent.md) |
| Trace how the idea evolved | [Historical Chinese design archive](archive/zh/README.md) |

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

## Current RFCs

| RFC | Status | Topic |
| --- | --- | --- |
| [0001](../rfcs/0001-canonical-task-contract.md) | Draft | Canonical task contract |
| [0002](../rfcs/0002-wasm-sandbox-and-plugin-capabilities.md) | Draft; partially implemented | WASM sandbox and plugin capabilities |
| [0003](../rfcs/0003-signed-approval-evidence.md) | Draft; partial foundation | Signed approval-role evidence; human ceremony remains target |
| [0004](../rfcs/0004-data-sovereignty-boundaries.md) | Draft; approved implementation target | Data sovereignty, privacy compilation, visibility, and compute placement |
| [0005](../rfcs/0005-dual-root-vault-and-recovery.md) | Draft; implementation none | Dual-root Vault, backup, and recovery target |

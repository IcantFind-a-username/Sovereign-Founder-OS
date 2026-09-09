# Founder MVP — Consultant Core v1 (orchestrator-built)

- **Outcome:** landed on local `main` (not pushed)
- **Base:** `cc43bd9` (Codex blueprint tip, fast-forwarded onto main) · **Head:** see `git log`
- **Commits:** slice A business graph · slice B Ollama provider and model.json · slice C AI employees and decisions · slice D compliance pack and checks · slice E pages · slice F docs
- **Backlog entries:** see `## MVP product line` in `docs/backlog.md`

## What changed

- `apps/cli/src/workspace/erp_types.rs`, `erp_ops.rs` — projects, tasks, follow-ups, payments, editing, acceptance, receivables, timeline; workspace version 2
- `crates/model/src/ollama.rs`, `apps/cli/src/workspace/model_config.rs` — loopback-only Ollama provider, `model.json`, `/api/model/status`
- `apps/cli/src/workspace/crew_types.rs`, `crew_roles.rs`, `crew_ops.rs` — six roles, allowed-input snapshots, prompts, output validation, deterministic templates, decisions, work suggestions
- `apps/cli/src/workspace/compliance_pack.rs`, `compliance.rs` — Singapore demo rule pack with sources, keyword retrieval, deterministic checks, reports bound to a facts digest
- `apps/cli/src/ui_mvp.rs`, `apps/cli/assets/*` — routes and the seven-tab bilingual UI
- `docs/superpowers/specs/2026-09-10-founder-mvp-consultant-core-v1-design.md` — design record; README and ROADMAP status text

## Tests

`erp_tests`, `crew_tests`, `compliance_tests`, `model_config` unit tests, `crates/model/tests/ollama.rs`, `ui_mvp` unit tests; every slice passed `./scripts/test_changed.sh` (ALL GREEN) before its commit. Browser verification of the pages was done by hand in the in-app browser against an isolated data directory.

## Deviations from the earlier plan

- The MVP was built on the real Workspace (vault, policy, audit chain) rather than the synthetic S2 business demo; the S2 candidate remains on `main` as its own `business-demo` command, unreviewed.
- Cloud model adapters were not added; only the loopback Ollama adapter. RFC 0004 still gates real egress.

## Discovered problems (queued)

See the open entries under `## MVP product line`.

## Open questions for the owner

1. Push `main` (a PR is required by the ruleset) — the owner decides when.
2. Whether to keep the Codex lane docs on `main` now that the lane is paused.
3. Whether the S2 `business-demo` command should stay, be folded into the Workspace, or be removed.

## Suggested next item

`HO-001` (capability bundle consumption) for the kernel line; for the product line, the first open entry under `## MVP product line` (live verification with a real Ollama model and the five-consultant usability protocol).

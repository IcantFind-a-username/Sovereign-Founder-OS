# S2-01A1 supervised execution

## Attempt 1 claim

Source base: `c89f7c908c154cdef3b929beec475ee46dc886e8`.
Dispatch HEAD is the controller claim commit containing this record; resolve
it from Git history. Partial independent design admission is documented in
[S2-00 supervision](S2-00-supervised.md). Only A1 implementation is claimed.

Worker: reuse `/root/s1_08_attempt2` for **new task S2-01A1 attempt1**, actual
**gpt-5.6-luna / medium** verified from session turn_context
`01a083f6-94a4-7d42-a07c-d50d68d8b9d3`. Its completed S1 task remains historical;
this is neither resumption nor erasure of that task's two-failure record.
New-agent creation previously hit the platform thread limit; existing role
reuse does not imply any automated harness/escalation capability.
Reviewer/fallback use independent Astra as prescribed. No self-acceptance.

One serial implementation writer. Worker owns exactly:

- `apps/cli/src/main.rs`
- `apps/cli/src/business_demo.rs`
- `apps/cli/business-demo/index.html`
- `apps/cli/business-demo/app.js`
- `apps/cli/business-demo/tsconfig.json`
- `apps/cli/business-demo/app.test.mjs`
- `apps/cli/tests/business_demo_http.rs`
- Worker report: `docs/handoff/codex/reports/S2-01A1.md`

Raw evidence: `.harness/s2-01a1/attempt1/`. Controller owns this record, backlog,
cards and admission records; no concurrent edits to worker-owned files.
Worker never commits, merges, pushes, opens PRs, changes dependencies/gates,
or modifies the frozen design or prior S1 source/test helpers.

Frozen inputs:

| Path | Git blob |
| --- | --- |
| `docs/handoff/codex/cards/S2-01A1.md` | `6b82038ddb684578141464479747a8b772025964` |
| `docs/handoff/codex/business-demo-contract.md` | `1a8047774ddef8ea4fe7827fe93336d306486d4c` |
| `apps/cli/src/main.rs` | `8694b214c3a589567c18d0319451104a7482d5f7` |
| `apps/cli/Cargo.toml` | `3e04e87ab77529a572bd84b2cdd8c4794ff86a51` |
| `crates/consultant-playground/tests/support/transport.rs` | `bf7b594e9bdd55305e04c34ea873ca1509127279` |

Budget: 30 minutes, up to three targeted repair cycles for a candidate; maximum
two substantive Luna failures before Astra fallback. Keep initial failing logs,
command output and failed probes; no silent overwriting or failure-count reset.
This task starts with zero implementation failures. Report exact active handles
and concrete progress if the budget ends while work is unfinished.

Implement the first runnable company/service form, backend save/reset and
required error behavior only. The card's seven-file bootstrap exception is
independently admitted; case/discovery and later business fields wait for A2.
Use existing pure amount parser and unchanged ChildServer/HTTP helper; declare
no duplicate launcher/capture/parser or speculative framework. Existing S1/legacy
paths remain unchanged; real providers/legal integration remains later work.

Run every card-named unit/HTTP/Node test and explicit new tsconfig check, then
required Rust checks and test_changed with a unique raw log. Check test-name
coverage, nonzero execution and skipped checks. Report actual source/test delta,
all untracked files, reuse/new tools and evidence; return for independent browser
and implementation review. No claim of complete S2 or full MVP follows.

Admission checks passed: local Markdown targets, exact six-document scope,
`git diff --check`, and docs-delta `test_changed` against c89f7c9 (session39058,
exit0: self-test/file-size/fmt). Raw `.harness/s2-00/first-slice-admission.log`.
No product tests were needed for these status/claim-only changes; product
checks become mandatory on the implementation candidate.

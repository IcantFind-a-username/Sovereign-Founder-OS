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


## Attempt 1 — changes requested

Worker stopped. Independent `/root/blueprint_review` (actual Astra/max, existing
reviewer reuse) rejected the eight-file uncommitted candidate over HEAD95c4af4.
Exact snapshots/bindings are `.harness/s2-01a1/attempt1/root-candidate/manifest.json`,
SHA256 `6ad0833f72f0261d556b7767cefb67a8f77b9ab6eb62737887aff61a8a31a2e9`.
Full candidate including report: +529/-0; no dependency, S1 or out-of-scope writes.
Backend blob `7609319dc1b16274528f71f32f74463c3b4bb417`, frontend
`bb50127786c2507709859f2c2d96549a4fd3e893`, HTTP test
`7c4a132530bd667dd5ee16929eb7f070cd3aead7`; worker report
`aaa73da60d171d59ec479bd2f7bc776375ac7f0e` preserved with the snapshot.

One substantive Luna failure. Required repair, in order:

1. Restore exactly `/`, `/app.js`, `/api/demo`, `/api/demo/command`; remove
   invented aliases. Match success/error envelopes, fixed status/code/headers,
   per-route methods and 405 Allow. Serialization failure cannot become success.
2. Enforce exact Host/Origin, POST custom header/media, GET body and POST length,
   duplicate framing/Expect/upgrade checks before appropriate reads; reject
   malformed/unknown/duplicate JSON fields. Preserve library pre-handler limits.
3. Restore frozen signatures/error types and clone→validate→bounds→commit;
   enforce revision/view limits, reset epoch difference with bounded retry,
   short-field controls, complete replacement and unchanged-state rejection.
4. Implement specified DOM rendering/fragment routing and exported signatures;
   canonical fetches/custom header/redirect policy, one in-flight request, busy
   controls, 409 refresh/explanation, 422 field retention, failed-state hiding
   and working Reload, inline reset cancel and grouped money formatting.
5. Replace shallow named-test bodies with the card's actual success/negative
   matrices and unchanged-view assertions. Include stale revision/reset, detached
   clone, exact startup capture, all routes/headers, real import-side-effect
   detection and library pre-handler cases. Test names alone are insufficient.

Full per-test repair details and candidate evidence:
`.harness/s2-01a1/attempt1/independent-review.md`. Reviewer did not run browser,
release or full gates because source violations already establish rejection.
Those acceptance checks remain required after repair. Missing screenshots were
honestly pending, not fabricated. The worker reported 4 unit/5 HTTP/4 Node passes
and gate94908 ALL GREEN, but its report still said running; final evidence must
be recorded accurately. Internal full-gate logs omit stdout summary by design.
Controller's preliminary strict:false concern was disproved: compilerOptions
correctly copy the existing frontend and are not a gate-relaxation finding.

## Attempt 2 claim

Same frozen A1 revision3/card/interface/write set; **failure count remains 1**.
Reuse actual Luna medium `/root/s1_08_attempt2` for S2-01A1 attempt2. It starts
from the preserved uncommitted attempt1 candidate plus this controller record
commit. Do not misdescribe HEAD alone as containing that candidate. No source
reset or earlier history deletion. Original raw snapshots/logs remain unchanged;
new evidence only `.harness/s2-01a1/attempt2/`. Worker report path remains
`reports/S2-01A1.md`, with prior exact report preserved in attempt1 snapshots.

Budget30min, at most3 targeted repair cycles. A second substantive failure stops
Luna and routes to Astra fallback/re-slicing; no third Luna retry. Repair test
assertions first and retain authentic RED/GREEN outputs, then implement all
five groups. Preserve the existing parser/helper/tsconfig reuse. Do not expand
A1 into later business or pending S2 design. Return stable candidate and exact
checks/remaining browser evidence for independent review; never self-accept.

Controller independently reran test_changed on the unchanged attempt1 product
candidate before recording attempt2: session47206 exited0; full workspace
clippy/tests, fmt/size/self-test and legacy frontend tsc ran. Raw root-gate-full.log
and root-gate-summary.log under attempt1 retain complete output and terminal
summary. This passing gate does not overrule the source/behavior rejection.
`git diff --check` passed. This commit records only backlog and this report;
product candidate remains explicitly uncommitted and bound by its manifest.


## Attempt 2 — changes requested; Astra fallback

Worker stopped. Independent `/root/blueprint_review` confirmed a second
substantive failure after verifying the exact eight-file attempt2 manifest.
No third Luna attempt is authorized. Source HEAD remains the records commit
`b7bb5ba1b0f88f6d4fa5afde86f5c80f5ccabc21`; product is uncommitted and preserved
under `.harness/s2-01a1/attempt2/root-candidate/`. Manifest SHA256:
`a9fd45fdbb5da8417e4a533c0b1dfd3db975bcdbd696e48faf35dbc6cd5b6a3d`.
Backend blob `fa2298d6f93559765751901ba1e11f241eac61a7`; app
`67df06acc4f7b6902479f92747817fa4b68fd5e7`; HTTP tests
`9503c666f61b8b4dc62d526b47d9c0f129b791bd`; report
`0c93e8f29e3bd6c7a72af1f94c62fd1a894538d4`.

Actual improvements: canonical routes/envelopes, Host/Origin/custom header,
handle/requestFor signatures, fetch redirect/header and grouped money.
Remaining issues still fail the same frozen A1:

- UI uses interpolated innerHTML, wrong render/dispatch state protocol, missing
  busy/409/422 handling. Initial failure writes literal button markup through
  textContent then dereferences a nonexistent Reload control. Reset Confirm and
  Cancel buttons default to submit, triggering unintended form submission.
- State clone/limits/reset collision and short-text rules remain incomplete.
- Request media/framing/strict JSON, exact statuses and CSP remain incomplete.
- Named tests retain shallow bodies, including save_conflict_and_reset without
  an actual conflict or reset; required negative matrices remain unproved.

Consolidated independent fallback checklist:
`.harness/s2-01a1/attempt2/independent-review.md` (reviewer-owned).
The final gate stdout is now preserved and reports ALL GREEN, with workspace
clippy/tests, fmt/size/self-test and legacy tsc. Root reverified current file
hashes against the candidate manifest; that unchanged-source passing evidence
is reused for this records-only commit. It does not establish acceptance.
Raw observations describe a post-repair HTTP RED; no historical pre-change
TDD evidence is inferred or reconstructed. Browser/release acceptance pending.

### Fallback claim

Reuse `/root/s2_design_card_plan`, actual configured **gpt-6-astra / high**, to
perform bounded fallback implementation of already-frozen A1. New agent thread
creation was unavailable; this is an explicit existing-actor reuse, not a claim
that its registered founder_architect role was changed to founder_fallback.
Its configured instruction prohibits unfrozen product behavior, which remains
prohibited here. Another Astra reviewer independently reviews its repairs.

Same exact seven source/test paths and worker report as the original claim;
no dependency, S1 helper, contract, card or gate changes. Both failed snapshots
and logs stay untouched. New raw path `.harness/s2-01a1/fallback/`. No commits,
push or self-acceptance. Controller commits only reviewed candidates/records.
Repair complete behavior and meaningful tests; do not simply make names pass.
Use existing ChildServer/HTTP/amount helpers, no replacement frameworks.
First coherent checkpoint within30min; preserve active handles and remaining
work if more time is needed. No failure-counter reset or unrelated stage work.
Next: independent code/browser/release review of the fallback candidate, then
A2 only after A1 implementation acceptance. Full MVP remains incomplete.

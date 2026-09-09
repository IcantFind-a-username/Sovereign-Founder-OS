# S2-00 supervised execution

## Design-task admission

Base: `ac4e09d759c5f1beebd8e3837d06ddd50d98fb59`, S1 independently accepted.
Card revision 1: `9a14ba3d45295db32fa16bf1bc66ea16232176f1`;
SHA256 `2f5afc267a280e1fd35e643ed657ff36cd71763ee8c99c1a04d235f04de144fa`.
This admits architecture work only; no S2 design or implementation is accepted.

Planner `/root/s2_design_card_plan` (founder_architect, Astra high) produced
`.harness/s2-00/plan/S2-00-proposed.md`, SHA256
`b60fc781f74fe848fb45eeef16255b1ba468c7431d519d66ce0fe07e379c45b5`.
Independent `/root/blueprint_review` requested only explicit optional bootstrap
subcard paths. Controller added S2-01A1/A2 paths, non-executable S2-01A index
semantics when split, and S2-01B dependency on the last split card.
Reviewer inspected the exact amended SHA256 above and accepted admission.
No runtime or browser checks were claimed by this read-only proposal review.

New reviewer creation failed with the platform agent-thread limit. Controller
reused an existing independent Astra reviewer and verified its actual session
turn-context model/effort as **gpt-6-astra / max**, not the preferred high.
This bounded stronger-effort substitution is recorded explicitly; no Luna
review or automatic escalation claim follows. Source/session evidence:
`rollout-2026-09-09T04-45-53-01a082c5-2695-79c3-90cf-6f8dcce29fb0.jsonl`.

## Design attempt 1 claim

Actor: reuse `/root/s2_design_card_plan`, configured founder_architect
**gpt-6-astra / high**. Source base is the commit above; dispatch HEAD is the
controller admission commit containing this claim, resolved by Git history.
No worker commit, push or PR. One writer; architect owns exactly the document
paths enumerated in [S2-00](../cards/S2-00.md), including its design report.
Controller owns this report, backlog and task-card registration; it will not
edit architect-owned documents during execution. Optional raw design evidence:
`.harness/s2-00/design/`. Existing proposal is preserved unchanged.

Budget: 30 minutes for a first coherent design candidate, at most three
focused repair cycles; report a concrete checkpoint if that budget is reached.
No Luna implementation attempt has started and no failure history is reset.
Prioritize a runnable editable company/lead/discovery slice, then full business
versions/review/delivery/billing. Preserve S1 and legacy behavior. S3 real models,
legal/RAG and unified final integration remain mandatory for the full goal.

Architect returns finite small cards with exact interfaces, dependencies,
write sets, named tests and applicable commands. Submit design candidate to
independent review before any implementation card becomes executable.

## Controller checks

Admission `git diff --check` passed. Local Markdown target-path checks on the
four changed documents found no missing targets; future plain-text output paths
are design deliverables, not falsely reported as existing files.
`TEST_CHANGED_LOG=.harness/s2-00/admission-full.log ./scripts/test_changed.sh`
completed with exit 0 (session 13728): gate-self-test, file-size, fmt,
workspace clippy/tests and legacy frontend TypeScript all ran. Summary is
`.harness/s2-00/admission-gate.log`; no required skipped check was reported.
Final admission scope: backlog, README, S2-00 card and this controller report.
New files were checked with `git ls-files --others --exclude-standard`.
No product changes or new browser conclusions; accepted S1 evidence remains
in its own stage report.


## First-slice design acceptance and remaining work

First design candidate `ef5440a71bd0e0451969fa8d4d3a81a8d1567ec0` received
three P2 findings from independent `/root/blueprint_review` (actual Astra/max):
code-enum spelling, tiny_http pre-handler response promises, and next-step
branches. Evidence `.harness/s2-00/design-review.md` is preserved.
Architect repaired the same attempt, cycle1, into
`c89f7c908c154cdef3b929beec475ee46dc886e8`; raw
`.harness/s2-00/design-repair1/` preserves bindings and passed document gate.
No product code changed; original design history remains committed.

The same independent reviewer explicitly accepted **S2-01A1 and S2-01A2 design
admission in dependency order** at c89f7c9. Enum and HTTP corrections pass;
startup/helper/dependency reuse and bounded first-launch write set are accepted.
This is partial design admission under protocol §9.7, not full S2 implementation
or complete design acceptance. Later-only next-step revision-cap handling
remains pending; it does not affect company/service or case/discovery.
The reviewer verified that §5 already permits a successor for stale/expired
submitted proposals, resolving controller's possible-deadlock concern.

Reviewed contract revision2 blob `1a8047774ddef8ea4fe7827fe93336d306486d4c`;
reviewed A1 revision2 blob `e627fa838a73fe1ea3f535f2134965d98601af99`;
A2 revision2 blob `41998f862e85804530f7e7b6c8dc68452327e3cf`.
Controller advances A1/A2 to revision3 only to record partial-admission status
and its evidence link; their behavior, interfaces, tests and write sets are
unchanged. The global contract remains a design candidate outside these scopes.
Next product work is S2-01A1, then S2-01A2 after A1 implementation acceptance.

Final partial-review verdict retains one later P2 only: contract §9's
revision10,000 replacement advice must include row13 complete_followup;
S2-06 must add due_followup_at_revision_cap. No new framework/file is needed.
S2-03's submitted shorthand can be qualified as current and unexpired.
Preserve this pending design work while A1 implementation advances.

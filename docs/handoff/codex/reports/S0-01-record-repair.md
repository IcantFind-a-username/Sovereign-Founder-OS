# S0-01 record repair — blocked

2026-09-09. This is a controller-requested record audit, not an implementation
attempt or independent acceptance. Outcome: **blocked**, diagnostic
`S0_01_HISTORY_NOT_REPLAYABLE`. No worker implementation card is released.

## Actual Git evidence

The inspected checkout is this repository's working tree, branch
`docs/founder-os-execution-blueprint`, initially clean at
`780975956f14ba24fcbe64e66a63010cde23684f`. Git uses SHA-1. Its actual linear
history is:

| Role in the observed history | Commit | Parent / changed paths |
| --- | --- | --- |
| B, last committed baseline before claim | `34e307beda8a9498d410b78d3e617d493009a573` | Records the preceding continuity-document review in backlog. |
| Historical backlog claim; incomplete C | `a3d1664e963809172c0a83b4ba6ef6cf72e222c4` | Parent B; only `docs/backlog.md`. No contract or claim event. |
| Configuration implementation, not a valid W | `ebceebf8cdf9d59ecfc2b8a2ebf9210de6160499` | Parent a3d1664; the five `.codex` files listed in the draft. |
| Historical smoke report, not a valid report-only R | `1a0f2d4e55e91776ed70bd1d7efccf0e45b4d460` | Parent ebceebf; both backlog and `reports/S0-01-attempt-1.md`. |
| Report whitespace correction | `780975956f14ba24fcbe64e66a63010cde23684f` | Parent 1a0f2d4; only that report. |

`git diff --name-only B ebceebf` includes `docs/backlog.md`. The worker did
not branch from B as protocol §9.6.1 requires. Choosing a3d1664 as a new B
after the fact would conceal the pre-claim baseline and missing C freeze.
Choosing ebceebf as B, as the smoke report does, would omit all configuration
changes. Neither makes the old implementation a lawful W. The later report
correction cannot repair the mixed report/backlog commit.

At inspection, `git log --all -- docs/handoff/codex/tasks
docs/handoff/codex/events` returned no history, and neither directory existed.
No archived contract or Event array was found. Existing claim and attempt
facts remain in their original commits and report, unedited. The report blob
at 7809759 is `ffef538faa70c2d9f17ffc1ed85caed983cf1899`.

## Evidence preserved and missing

The original report records attempt 1, a Luna read-only smoke thread
`01a0833d-1fe6-7813-8b15-103204ebe086` with explicit
`gpt-5.6-luna / medium`, and an Astra read-only smoke thread
`01a0833d-a813-7810-b2cb-e2fcd4f73c87` with explicit `gpt-6-astra / high`.
These are cited historical report claims, not newly observed dispatches and
not proof that either thread authored or independently accepted a W.
The report also records TOML parsing, controller approval waiting, actual
reviewer workspace-write permissions, unverified configuration loading,
nested dispatch and slot lifecycle. None is silently promoted to acceptance.

The original `Candidate` label conflicts with its statement that there is
no product candidate and with the missing candidate/check/review bindings.
Its failure count 0 is a historical statement, not a replayed result.
`spentMs` is explicitly unmeasured. The actual historical controller thread,
model/effort, pre-dispatch budget, implementation author binding, stopped
worker observation and complete CheckRun/Review are unavailable. No active
process or controller lock ownership has been verified by this record audit.

Contracts v2 requires a nonempty event stream beginning with a real claim;
Actor identifiers are nonempty, spentMs must be an integer and block must
name the actual source state. There is no unknown actor/time or legacy
diagnostic event. Inventing a claim/start/block would fabricate those facts;
an empty event array would also fail validation. Therefore this repair does
**not** create `events/S0-01.json` or pretend that a valid `Blocked` state has
been replayed. “Blocked” here is the audit outcome. No historical event,
attempt or failure is deleted, reset, reclassified as accepted, or replaced.

## Non-executable contract draft

[contract.draft.json](../tasks/S0-01/r2/contract.draft.json) contains the exact
TaskContract v2 fields so a reviewer can inspect the proposed shape. Its
noncanonical filename is deliberate: it is **not** `contract.json`, is not
a frozen claim snapshot, must not enter validateEvents contract history, and
must not authorize dispatch or acceptance. It cannot retroactively repair C.

All source/input object IDs are read from actual B Git objects:

| Path at B | Blob |
| --- | --- |
| `docs/handoff/codex/cards/S0-01.md` | `f6231acf98cb611b0798ee1aff49e8ca1daf5fa4` |
| `docs/handoff/codex/README.md` | `0a3619c323189fd93f0049c291d40e037c7b6f2f` |
| `docs/handoff/codex/protocol.md` | `1a4eb6780bc7b36f621b1ecf1f70d372acdfff4a` |
| `docs/handoff/codex/contracts.md` | `cd8c34ee497676aa7dc6749ab16646eeb88031e1` |
| `docs/handoff/codex/models-and-goals.md` | `09888cc8e0b1f0a672d3f57c60a617a15dbb4489` |
| `CLAUDE.md` | `d15b2a5d109f4d2e7090a13ae4ce5eea1e49f8d7` |
| `docs/backlog.md` | `e0c7fbb42c883211920517c90f514b7d206e3e2f` |

The five exact `.codex` write paths come from card revision 2; the separate
reportPath names the existing authorized attempt-1 report. Neither backlog
nor this repair report expands the worker write set. Dependencies are empty
as stated by the S0-01 card; backlog records S0-00 complete. Prior accepted
documentation does not constitute S0 implementation acceptance. This audit
changes no product interface or RFC boundary.

The expected actor models are the §8.2 bootstrap **requirements**, not a
claim about unknown historical actors. The 1,800,000 ms attempt budget is
only a retrospective proposal from protocol's 30-minute suggestion. The
60,000/1,800,000 ms command timeouts are proposed review parameters; neither
they nor this budget were proven frozen before attempt 1. The card's two
check commands are preserved; no past pass or new CheckRun is asserted by
the draft. A new attempt must use a newly approved exact report path and
parameters rather than overwrite attempt 1's report.

## Required next action and unresolved decisions

The controller must submit this blocked audit to an independent reviewer.
Before dispatching another implementation attempt, the strong architect and
reviewer must resolve how to preserve the incomplete legacy claim/attempt
under the strict v2 history rules. The current schema has no lawful automatic
import or reset path. A reviewed, explicitly authorized protocol migration
or other history-preserving recovery decision is required; this report does
not invent one, fabricate an attempt_fail, or authorize a reslice event.

The next implementation attempt needs a lawful number preserving historical
attempt 1, a real controller identity and stop/lock observations, a clean
selected B, all frozen card/input blobs, exact five-file write scope and a
new unique report path, actual actor snapshot and measured budget. C must
archive the contract and lawful events before dispatch; the worker branches
from B, supplies W, then a report-only R. Candidate checks must run at W;
an independent reviewer must bind its actual thread, checks and conclusion
to W. Bootstrap dispatch/caller/decision author and lifecycle evidence must
be recorded to the limits the tools actually demonstrate. Historical .codex
content may be reused only through that reviewed recovery, not by relabeling
the old commits. S0-02 and later work are not released by this audit.

Only the first unresolved recovery decision should be pursued next; do not
repeat this audit or treat another documentation pass as implementation.
Changing cards/protocol/backlog is outside this repair's write authority.

## Validation and tool reuse

Validation: Python standard-library JSON parse passed. Git verified B as an
actual commit and all seven card/input blobs as existing objects matching B.
The first binding check caught a mistyped backlog blob; it was corrected and
the complete check rerun successfully. `git diff --check` and explicit
`git diff --no-index --check /dev/null <path>` checks for both new files pass.
With `TEST_CHANGED_BASE=780975956f14ba24fcbe64e66a63010cde23684f` and
`GATE_SELFTEST_RUNNING=0`, `./scripts/test_changed.sh` exited 0:
`ALL GREEN — steps: gate-self-test file-size fmt — no cargo test scope`.
Its local log is `.harness/s0-01-record-repair-gate.log`; this is verification
of this documentation repair, not an S0-01 candidate CheckRun. JSON parsing
cannot validate historical truth or certify a frozen TaskContract. S0's
state/Git validator scripts do not yet exist at the inspected HEAD.

Reused: Git object/tree/diff inspection, Python standard-library JSON parsing,
and `scripts/test_changed.sh`; no new utility, scanner, serializer or runtime
dependency. The supplied Python benchmark/artifacts.py, report.py, metrics.py,
matcher.py and review/executor.py are absent from this Rust repository.
Verified existing Rust inventory: WorkflowRunner (`workflow/src/lib.rs:112`),
AuthorityStore (`authority/src/lib.rs:167`), AuditLedger/verify_chain
(`audit-ledger/src/lib.rs:42,135`), ModelProvider (`model/src/lib.rs:118`),
policy_decision_digest (`capability/src/v2.rs:802`), evaluate_prepared
(`policy/src/lib.rs:281`), all beneath `crates/`, and existing
`crates/consultant-playground/tests/support/` fixtures. None needs invocation
or reimplementation for this record repair. No product/configuration files,
credentials or external model APIs were accessed for implementation.

## Recovery summary

Goal remains models-and-goals §13.1; current stage S0, S0-01 revision 2.
Audit HEAD/B and the historical implementation are given above; lawful W
does not exist. Attempt 1 remains historical and unreplayable; actual
failure count, spentMs and lock owner are unverified rather than reset.
No accepted CheckRun/Review or new implementation is produced. Current
role is a delegated record auditor, not controller or S0 implementation
worker. Next: independent review and the named history-recovery decision;
only then freeze a lawful fresh attempt. Original claim/report and all
commits stay intact. This report is the exact authorized repair output.

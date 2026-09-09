# S2-00 design candidate — attempt 1

Status: candidate for independent review; no implementation dispatched.
Role: founder_architect, Astra high. Dispatch HEAD
`6e90868614ce6fb177db0b2fe9ae7f211fba7d89`; product source baseline
`ac4e09d759c5f1beebd8e3837d06ddd50d98fb59`; frozen S2-00 revision1 blob
`9a14ba3d45295db32fa16bf1bc66ea16232176f1`. No commit made by architect.

## Design delivered

The sole [business contract](../business-demo-contract.md) freezes a separate
`business-demo` command and memory-only browser exercise, editable company and
case forms, evidence/assumptions, immutable proposal snapshots, simulated review
and rework, explicit delivery results, separate invoice/receivable/payment
facts, follow-up and an exercise-day overview. It defines exact wire shapes,
Rust entry points, ownership, limits, transitions, route/error behavior and
the founder's complete browser checklist. Five AI roles and legal coverage
remain visibly disconnected; actual S3-M/S3-L and final integration are required.

Manual text is labeled unverified experiment input, not magically certified
synthetic. The new command never invokes legacy roots, store, owner keys, model
gateway or effects. It is not an owner authentication or at-rest protection
claim. tiny_http's weaker transport-liveness properties are explicit; no S1
boundary or gate is relaxed. Separate command/UI/port preserve legacy surfaces.

The first milestone is split: [S2-01A1](../cards/S2-01A1.md) produces a real
company/service browser form; [S2-01A2](../cards/S2-01A2.md) adds editable linked
case/discovery. Subsequent cards follow actual business behavior, without an
infrastructure-only staging chain. All registration glue and tests have owners.
First-launch seven-file scope is a documented bounded exception; most later
cards own three or four files. S2-01A is only an index. S2-07 is reviewer-only.

## Reuse and decisions

Reused in the design: existing pure workspace amount parser, serde/JSON,
tiny_http, locked rand/hex, ChildServer::start_command/stop/captured_output,
raw HTTP parser/request helpers, FakeRoot, RustLexer and established gates.
The existing startup parser is reused by keeping first line
`Playground: http://127.0.0.1:<port>` and identifying Business demo on line2.
No duplicate child launcher, capture loop, parser, canonical serializer, hash,
atomic writer or workflow engine is authorized. Legacy operational functions
were inspected but are excluded due to persistence/authority/UUID/time effects.

New work declared: S2 domain validations/transitions, detached view, basic DOM
forms and feature tests. No new runtime dependency. No measurement/automation
framework or Python package. No benchmark helpers were claimed to exist.

Open dependencies are outside this synthetic stage: true owner/authority and
protected coordination, RFC0005 governance/activation, real model dispatch,
legal retrieval/rules and final shared AI/business integration. S2 makes no
decision purporting to supply those mechanisms or professional legal approval.

## Validation and file evidence

Actual commands/results:

- `git diff --check`: exit0.
- Local Markdown target inspection across14 candidate documents:111 local
  references checked, zero missing targets. Existing external URLs were not
  fetched; this task makes no new external-source claim.
- `git diff --name-only` plus `git ls-files --others --exclude-standard`:
  only the14 permitted document paths; no unexpected writes.
- `TEST_CHANGED_BASE=6e90868614ce6fb177db0b2fe9ae7f211fba7d89 TEST_CHANGED_LOG=.harness/s2-00/design/full-gate.log ./scripts/test_changed.sh`:
  exit0, ALL GREEN; gate-self-test, file-size and fmt actually ran. The base is
  the accepted clean dispatch HEAD, whose full product admission gate had
  already passed. This candidate changes docs only, so no cargo-test scope
  was selected; no new product/browser/runtime test pass is claimed.

Gate log SHA256: `98b5726139612603584352a7f90c03e312af70e0e5331424e3f88eaa9b2985fd`.

Tracked `git diff --stat` (new files are separately included below):

```text
docs/handoff/codex/README.md     |  3 ++-
 docs/handoff/codex/milestones.md | 27 ++++++++++++++++-----------
 2 files changed, 18 insertions(+), 12 deletions(-)
```

New files (git untracked inventory, included in candidate scope):

- `docs/handoff/codex/business-demo-contract.md`
- `docs/handoff/codex/cards/S2-01A.md`
- `docs/handoff/codex/cards/S2-01A1.md`
- `docs/handoff/codex/cards/S2-01A2.md`
- `docs/handoff/codex/cards/S2-01B.md`
- `docs/handoff/codex/cards/S2-02.md`
- `docs/handoff/codex/cards/S2-03.md`
- `docs/handoff/codex/cards/S2-04.md`
- `docs/handoff/codex/cards/S2-05.md`
- `docs/handoff/codex/cards/S2-06.md`
- `docs/handoff/codex/cards/S2-07.md`
- `docs/handoff/codex/reports/S2-00-design.md`

Candidate file Git blobs (the report itself is excluded to avoid a recursive
self-hash; controller binds its final blob during acceptance):

| File | Git blob |
| --- | --- |
| `docs/handoff/codex/business-demo-contract.md` | `7719aaffd6bb514ad405917bec6f2abbd2c2f77b` |
| `docs/handoff/codex/README.md` | `cd03aee2bd608398d2a9c63681df51afbde5e5ac` |
| `docs/handoff/codex/milestones.md` | `b4814ccf0bb2c0b4e111a0f28a42d57992cbd07b` |
| `docs/handoff/codex/cards/S2-01A.md` | `650b7e37f423b47e4dba4bcf400308adec372249` |
| `docs/handoff/codex/cards/S2-01A1.md` | `e29c58be8c1655f7f175d9627e4dede1f20d6e6c` |
| `docs/handoff/codex/cards/S2-01A2.md` | `e4c3568fd3955b6ce02518be7d39a15a90841caf` |
| `docs/handoff/codex/cards/S2-01B.md` | `49f78afc5fd772113148437b70a8bf9056bb19fb` |
| `docs/handoff/codex/cards/S2-02.md` | `830bf34467efaab74327a28a3891bfba36c76d08` |
| `docs/handoff/codex/cards/S2-03.md` | `00a2f46bbaa053646eb4d5c86546613ccae35870` |
| `docs/handoff/codex/cards/S2-04.md` | `7324bbf92bcd456a43d0a0130f886da1aa30c14b` |
| `docs/handoff/codex/cards/S2-05.md` | `cd0a24c526381a4bce07e2a502d3447b17e8b507` |
| `docs/handoff/codex/cards/S2-06.md` | `706267bff58afbd36b0ba11853a430d29e855dd3` |
| `docs/handoff/codex/cards/S2-07.md` | `989f02d3a59b5f9f5f5ea2a21fa3e4c2a3da0b48` |

Raw manifest: `.harness/s2-00/design/candidate-files.json`, also records SHA256
and line counts for each listed document. New documents are explicitly checked;
plain git diff does not include them. No product/test/dependency/RFC/old gate,
backlog or controller record is modified. No browser/model/user observation is
asserted. There is no new implementation, new tool executable or runtime metric.

## Recovery and next action

Same attempt1 and accumulated budget; no failure history reset. The architect
stops at this candidate. Independent reviewer should inspect interfaces and
finite write sets, especially the initial visible slice and reuse boundary.
On acceptance controller records admissible cards and claims S2-01A1; the
architect does not dispatch implementation or declare the continuing goal done.

## Focused repair 1 — same design attempt 1

Latest status: repair candidate for exact-delta independent review. The first
candidate remains committed at `ef5440a71bd0e0451969fa8d4d3a81a8d1567ec0`;
all preceding validation/blobs above describe that preserved candidate, not this
repair. Its independent review was changes_requested with three P2 findings
recorded in `.harness/s2-00/design-review.md`. No Luna implementation failure,
new attempt, budget reset or replacement stage plan was introduced.

Repair scope is exactly the three findings:

1. Contract revision2 explicitly maps Currency/RegionCode/TransactionKind code
   variants to SGD, SG/EU/US, B2B/B2C and lowercase unknown; ordinary enums remain
   snake_case. S2-01A2 revision2 pins input/output roundtrips, null subdivisions
   and lowercase alias rejection with a named test.
2. Application HTTP guarantees now apply only after tiny_http delivers a request.
   Unknown Expect is a pre-handler417/empty response; library framing errors/
   disconnects and other direct transport responses do not promise application
   JSON/headers. S2-01A1 revision2 names tests separating library and handler
   behavior. No parser, library or S1 change is introduced.
3. Ordered total next-step table now includes valid draft→submit, completed
   tasks→acceptance draft, and expiry before review/plan. S2-06 revision2 names
   every branch/mixed-state fixture under its existing table-driven summary test,
   including bounds advice; no planner framework or new card is added.

Current write delta: contract, S2-01A1, S2-01A2, S2-06 and this report only.
No source/test/dependency/RFC/gate/backlog/controller writes. No new tools,
product implementation or runtime/browser claim. Reuse inventory is unchanged.

Repair validation and new file bindings are recorded below after checking the
final document delta. Raw repair artifacts use `.harness/s2-00/design-repair1/`;
original design/review evidence is retained unchanged.

Repair checks:

- `git diff --check`: exit0.
- Local link target check: 17 references in the five changed documents,
  zero missing targets; exact changed-file whitelist matched, zero new files.
- `TEST_CHANGED_BASE=ef5440a71bd0e0451969fa8d4d3a81a8d1567ec0 TEST_CHANGED_LOG=.harness/s2-00/design-repair1/full-gate.log ./scripts/test_changed.sh`:
  exit0; gate-self-test, file-size and fmt ran. Docs-only delta selected no
  cargo-test scope. No additional full product or browser test was needed or
  claimed for this documentation repair.
- Gate SHA256: `98b5726139612603584352a7f90c03e312af70e0e5331424e3f88eaa9b2985fd`.

Repair candidate Git blobs (report excluded to avoid recursive self-hash):

| File | Git blob |
| --- | --- |
| `docs/handoff/codex/business-demo-contract.md` | `1a8047774ddef8ea4fe7827fe93336d306486d4c` |
| `docs/handoff/codex/cards/S2-01A1.md` | `e627fa838a73fe1ea3f535f2134965d98601af99` |
| `docs/handoff/codex/cards/S2-01A2.md` | `41998f862e85804530f7e7b6c8dc68452327e3cf` |
| `docs/handoff/codex/cards/S2-06.md` | `cb8e8d075064f579b4e2b42d86135e2810f3d421` |

Raw bindings/scope: `.harness/s2-00/design-repair1/candidate-files.json`;
final diff stat: `.harness/s2-00/design-repair1/diff-stat.txt`.
Independent reviewer should inspect the exact delta from the retained first
candidate. Architect stops here; no implementation dispatch or self-acceptance.

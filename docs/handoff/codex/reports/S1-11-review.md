# S1-11 independent review

**Verdict: accepted**, bound to candidate HEAD
`36efdf28a3dad44499228a3345cf0288d4b77527`, product commit
`1d4da2423ba386a722f02fd190857aea6a2e7a10`, frozen revision2 card blob
`4903767318fc35a3c79d7b2d9419df394c3c59db`. Reviewer `/root/s1_11_review`
performed no implementation, gate relaxation, commit, merge, push or effect
authorization. This accepts S1's fixed synthetic Playground as runnable and
observed under the conditions below. It does not accept real-user usability or
complete [the full MVP goal](../models-and-goals.md#131-当前推荐-goal交付全流程可演示-mvp).

## Reviewed scope and binding

Read the frozen card, standalone-v2, domain/action/DTO, catalog, guidance,
HTTP, assets, server and launch contracts, RFC0004 local-consumer/model/recovery
constraints, RFC0005 owner-presence/activation constraints and full goal§13.1.
Inspected production Rust, all six assets, CLI dispatch, source/manifest gates,
real TCP tests and shared capture/root fixtures, accepted S1-07/08/09/10A/10
reports and their raw evidence. No accepted attempt history was changed.

Relevant implementation/gate commits, retained rather than squashed:

| Increment | Product | Preceding gate |
| --- | --- | --- |
| Fixed graph / physical boundary | b4a767b | 48b2446 split |
| Four actions | bc0fe7b | 5d909dd |
| Detached DTO | 9e938ef | 5e740b1 |
| Bilingual catalog | 551fbf5 | 4274b13 |
| Search / guidance | 8583294 | 59c0d7e |
| Typed HTTP | fecab4c | c978e9b |
| Embedded UI | 18844cb | da47f15 |
| Bounded transport | b7caaad | f65a6ed |
| Real CLI wiring | 6e0976c | existing accepted boundaries |
| Complete child capture | 73cd9b2 | frozen S1-10A |
| Release root isolation | 1d4da24 | frozen S1-10 revision3 |

All new raw review evidence is under `.harness/s1-11/reviewer/`. Its
`artifact-manifest.json` binds 283 files plus reused gate/evidence hashes,
SHA256 `509ff2821b3ed7b37f6234edffa0a87824f9df414fa8dcbd050d1981917b5423`.
These ignored local artifacts are not Git-hosted evidence. The manifest includes
complete HTTP requests/responses, synthetic seed inventories, process metadata,
stdout/stderr and browser events; it does not substitute for their inspection.

Candidate release binary SHA256:
`0c259b55cb637f8a02a69e00ee51aa9c0937c6676b5826295aad0968ace2222f`.
An exact release build during review completed without changing this hash.
S1-08 used the earlier `8eaec9f6…` binary; its frozen asset/Node source hashes
still match this candidate, while the new live browser checks use the current
release binary. No old binary is represented as current execution.

## Physical scope, HTTP and authority

The unpublished leaf has exactly six production source files and three normal
dependencies: serde1.0.228, serde_json1.0.150, httparse=1.10.1 with defaults
false. Cargo's transitive tree is preserved in root-gates/cargo-tree.log.
No product crate, dev/build dependency, feature, alternate backend or runtime
root selector enters this leaf. Its sole public entry is run(u16). The graph
and its fields remain private and non-serializable; copied response DTOs are
one-way. Four closed unit actions cannot carry arbitrary business values.

The actual source/manifest hostile checks executed. They pair module inventory,
exact production token fixtures, terminal test wrappers, six ordinary embedded
asset files, source roots and symlink rejection. This is complementary to
runtime nonmutation evidence: equal output alone cannot prove absence of reads.

Inspected exact eight-route matching and validation order: trusted bound port,
unique Host, optional unique matching Origin, raw target, route, method, body
size, encoding/media/JSON and session. Transport framing precedes handler
validation. Duplicate headers survive httparse; no target normalization occurs.
Limits remain8192 head bytes/32headers/257 actual body bytes, with one shared
five-second read deadline and separate total five-second write deadline.
Instant/Duration serve only transport liveness; startup stdout prints the fixed
listener URL. There is one literal loopback bind, one session and one request
per connection, no outbound client, production thread, filesystem/environment
read, business clock or request-controlled authority. A completed POST may
apply before response loss; the product makes no rollback/exactly-once claim.

The real TCP suite actually exercised seven named transport cases, including
framing/body limits, raw duplicate/target/method precedence, typed errors,
nonmutation, slow clients, disconnect/pipeline behavior, restart and process
stop. Private tests exercised read/write deadlines and write failures. Exact
response headers omit Date/Server/CORS/cookies. The S1-07 original94 execution
log SHA256 is `42097b62be0e8b85a505b62f96c5dd94e53a63f9c3d1aa62711dfeae17e32e89`;
review reran the current97 executions including later capture regressions.

All three JS modules were inspected: only app.js fetches, only the two literal
same-origin APIs, fixed action JSON, omitted credentials, no-store and rejected
redirects. Rendering uses static DOM/textContent. No storage, cookie writes,
remote assets, dynamic code, timers/retries/polling, workers, websocket or
business input is introduced. The CLI arm directly calls the leaf and performs
no data_dir/config/owner/model/effect setup.

## Browser evidence and independent reproduction

S1-08's45-file manifest SHA256
`169168575cbc98da56b1f1464fde40c5b9b9979d4e952ee345536b765a8eddcc`
was recomputed; every file length/hash and its frozen asset sources match.
Inspected screenshots for375×812 and1280×900 in both locales, raw action and
network records, all four busy/current-locale cases and24 failure/reconciliation
cases. Rechecked failure hiding/focus/disabled state, current locale after delay,
actual GET prices after discard and seven contrast calculations. Keyboard
Tab/Shift-Tab/Space and corrected Enter records, visible focus, target sizes
(minimum68×46), reduced-motion records and the754-request index are retained.
No cross-engine substitution is used: ego Chromium150 supplied network/fault
checks; S1-08 CUA IAB supplied separate real-action screenshots, with internal
engine version unavailable. No screen-reader or non-specialist study occurred.

Independent current-binary reproduction used ego taskspace16 and a new tab at
http://127.0.0.1:51281, PID62701, with375×812 viewport. It observed initial
English, Chinese selection without requests, then price→3500, promotion→Customer,
fixed two-hit search and Reset→2500/Lead with Chinese retained and search hidden.
Each action emitted exactly one fixed POST. Guidance advanced to the recorded
scoping-call suggestion and explicitly denied that customer contact occurred.
Raw file `browser-happy.json` SHA256:
`817bc4f490d6b4b537f8fd7dcbbd8f765df2862b24fb07bb7df9d4fffd0251a2`.

At1280×900, browser-only Fetch interception discarded an actual200 price-action
response after server application. The UI hid stale business/search, disabled
four actions, cleared busy and focused the bilingual error. Reload displayed
3500 and default English. The first capture omitted Network.enable and is
preserved as partial `browser-failure.json`; it proves DOM/Fetch observations,
not request counts. A bounded corrected execution with Network enabled used
PID64723/session10175, printed http://127.0.0.1:51764. It captures the real POST,
server350000-cent response, failure, then exactly one state GET/no POST on
reload. `browser-failure-corrected.json` SHA256:
`876031a8a8dcad5b1a2e7f9a958fddee957d4f2ed2dc392c95ce6462199bcc67`.

One earlier happy-path evidence write failed because ego's working directory
was different; no file was produced. The entire path was repeated and saved
using an absolute path. A raw-evidence assertion subsequently detected the
missing Network domain above; neither probe was counted as passing evidence.
No candidate change was needed. The full S1-08 fault matrix was inspected,
not rerun or replaced with Node tests.

## Release two-root isolation

Required release test rerun passed23 executions:18 inherited domain/catalog,
three inherited helper/plumbing and two named isolation/sensitivity tests.
The two actual release children were PID63850 and63870, same port51592,
on macOS arm64. Both recorded44 ordered exchanges. Independently compared all
88 request/response file pairs and Content-Length framing, exact stdout/empty
stderr, differing pre-run inventories and each unchanged after inventory.
No byte normalization occurred. Tests assert distinct roots/PIDs, canary
absence, complete expected states, rejected-request nonmutation and reset on
new process; capture helper waits/reaps and joins both readers before output
is accepted. Sensitivity covers exact contents, removed entries, added files,
empty directories,0640→0600 modes, symlink/unexpected type and unavailable root.
Access time is intentionally excluded. Installed dirs source observations and
accepted capture-failure evidence were inspected; no OS wait failure was induced.

The unchanged test writes its artifacts under S1-10/fallback. Only the two
new run directories were relocated, without content changes, to reviewer/
isolation/run-a-1788924585472816000 and run-b-1788924586106571000;
`isolation/relocations.json` preserves original→destination mapping. Historical
accepted S1-10 runs were untouched. Rerun log SHA256:
`8672600cc894b1d81982ac9d37c7c77b4cf20548aa4fa081fd7baa0e0f60b120`.

## Live existing-product comparison

Protected ui.rs, old assets and Workspace diff is empty against pre-CLI
baseline610be5a. Ui variant/arm and existing command bodies remain unchanged.
Used the recorded real pre-CLI binary, not a rebuilt approximation:
SHA256 `195bf1e6053305a67ad0ab41d204f9c298c735974179618eb9f124548c73082f`.
Its S1-09 metadata records dev profile and leaf-only concurrent edits with no
CLI dependency on that leaf; it does not claim an originally clean worktree.

Established one seed under reviewer/legacy/seed using baseline CLI integrity
and baseline UI export behavior. Two seed exports establish a real signed
export event and capture a valid bundle. Stopped/reaped that process, then
copied the complete seed into distinct baseline/candidate roots and verified
identical file hashes before launch. Child-only HOME/XDG paths and cwd point
inside these synthetic roots. No real app data was read. Live workspace and
disclosure lists are empty; this is not nonempty disclosure coverage.

Both real `sovereign ui --port 51547 --no-open` commands served all12 required
route cases: five assets; state with disclosure/integrity fields; command-center;
workspace; export; valid and missing-bundle verify-export POSTs; unknown route.
Complete raw request/response bytes are retained. Status and ordered non-Date
headers matched exactly. All payload bytes matched except the specifically
allowed exported_at_unix token; parsed payload fields also matched after only
that field's exclusion. Every Date was valid canonical HTTP-date/current within
its recorded request window; both export times were integers/current in their
windows. Content-Length was independently checked and equal; no length delta
needed explanation. No timestamps were manufactured and no other normalization
occurred. UI stdout/stderr, integrity and `verify-export <same-file>` stdout/
stderr/exit all matched. Both CLI checks returned0; seed and both UI children
were terminated/reaped with recorded status and process windows.

`legacy/comparison.json` SHA256:
`eb1467f75653a1795dc430bc33fe36f1eed8cf48b2eae5cb9c0dc807c43e92b3`.
Established nonempty export/disclosure fixtures were separately inspected and
actually passed in the final workspace gate: draft_assistant_records_a_persistent_data_disclosure,
export_contains_state_and_verified_chain, integrity_check_binds_state_to_the_signed_chain,
verify_export_accepts_genuine_bundle_and_rejects_tampering and the existing
Workspace/command-center/stage1 suites. These tests use deterministic stand-ins;
they do not demonstrate a real model integration.

## Required gates and remaining limits

Reused `.harness/s1-10/fallback/full-gate.log` at the unchanged final source,
SHA256 `4f1a0609a18bef02b34fddf1ebeaa3854a1c70921554969a073f08bcd55e0446`:
404 Rust test executions,0failed/ignored, plus test_changed self-test, fmt,
workspace Clippy, old frontend TypeScript and file-size91files. Counts include
repeated imports and fixture plumbing, not404 independent product behaviors.
Empty library/doc-test targets are not used as acceptance tests.

Root's exact-HEAD additional logs under `.harness/s1-11/root-gates/` record
all-targets/all-features workspace Clippy, Node140passed/0failed/0skipped,
new crate-root TypeScript and cargo tree, all exit0. Reviewer additionally ran
leaf97passed/0failed/0ignored, release isolation23passed/0failed/0ignored,
`cargo build -p sovereign-cli --release --locked` and git diff --check, all0.
All required card commands are thus covered without duplicating the full gate.
`independent-checks.json` SHA256:
`4922806d22c470e56468ebc9b453768e0b32bbed35c0aa033ab9955a191456cc`.

Earlier S1-08 unchanged-source write-timeout watchdog flake remains unresolved
and preserved in its supervised report; the exact test/full rerun then passed,
and this review's private timeout test passed. No threshold was relaxed.
Original missing RED chronology and overwritten logs in earlier worker attempts
remain historical limits; later success does not reconstruct them. This review
ran on macOS arm64, not a fresh Linux/Windows browser/platform matrix. The
synchronous server can delay other clients within its bounded transport budgets.
No absolute security, owner authentication, model use, contacted customer,
persistence, recovery, protected real-data handling or full business flow is
claimed. Actual OwnerSession/ActiveV2 and RFC prerequisites remain construction
requirements for later integration; flags or fixture grants cannot replace them.

All reviewer tabs were closed; final ego inventory contains only root's retained
preview at7788. PID62701 was SIGTERM-stopped and observed absent after its launcher
parent exited (no wait exit code was available). Corrected child64723 was stopped
with Ctrl-C and its exec session returned exit1; PID absence is recorded. Legacy
and release-isolation children were reaped. Root preview PID95545 was untouched;
it retains the earlier accepted binary and is not represented as this review's
current-binary process. The two reviewer browser URLs above are stopped.

## Preview and next stage

From the repository root:

```sh
cargo run -p sovereign-cli -- playground --port 7788
```

Manually open the printed URL. If7788 is occupied, choose an unused port or0.
For the release executable:

```sh
cargo build -p sovereign-cli --release --locked
./target/release/sovereign playground --port 7788
```

Try Correct offer price, Mark as customer, Show reporting search and Reset in
English/Chinese. Language selection makes no request. Reset restores the fixed
fixture and preserves language; page reload GETs server state and resets local
language/search view; process restart restores the initial fixture. After a
failure reload before continuing, since an undelivered POST may have applied.
Ctrl-C stops the process. No developer test executable is needed for preview.

Five-consultant usability: **Target / not performed**. A future consented protocol
may have five independent consultants practice the thread, change language,
find guidance/search and Reset, recording completion, misunderstandings and
observed obstacles. No participants, timings, success rates, recruitment or
professional validation are invented or authorized here.

Nominate **S2-00 design** next. Its architect should freeze the complete synthetic
business state graph, separate experimental surface, exact interfaces/write paths
and tests while preserving this S1 leaf and legacy UI boundaries. Proposal,
review, delivery, billing, navigation and S3-M/S3-L integrations remain Target
until separately reviewed cards. This checkpoint does not dispatch S2 behavior
or complete the continuing full MVP goal.

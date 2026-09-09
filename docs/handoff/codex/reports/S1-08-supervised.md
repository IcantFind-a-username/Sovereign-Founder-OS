# S1-08 supervised execution

Accepted after two substantive Luna failures and Astra fallback; no source edits. Original claim
`0246b79de8bfdec732edda4b2ac6e7d9ce0fe406`, base5e3a7fb; revision1 card
`c50171cc49e63d3d3a74f53498450af515e109bf`. Full MVP remains active.

## Attempt1 — changes_requested

Worker `/root/s1_08_browser` Luna medium stopped. Independent reviewer
`/root/recovery_spec_card_review` Astra high rejected incomplete evidence.
Only observations.md has one1200x887 EN metric snapshot; network.json is empty,
no screenshots or full six-case observations. Claimed happy actions are not
proved by those artifacts. Observations blob9939bbcd4626789f04d562f7be13aacc94896b73;
empty network blobe69de29bb2d1d6434b8b29ae775ad8c2e48c5391. Preserved under
`.harness/s1-08/attempt1/`. Worker reports test server35201/:56506 stopped;
rootpreview46719/:7788 remains available.

## Actual capability diagnosis and revision2

Help Unknown helper does not prove API unavailable. Root directly verified ego
viewport Emulation succeeds (375x812 actualpageInfo), Fetch.enable/disable
succeed (event handling not yet proven), while screenshot calls timedout twice.
Browser.getVersion unsupported. CUA IAB screenshot returned a real page image;
native Mac app inventory was unavailable because screen is locked. This is a
local native-UI limit, not evidence that browser acceptance is impossible.
Probe record `.harness/s1-08/root-capability-probes.md`.

Cardrevision2 permits installed CUA browser screenshots supplementing ego
checks, with separately bound engine/state/URL/viewport/locale evidence. No
cross-engine substitution or omitted tests count as a pass. No production
relaxation/framework introduced. First failure is retained; second failure
would stop Luna and route to Astra. Continue actual six-case acceptance.

## Attempt2 — incomplete; escalate to Astra

Worker `/root/s1_08_attempt2` returned terminal incomplete at base `e344ed8`,
revision2 card blob `14820252b9954a4a731308bc42d12bc3c1e74e9d`.
No source edits. Partial observations cover bilingual viewport metrics and
search/reset; the network artifact summarizes startup GETs only. Delayed
response/busy, injected failures and full keyboard/focus/contrast remain
unproved. This is the second substantive failure; no third Luna attempt.

Preserved artifacts under `.harness/s1-08/attempt2/`: observations Git blob
`4720dc240c3ca1a931db0dea7d18de610392113b`, network Git blob
`20660054c3531e8354af7565eb15b9605f7e2052`. These partial summaries are not a
complete browser transcript. Worker reports its `:56994` child stopped;
root process inventory finds only the retained `:7788` preview (PID 95545).
No test_changed/cargo test process remained in that inventory. Worker-reported
Node/TypeScript/Rust passes lack bound full gate evidence here and do not
establish acceptance. Its 30-second observation timeout is not a passing gate.
Astra fallback must produce the original six groups, preserving both failures.

Root record gate completed exit 0: workspace tests/Clippy, fmt, file-size,
gate self-test and old frontend TypeScript; log
`.harness/s1-08/root-record-gate-full.log`. This verifies the unchanged source,
not missing browser cases. Fallback `/root/s1_08_fallback` dispatched with the
same source base/card and six-path ownership; root alone owns these records.

## Astra fallback — stopped candidate, independent review pending

`/root/s1_08_fallback` submitted at HEAD
`672cd236a064d9095fdff2eb5c93be1ef81000ee`. Source diff against `e344ed8`
is empty; no product/test edits. Release SHA256 remains
`8eaec9f6769e37697385cba4913d27dd7a4a57634815d5e657111771246237f3`.

Raw report: `.harness/s1-08/fallback/observations.md`, SHA256
`bc04fcb5128ac80c72c5a6077190d0cabeec74dc7636db50e40be52e7bd50c98`.
The 45-artifact manifest `.harness/s1-08/fallback/artifact-manifest.json` has
SHA256 `169168575cbc98da56b1f1464fde40c5b9b9979d4e952ee345536b765a8eddcc`;
root verified every listed file's size/hash. `candidate-freeze.json` binds
source files. These ignored raw files are local evidence, not Git-hosted assets.

Submitted observations cover all six named card groups: four viewport/locale
pairs, actual actions and guidance, genuine delayed POST responses, 28 busy/
failure scenarios, GET reconciliation, keyboard traversal and activation,
contrast/motion, and full CDP traffic. CUA screenshots are separately bound to
its own real actions; ego provides network/fault evidence. `network.json`
indexes the original event artifacts, including unsuccessful probes, and
includes genuine fresh-origin favicon startup. Earlier no-op clicks, malformed
Enter events and incorrect initial metadata probes remain labeled as failed
probes; the report identifies their corrected executions. No screen-reader or
five-consultant user study is claimed. CUA internal engine version was not
available and is not invented.

Node 140 and leaf Rust 94 tests passed; release build retained the accepted
binary. TypeScript/fmt/size/diff checks passed; the unchanged-source full gate
above is reused. Main child PID6216/session98314/:57896 and fresh-origin child
PID14830/session71120/:59309 were terminated and reaped (143); temporary tabs
closed. Root process inventory confirms only preview PID95545/:7788 remains.

Independent reviewer `/root/s1_08_review` is reviewing this exact candidate and
manifest. Submission is not acceptance; both Luna failures remain retained.
Next card, only after acceptance: S1-10A revision1,
blob `5a2ea0c3b71bb65db12a5535c49a3a588cead2c9`.

## Independent acceptance

`/root/s1_08_review` returned **accepted**, bound to HEAD
`672cd236a064d9095fdff2eb5c93be1ef81000ee`, revision2 card and the manifest above.
Reviewer checked all 44 action records, four busy and 24 fault records, decoded
real reconciliation bodies, inspected four-pair screenshots and keyboard
records, recomputed contrast, and reconstructed the 754-request index from
raw CDP events. Frozen source hashes and every artifact match. Source review
covers capabilities traffic alone cannot disprove. No missing card requirement
found; no additional gate needed for unchanged source. Next: S1-10A.

This accepts S1 browser behavior only, not S1 process isolation, user studies
or the full business/AI MVP. Preview command remains
`./target/release/sovereign playground --port 7788`; live root preview URL
http://127.0.0.1:7788 (verified HTTP200 during review). Root owns its lifecycle.

### Record-commit gate observation

The post-review record gate hit the existing
`server::tests::write_response_times_out_against_nonreading_peer` seven-second
watchdog once with unchanged source. Preserved full failure log:
`.harness/s1-08/acceptance-record-gate-failed-full.log`. Immediate exact-test
recheck passed in 5.01 seconds (`write-timeout-recheck.log`). Cause remains
unresolved; this is not a source fix or proof that intermittent timing cannot
recur. The controller reruns the full required gate before committing records.

Full gate recheck completed exit0; preserved as
`.harness/s1-08/acceptance-record-gate-green-full.log`. Initial watchdog failure
remains visible above; no tests were skipped or thresholds changed.

# S1-08 supervised execution

Incomplete. Two substantive Luna failures; no source edits. Original claim
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

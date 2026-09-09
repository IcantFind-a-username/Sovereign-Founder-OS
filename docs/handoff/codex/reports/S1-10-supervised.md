# S1-10 supervised execution

Accepted after two substantive Luna failures and Astra fallback. Revision3 card blob
`874f76dc1c1277a7f5cc31ee1f1414c9c04ba836`; attempt1 claim/base
`5b7111ba85222f4a7f575f191df126aa7dc34e97`.

## Attempt1 — changes requested

Stopped `/root/s1_10_worker`. Four candidate blobs: CLI test
`947ab32f42147ae0cb7834fcc0af15c50f0eb904`; fixture
`b2a8002a7688fe637eaca9830dd4747a406c196b`; CLI manifest
`3e04e87ab77529a572bd84b2cdd8c4794ff86a51`; lock
`f09db2287d3c1433bafee254fd72a17e2cac746b`. Accepted helper
`bf7b594e9bdd55305e04c34ea873ca1509127279` unchanged.

Independent Astra `/root/s1_10_review` requested:

1. Exact44response ordered transcript: missing invalid action, Host before
   Origin, final correction/promotion/GET only; fixed action bodies and step IDs.
2. Full expected state equality after every rejected request; assert both final
   actions and final corrected/promoted state; explicit Binitial==Ainitial.
3. Independent fixture per mutation with immediate before snapshot and specific
   content/mode/deleted-entry checks; don't pass from unrelated directory/mtime
   differences. Cover empty nested dirs, symlink/unexpected type and genuine
   root-independent I/O failure.
4. Restore private FakeRoot.work/TreeEntry fields, put fixture tests in its
   module; move function-local PermissionsExt import to module scope.
5. Actual raw request/response/stdout/stderr/root inventories and hashes bound
   to rerun binary/platform/PIDs/port/cleanup/canary/sensitivity, not just count.
   Observe installed dirs source and preserve atime exclusion.
6. Same-port startup failure needs named environmental diagnostic without
   treating every failure as confirmed collision or choosing another port.

Positive: actual Cargo releaseCLI, child-only HOME/XDG/cwd, complete raw/output
comparison and recursive snapshots. Reused domain/catalog oracle permitted;
23passed cases are18 inherited domain/catalog,3 inherited helper,2 new named.
Run macOS PIDs47414/47420 sameport65032,43entries. Binary SHA256
`0c259b55cb637f8a02a69e00ee51aa9c0937c6676b5826295aad0968ace2222f`.
Gate logs under `.harness/s1-10/attempt1/` support passing checks. Root preserved
new source snapshots and shared full gate as root-observed-full-gate.log.
The initial two-failure log WAS OVERWRITTEN; worker acknowledged it. It cannot
be reconstructed as original RED. No raw transcript/inventory artifacts were
supplied, so those completion claims are unproved.

Attempt2 preserves all candidates/failure history, samecard/fourfileownership,
30minutes/3targetedrepaircycles. Another substantive failure stops Luna and
routes to Astra fallback. FullMVP Goal continues; no isolation acceptance yet.

## Attempt2 — changes requested; Astra escalation

Stopped candidate at HEAD `d38bd82ce35be794b760f75d2ebe095d2edc73ca`:
CLI test `b58da5b86bfc435358357c3bd8df7a7370104df9`, fixture
`27c984e6accbe31d64eba97d705ec8ea9c4414d5`; manifest/lock/helper unchanged.
Independent Astra `/root/s1_10_review` rejected the remaining sensitivity
failure: content must assert recorded before/after bytes; removal must assert
file/directory absence; additions must assert specific file and empty nested
directory entries; mode must establish two distinct known modes and assert
recorded permissions. Whole-tree inequality can pass from unrelated mtime.
These explicit first-review requirements remained unfulfilled: second failure,
no third Luna attempt.

Reviewer independently verified the final 44-exchange evidence pair under
`.harness/s1-10/attempt2/run-a-1788924027234331000` and
`run-b-1788924027548707000`: macOS PIDs54779/54782, sharedport50002,
all raw request/response pairs and hashes match, correct Content-Length,
unchanged before/after inventories, unequal roots, exact stdout/empty stderr,
canary absence. Full state/error/final/restart assertions are now present;
fixture fields private. Binary SHA unchanged from attempt1.

Logs were initially written below apps/cli/.harness then moved to the approved
repository-root evidence directory; final source path is correct. Final
release test follows that correction and records23passes (18+3+2 as above).
The supplied full-gate summary predates that final edit and cannot bind final
candidate status. Root preserves this gap and will run a fresh baseline gate;
it does not reconstruct any missing historical log. Prior artifacts remain.
Astra fallback repairs sensitivity within the same four-file card scope and
must receive independent acceptance before S1-11.

Root fresh baseline gate69499 completed exit0 on the stopped attempt2 source;
root-baseline-gate.log/full.log preserve actual output. This newly bound pass
does not resolve sensitivity defects or alter earlier chronology.

## Astra fallback — independently accepted

Reviewer `/root/s1_10_review` accepted at HEAD
`c3aa18df99a185f2b934bbb330c0bf7e9d06d820`. Product commit
`1d4da2423ba386a722f02fd190857aea6a2e7a10`: four files,+462/-0.
Final CLI test blob `432f9e56ffa4568a2f6d4987737b4a8084041d62`, fixture
`5fd560e3a830200640af03069806c5ab02ccd654`; manifest/lock/helper unchanged
from reviewed attempt2. Only remaining test assertions and artifact location
changed during fallback. Specific bytes, key absence, nested empty-directory
representation and recorded0640→0600 permissions now prove sensitivity;
additional umask077 run passed.

Final release evidence under `.harness/s1-10/fallback/`:
run-a-1788924262465031000 (PID58093), run-b-1788924263048365000 (PID58101),
macOS arm64, sameport50525,44ordered exchanges each. Reviewer verified raw
request/response equality and saved SHA256 list, complete expected states,
seven error nonmutation checks, corrected/promoted final state and restart,
before/after inventories, unequal roots, exactstdout/emptystderr/canaryabsence.
BinarySHA256 remains
`0c259b55cb637f8a02a69e00ee51aa9c0937c6676b5826295aad0968ace2222f`.

Candidate/baseline copies and hashes, raw exchange/output/inventory files,
installed-dirs-observation.log, release-isolation.log, sensitivity-umask077.log
and all gate logs are preserved there. TEST_CHANGED_LOG directed full output
to fallback/full-gate.log; gate-summary.log records ALL GREEN at finalsource.
Session95718 completedexit0. Release23passes=18inheriteddomain/catalog+
3helper+2newnamed; requiredchecks postdate finaledits. Independent diffcheck
passed. No production/helper changes; both prior failures preserved. Access
time excluded; process evidence is complemented by source-level absence of
read capability, not proof of that property by itself. All owned processes
cleaned; rootpreview95545/:7788 untouched. Next S1-11 reviewer-only checkpoint,
then S2-00; fullMVP remains unfinished.

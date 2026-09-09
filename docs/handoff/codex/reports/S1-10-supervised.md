# S1-10 supervised execution

Incomplete; one substantive Luna failure. Revision3 card blob
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

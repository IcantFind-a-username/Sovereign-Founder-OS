# S1-07 supervised execution

Status: accepted after two substantive Luna failures and Astra fallback. Full MVP Goal
remains models-and-goals.md §13.1. Card revision1 blob
`66c8705ac5a71c7f5f9345cda9a60c927f5a36b3`, product base
`f65a6ed41d0893acd04a28b2dbabbd4a8b6745d9`, claim
`610be5a3dfa3d4895504e7cf6d10a1c4bea0448f`.

## Attempt1 — changes_requested

Actor `/root/s1_07_worker`, founder_worker (Luna medium), stopped.
Independent reviewer `/root/recovery_spec_card_review` (Astra high), read-only.
Four product files +318/-4, not the required six-file deliverable.

Candidate blobs: server `97ce844f607d758fa29ea6316c18bd596bc12ffd`;
lib `b0652acd65903a8818573e99f78433aabd08bef0`;
manifest `879057214f17e3f66e12e53f4db9362677f50957`;
lock `179139daf7a08a4430ba6291e7f7e30c9f116001`.
Raw preservation: `.harness/s1-07/attempt1.patch`,
`attempt1-server.rs.txt`, `attempt1-candidate.txt`.

Server copied the production fixture but omitted the mandatory terminal private
test module. Shared boundary rejects ServerTestModuleShape; asset tests reuse
this boundary, so this is not an old asset lib-shape conflict. Both integration
test files and all seven real TCP cases/private write tests are absent.
Restoring original lib module-separating comments permits rustfmt and token
boundary simultaneously. Frozen lib example omits those comments, a formatting
sample defect, but no grammar/gate change is necessary to implement the card.

Worker claimed 26 lib tests/clippy/tree/size/diff pass and crate/fmt failures;
no raw command logs were archived, so these remain unverified claims. No
preimplementation RED was captured. Subsequent tests must retain honest timing;
postimplementation sensitivity evidence cannot repair that historical gap.
Temporary unrelated wasm checksum edits were restored before this candidate.
No commit/push/merge by worker. Missing implementation is a substantive failure.

## Next action

Fresh Luna attempt2, same six-file card and 30min/three-cycle budget; cumulative
failure count1. Restore comments, implement terminal private tests and complete
real transport helper/tests, archive all commands. Independent review required.
Second substantive failure ends Luna retries and routes to Astra fallback.
Controller records elapsed time only where actual evidence exists; no inferred
historical timing or token usage. Draft S1-10/S1-11 docs are separate root work.

## Attempt2 — changes_requested; Luna limit reached

Actor `/root/s1_07_attempt2` (Luna medium), stopped. Independent read-only
reviewer `/root/recovery_spec_card_review` (Astra high) rejected six files
+718/-0 on unchanged claim HEAD. Production first305 lines remain exact fixture.
Blobs: server `93bdcf17889d2668ed26bc68970e9c0958e6213f`;
integration `b03229cee5c1c1503197dfb8ccc4624ff1fd38b9`;
helper `f865fb005bf7e8d810c1cbbcf05196b884565fdd`;
lib `807da2b70e502861148e54da4d1a9ea9faa28765`;
manifest/lock unchanged from attempt1. Preservation: `.harness/s1-07/attempt2-candidate.txt`,
`attempt2.patch`, three `attempt2-*.rs.txt`, full `attempt2-scoped-full.log`.
Latest final crate/clippy/fmt/tree/size/diff/scoped logs green. Earlier E0382
failure was observed by controller but worker overwrote those raw logs; do not
claim preserved full failure chronology. Baseline seven failures were child
startup parsing failures, not transport semantic RED.

Material gaps despite green commands:

1. Missing full DTO/assets/header/length/eighth-route assertions.
2. Combined duplicate/lowercase attack masks individual guards; missing port,
   Origin/Content-Type, target, OPTIONS, typed code/Allow/HEAD length cases.
3. Missing framing matrices and boundaries; if-let-Ok swallows failures.
4. Missing huge Content-Length with257 bytes and open sender, timely typed413/close.
5. Sleep then client disconnect can rescue server with no deadline; no observed
   408/body/trickle/shared-budget evidence.
6. Missing full four-action snapshots, real disconnect, changed-state restart,
   incomplete-body stop and bind conflict; pipeline checks only price.
7. Write watchdog fabricates TimedOut on timeout and thus passes when mechanism
   failed; actual write_response must finish itself, watchdog expiry fails.
8. Startup parse errors leak child before Drop guard; reader thread never joined.

Luna retries ended. Fresh Astra fallback owns same six paths, plus its raw logs;
no grammar/gate/production capability expansion. Reuse approved transport code
and repair test helper, rather than create a parallel harness. Independent
strong reviewer must assess fallback candidate before acceptance.

## Astra fallback — running

Actor `/root/s1_07_fallback`, founder_fallback Astra high, same six-file scope.
Controller observed `.harness/s1-07/fallback-baseline-helper-red.log`: one
`transport_helper_rejects_malformed_response_headers` test actually failed
because the old helper accepted malformed response headers. This is new helper
regression evidence, not reconstruction of missing original transport RED.
Candidate and complete checks remain pending; no acceptance claimed.

Fallback implementation decision: a pipelined second request left unread can
cause TCP ConnectionReset before the first response body is fully delivered.
Controller permits this test to accept only that specific error or an exact
prefix of the expected first response, followed by a complete GET equal to
the first-action oracle (second action must not apply). No other error masking
or relaxation of ordinary response assertions is permitted. This interprets
the existing one-request/close contract, without changing production behavior.
Test-client timeout workaround is fixed one-second waits within an eight-second
absolute budget; previous Darwin EINVAL diagnostics remain preserved, and their
root cause is not asserted from the workaround alone.

## Final independent acceptance

Reviewer `/root/recovery_spec_card_review` accepted the stopped six-file
candidate against claim610be5a. Root committed exactly those files as `b7caaadfa52371096292ab45dcba54604d1e8222`. Diff +1251/-0. No grammar or production-fixture relaxation.
Final blobs: server `7dc52db5cbfbc9251ab18360790b901135a3a863`,
integration `6e7a0bd7560782f785fcbf3b51299deb34025827`,
helper `0a56bf917bdfe94f6a7a41bb13ad4137d08f0be9`; lib/manifest/lock as
attempt2. All eight review gaps independently verified closed.

94 crate test executions:30lib(including4private transport),35boundary,
29integration(7socket,3helper,1child entry,18inherited domain/catalog).
No ignored tests. Package checks, clippy,fmt,tree,size,diff and full scoped
workspace tests/clippy/legacy frontend tsc passed. Full log
`.harness/s1-07/fallback-scoped-full.log`, SHA256 `42097b62be0e8b85a505b62f96c5dd94e53a63f9c3d1aa62711dfeae17e32e89`.
Raw malformed-header helper RED retained. Isolated read/write deadline
mutations to30s both fail at real watchdogs; candidate was never mutated.
See fallback-read-sensitivity-2.log and fallback-write-sensitivity.log.
This proves observed server behavior, not CLI/browser or full MVP completion.
Next: S1-09 real CLI wiring, then S1-08 browser. Two-root and final review
remain required. Two Luna failures and missing original RED stay preserved.

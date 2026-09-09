# S1-10A supervised execution

Accepted after two substantive Luna failures and Astra fallback. Frozen revision1 card blob
`5a2ea0c3b71bb65db12a5535c49a3a588cead2c9`; first claim/base
`47c4542b73c523a3964d889611a60e6150faee89`.

## Attempt1 — changes requested

Luna `/root/s1_10a_worker` stopped after two targeted repair cycles. Only
`crates/consultant-playground/tests/support/transport.rs` changed: +273/-55,
Git blob `54c14966f45cd6f535ebf4bb28f4fb709221eeed`. Preserved candidate and
patch in `.harness/s1-10a/attempt1/`. The candidate is not accepted or committed.

Independent Astra `/root/s1_10a_review` rejected this exact helper:

1. An unterminated startup line fragment is parsed and rejected before its
   remaining bytes arrive. Complete-line parsing must preserve canonical
   checks and the first4096-byte bound; prove split-read behavior.
2. try_wait errors skip kill and can proceed to blocked wait/reader joins;
   attempt all cleanup and preserve failures across repeated failed cleanup.
3. Only stderr overflow is tested. Startup publication races later stderr
   writes; a100ms sleep guesses overflow completion. Cover both pipe limits
   and deterministic synchronization with observable reaping/reader joins.
4. Future S1-10 consumer leaves start/start_mode unused. Use only the card's
   authorized method-specific allowances, with reuse comments, and verify
   the frozen import shape.

Missing-method RED is recorded in red-missing-method.log; it is compile-level,
not behavioral proof. Original9failures and subsequent repairs remain in logs.
Final transport suite32passed (two new behavioral cases plus child plumbing);
full gate ALL GREEN is in test_changed-final.log with full output copied to
`test_changed-full.log`. A green gate did not prove omitted requirements.
Original Response/raw HTTP helpers unchanged. Earlier manual probe PID21457
was terminated by worker; root confirmed it absent. Root preview untouched.

Attempt2 retains this candidate and failure count, same card/write set, bounded
30minutes/3targeted repair cycles. No third Luna attempt after another
substantive failure; then Astra fallback. FullMVP remains active.

## Attempt2 — changes requested; Astra escalation

Stopped Luna `/root/s1_10a_attempt2`, base
`432ced48a5635e3d2f90d7bc2725db74a907d45a`; helper blob
`37f4ff9c3090d23b2c708eb8b375de487892d844`, +340/-55. Same card and ownership.
Astra `/root/s1_10a_review` verified chunk-safe completed-line parsing, sticky
cleanup failure/kill after status-check error, and narrow method allowances.
It still rejected deterministic fixture synchronization: success publishes
startup before stderr; stdout overflow publishes startup before oversized
write. Immediate stop can terminate either write. Cleanup evidence remains
limited to error assertions, without concrete reader-join/reaping checks.

The worker supplied no attempt2 raw directory at terminal handoff. Root then
created `.harness/s1-10a/attempt2/` to preserve candidate-transport.rs and patch,
plus a copy named root-observed-shared-gate.log. Shared log contains33 transport
passes but is not sufficient to bind all worker claims to this candidate.
No historical behavioral RED is reconstructed. Worker-reported full gate and
inherited restart timing failure/rerun remain unverified pending original
execution evidence. Root process inventory found no remaining fixture/gate.

Two substantive failures reached: no third Luna attempt. Astra fallback must
finish the unchanged single-file contract, preserve earlier evidence, and
receive independent review. Root runs a fresh baseline gate separately before
handoff; it cannot prove missing historical chronology or absent race behavior.

Root baseline gate session51453 completed exit0 on unchanged helper37f4ff9;
raw summary/full output are root-baseline-gate.log and
root-baseline-gate-full.log under attempt2. This newly observed pass does not
resolve the independent synchronization finding or restore missing old logs.

## Astra fallback and independent acceptance

Stopped candidate `bf7b594e9bdd55305e04c34ea873ca1509127279`, reviewed against
HEAD `76ae0ced6cea5914bd25a9f628940bb81369bed9`. Product commit
`73cd9b2f98cce2af0954b05091e19c7e22e2cfb7` changes only the shared helper,
+484/-55,630lines. Relative to attempt2, fallback changes only its terminal
regression module; the previously reviewed parser/capture/lifecycle prefix
and original HTTP helpers are unchanged.

Astra `/root/s1_10a_review` independently **accepted** all four repaired
findings. Tests use bounded write acknowledgment and overflowing-reader
completion before stop. They verify exact stdout including pinned libtest
preamble, exact stderr, both65536/65537 limits, split/Interrupted reads,
post-start read errors, reader panics, sticky repeated cleanup, reaped child,
joined handles and unaffected pipe bytes. The frozen CLI import shape and
startup boundaries were inspected. OS-level wait errors were not injected;
this limit remains explicit.

Evidence: `.harness/s1-10a/fallback/handoff.txt`, saved candidate/hash/patch,
transport-final.log (32passed: two named behavioral tests plus one fixture
among inherited cases), test_changed.log/full.log (ALL GREEN at exact hash).
Other final checks passed. No inherited timing failure occurred this run.
A labeled truncation mutation produced one real behavioral RED and was restored;
this is test sensitivity, not reconstructed historical TDD. Earlier failed
attempts remain intact. All owned sessions and children stopped; preview95545
at7788 untouched. New helpers are test-only captured_child acknowledgment and
assert_reaped_and_joined, reusing std IO/TCP and existing capture lifecycle.

Next S1-10 consumes this accepted helper read-only. This does not yet prove
CLI root isolation or complete the fullMVP Goal.

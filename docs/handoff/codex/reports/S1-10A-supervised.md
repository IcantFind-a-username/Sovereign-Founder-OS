# S1-10A supervised execution

Incomplete; one substantive Luna failure. Frozen revision1 card blob
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

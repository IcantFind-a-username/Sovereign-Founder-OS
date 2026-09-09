# S1-02 supervised implementation evidence

Card revision1, blob76a00e5b165289e763150674818c75a33bd6cf8d;
source base b4935f00dfb06704454bf3b305a2c79ad1402073, claim6e23976.
Worker /root/s1_g01_luna (Luna medium), exact three-file scope: domain.rs,
leaf Cargo.toml, Cargo.lock. No other production files changed. Timing and
cumulative model budget are unavailable, not inferred or reset.

## Attempts and fallback

Attempt1 implemented the approved production DTO and dependencies, but tests
checked only three metadata values and partial types instead of all 15 values;
search read invariance and snapshots across session mutations were absent.
Controller requested changes; first substantive failure.

Attempt2 supplied full four-state JSON expectations, search invariance and
snapshot mutation checks, but reset each case after first normalizing it to the
same corrected/customer state. Second substantive failure. No third Luna retry.
Controller fallback changed only `let mut reset = modified` to `session`, so
all four original states actually exercise reset. Independent review required.
Existing action/reset tests and the exact JSON test cover the fixed initial
projection as well. No interface or production behavior was widened in fallback.

## Verification and limits

Final scoped gate with TEST_CHANGED_BASE=b4935f0 ran file-size, fmt, workspace
Clippy, workspace tests and frontend tsc; process64181 exited0 with ALL GREEN.
Full log: .harness/s1-02/integration/scoped.log. Cargo.lock changes only add
serde/serde_json to the leaf package; existing workspace versions are reused.

Worker's preserved red-green.log contains a locked-file update error, not a
semantic test failure. Original test-first execution is therefore unproven;
this record does not reconstruct or claim that history.

Controller subsequently copied the final domain into a temporary isolated test
crate using serde/serde_json. Changing only the company-name projection to an
incorrect string made the exact four-state JSON test fail (1 failed, exit101).
Restoring the exact final source made the same test pass (1 passed, exit0).
Process77569 completed successfully; logs mutation-red.log/restored-green.log
under .harness/s1-02/integration/. The temporary directory was removed and no
repository code was modified by the probe. This establishes detection of a
wrong projection, not retroactive test-first ordering.

Reused reachable_states and serde_json; no new product utilities or parser.
This is a detached Serialize-only read model, not a browser or real-model MVP.

## Independent acceptance

/root/recovery_spec_card_review accepted the final three-file candidate against
HEAD6e23976, domain blob bac8ff1597079d18ec7c7fd9a425fdd844c4d994. Review checked
actual code, full gate and the isolated mutation logs; the test-first evidence
limitation and both failures remain explicit. Next card is S1-G03 before S1-03.

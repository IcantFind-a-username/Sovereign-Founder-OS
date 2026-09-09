# S1-09 supervised execution

Accepted product commit `6e0976c630f091f46aa51f64c11044b5f2a2fc6d`; four files +106/-0. Claim
`8c350936bb8203c6a63d7bbf62fb2530c0d44047`, base961db08,
cardrevision1 `e95a914701abcdbc5cdf705cf21e9f0cafd69386`.
Luna worker `/root/s1_09_worker` attempt1 stopped; no substantive rejected
attempt. Root moved only cfg(test) cli_tests declaration to EOF to satisfy
contract, then rebuilt/rechecked. Independent Astra reviewer
`/root/recovery_spec_card_review` accepted corrected candidate and evidence.
Final main blob `8694b214c3a589567c18d0319451104a7482d5f7`.

## Behavior and checks

Adds only `sovereign playground --port <u16>`, default7788, direct leaf run
call, normal dependency/lock edge and three automated CLI test groups.
31 CLI tests pass (3new,28existing); Playground94 executions pass. Ui variant,
arm, old assets and Workspace unchanged; original and final Ui help stdout,
stderr and exit0 match exactly. Actual old binary is preserved at
`.harness/s1-09/baseline/sovereign`; metadata baseline-ui-help.json explains
its source context. No claim of live legacy-route comparison yet (S1-11).

Final root scoped gate exit0: workspace tests/clippy,fmt,file-size,gate-self-test,
oldfrontend tsc. Logs root-final-scoped-summary.log and
root-final-scoped-full.log (SHA256 `9c792dc9f3f55c034cbea37fa962bdc52978b0f95275ae9dfc82a92db6e30c14`),
under `.harness/s1-09/`. Release rebuild exit0, final binary SHA256
`8eaec9f6769e37697385cba4913d27dd7a4a57634815d5e657111771246237f3`.

## Actual process smoke

Root executed final release binary with port0, exact raw startupURL port56390.
All six assets match embedded files byte-for-byte with exact MIME/header/length.
GET complete synthetic state/catalog/teaching succeeded; CorrectOfferPrice
changed only expected price/guidance, POST equals subsequent GET; Reset returns
full original snapshot. A second bind to56390 exited1 with AddrInUse.
Ui help raw bytes/status match preserved baseline. Every curl used --max-time5.
Session59978 stopped via Ctrl-C, tool returned exit1; conflict process exited1.
Prior probes69160/85813 also stopped; no server is claimed running here.
Actual request/response files smoke-* and smoke-result.json bind final hash;
pre-eof-smoke retains the earlier binary run. Final binary was retested because
its hash changed after the root correction. This is manual terminal-tool smoke,
not a Rust test count and not browser acceptance.

## Evidence limitations and next step

Original worker-red-interface.log actually ran28old tests GREEN. Worker later
removed/restored the variant/arm and produced E0599 in
worker-red-interface-genuine.log; root observed this after release build.
That is a postimplementation interface probe, not original test-first RED.
Do not claim the missing chronology was recovered. Functional/capability/actual
runtime acceptance is supported by the independent checks above.

S1-10A/10/11 frozen cards split helper capture from root isolation; neither
claims implementation. Next is S1-08 actualbrowser, then10A→10→11. FullMVP
models-and-goals§13.1 remains incomplete.

To run this accepted increment:

```sh
cargo run -p sovereign-cli -- playground --port 7788
```

Open the printed URL; Ctrl-C stops it. Four fixed actions and bilingual
synthetic data only, no real model or real business persistence. Full business,
AI and legal flows remain subsequent stages.

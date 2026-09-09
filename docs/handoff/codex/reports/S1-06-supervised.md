# S1-06 supervised acceptance

Source base da47f15d167e4f41cdf2aa48182fbc6f0575ab98; first claim fe119f4;
card revision1 blob0b801adb59f596ccb7089b3c501e144e29d80f2a. Exact ten files
listed in S1-06; actor /root/s1_06_luna founder_worker Luna medium. No server,
browser or full MVP acceptance. Runtime token/time accounting unavailable.

## Attempt1 rejected

Stopped candidate +137/-0 in ten files (nine new). Actor admitted tsc failure
and incomplete busy/failure/render/bootstrap. Root independently ran Node3tests
EXIT0 and tsc EXIT2: .harness/s1-06/attempt1-node.log and attempt1-tsc.log.
No original preimplementation RED log was preserved; do not reconstruct it.
Independent /root/recovery_spec_card_review bound changes_requested to
fe119f46344cd5f746ecf9cad21ded0cc388b60c; app9f15acb1ae39837d9da2b6a4e18401d1792483b1,
UI6d874bae8e914ddab4650379afca5c0ac1dc619a, tests004c7036e2c08068eef603c04cc6820f405134ed.

Seven required fixes: nonexistent company_name_label crashes render (reuse
company_label), offer label/discovery next-step/title/search ARIA/suggested
button missing; success/failure busy and stale state not closed; incomplete
three-level metadata/full DTO/key validation; empty language controls/static
Language label and size/wrapping; absent real request/error Node matrices;
Rust marker tests instead of exact asset bytes; minified JS and broad/invalid
JSDoc producing five tsc errors. Current guidance field names were correct and
translate accepted arrays; root's earlier hypotheses about those were not
accepted findings. No new catalog keys or runtime dictionary is authorized.

## Attempt2 rejected

Claim b4fe8290da375c1fc1ec0b02f29ac7662db7bae3. Stopped candidate +145/-0.
Root independently reran Node3PASS and tscPASS; logs attempt2-node/tsc.log.
Independent reviewer returned changes_requested. App
0af6d87cae60d243fb8686943b41e110bc072640; UI64ead87720f447c71ecbaf3d2439a02a84c64b8b;
tests unchanged004c7036e2c08068eef603c04cc6820f405134ed.

Bootstrap export and failed-locale guard improved. But missing company key
still crashes first render; state/teaching empty objects pass the now-shallower
request validator; success never clears busy, failure keeps busy/loading;
all three Node tests unchanged, no request/error coverage; HTML/CSS and Rust
byte checks unchanged. Widening renderer locale to string cleared a type error
without implementing the frozen DTO/Locale typing. This is second substantive
failure despite green narrow checks. No third Luna attempt.

## Astra fallback accepted

Same ten-file ownership, no gates/HTTP/domain/catalog/dependencies/docs change.
Replace/refactor UI and tests to satisfy complete assets contract, real32keys
and DTO, one fetch point, all error paths, full busy/failure state, safe DOM,
static three HTML fallbacks, responsive layout and exact Rust mapping/bytes.
Preserve attempts and separate fallback logs. Independent review remains
required after stopped candidate, including explicit Node/checkJs runs;
existing scoped gate checks only the older CLI frontend tsc, so its green
alone does not prove this new frontend type check. Full Goal13.1 remains active.


Fresh /root/s1_06_fallback (Astra high) rewrote the implementation and tests
within the same ten files. New regression suite on the unchanged failed code
produced140 tests,41 pass/99 fail; fallback-red-node.log preserves that actual
RED. After repair140/140 pass, with final explicit new-frontend tsc also green.
All four named groups now substantively check catalog/locale, exact requests,
all11 errors and invalid response matrices, import effects and no retries.
Rust tests compare all six asset byte arrays to the exact files. Responsive,
keyboard/ARIA styling and DOM state logic received source review, not browser
acceptance. No generic framework/tool, dependency or extra runtime dictionary.

Stopped10-file diff+803/-0, nine new files. Key blobs: app
113ad093681ffb0a5ce8abe29d00676f641275ce; UI
c7f51fab80a69fe97e98393f63cc32184dcba789; Node tests
e96d3826862a634370e8754f6d4bd609cd3660be; assets.rs
5c00b3c3e04eadceb3bdfcd265945307c00e32b0. Rust formatting initially differed,
recorded fallback-fmt-initial.log, then corrected without production logic change.
Final Node/tsc/Rust/Clippy/fmt/diff/scoped and explicit workspace checks exited0.
An already launched explicit workspace test/Clippy duplicated scoped work;
no further rerun needed. Full scoped log fallback-scoped-full.log SHA256
e67f2b327af550800a3ad4a5002d6e0fe971d931651e2144f927dc4d345df809.

Independent /root/recovery_spec_card_review accepted exact candidate bound to
HEAD4ba15c8cc31e3ed8dca8ca29fd89f8c45c088d5d, closing seven prior gaps.
Two Luna failures remain recorded. Next nominated S1-G07 (transport gate).
Accepted scope is static UI/embedding only: actual network/browser interception,
375px rendering, keyboard and end-to-end operations remain unrun until server.
Full business/real-model/legal MVP Goal is not complete.

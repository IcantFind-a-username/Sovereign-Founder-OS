# S1-G04 supervised evidence

Card revision1, blob7c5454b46e1849bdbd43756ce9c1d63e269ea11d; source base
3feb5c0ffeea2539c069703791f37449a637f007, claim7123c6a. Fresh worker
/root/s1_g04_luna (Luna medium), exact four test-file scope. Timing/budget
unavailable; no reset or inferred accounting.

Attempt1 was rejected: teaching additions were admitted as a complete domain,
required Serialize derives were missing, and mutations often changed unrelated
text instead of the teaching method. First substantive failure.

Attempt2 repaired the domain composition and derives, but omitted the claimed
additions-only negative test and retained off-target Deserialize/fifth-action
mutations plus unchecked replacement operations. Second substantive failure;
no third Luna attempt. Controller fallback modified only the new teaching tests:
explicit additions-only rejection, 64 changed text positions, actual fifth unit
variant and ReportingHit Deserialize, and changed-source assertions for swapped/
extra entries. Formatting followed; original fixtures and production unchanged.

The catalog comparator is extracted from the existing implementation rather
than copied. Old domain/catalog alternatives remain; the new domain is the fixed
read-model fixture plus teaching additions, and the new catalog has exactly the
approved 32 keys with 64 ordinary-string slots. Existing source closure untouched.

Independent review found that .harness/s1-g04/red.log and red-attempt2.log
were temporary probes expecting an additions-only fragment to be accepted. They
do not prove the required full-fixture RED and remain preserved without that
claim. Original test-first execution is unproven.

Controller later copied the final tests/fixtures/lexer into an isolated temporary
test crate. The old boundary from HEAD7123c6a, with only two fixture-reference
constants added and no validation changes, rejected the full new domain with
DomainProductionShape (1 failed, exit101). Replacing it with the exact final
boundary passed the same acceptance test (1 passed, exit0). Process90861 exit0;
logs old-gate-complete-fixture-red.log and final-gate-complete-fixture-green.log
in the integration directory. Repository code never changed during this probe;
temporary files removed. This establishes detection, not retroactive ordering.

Final integration process59851 ran fmt, workspace tests,
workspace Clippy and scoped gates, all exit0. Logs under
.harness/s1-g04/integration/. No new parser, dependency, product input or IO.

Independent review follows before acceptance. Full Goal remains section13.1;
this gate only enables S1-04 implementation, not HTTP/browser or real AI.

## Independent acceptance

/root/recovery_spec_card_review accepted the final four-file candidate against
HEAD7123c6aa1ab289af80822cc1a03a202813317bb2 after checking grammar, derives,
shared catalog validation, mutation coverage, full gates and isolated old/new
comparison. Invalid original probes and both failures remain explicit. Next
is S1-04 product implementation; no HTTP or model capability is yet delivered.

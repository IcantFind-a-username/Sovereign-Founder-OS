# S1-G03 supervised execution record

Card revision1 blob b2f529ef84ac195b95e4d2d7e2d45c815fb1809f; base
 e31795e2d9f9c27edfdc56495eb87bfc486ed126, claim782f93d. Scope is the three
card-listed test files. Timing and cumulative model budget unavailable, not reset.

## Two Luna attempts

/root/s1_g01_luna attempt1 accepted arbitrary tokens at catalog text positions,
never enabled the new lib/source-closure pair, and implemented only partial
mutation tests. Controller rejected it, first substantive failure.

Attempt2 repaired ordinary-string/path checks and some mutations, but explicitly
reported source-closure pairing and mismatch/symlink coverage unfinished. Second
substantive failure, no third Luna attempt. Its claimed RED log at
.harness/s1-g03/attempt2/red.log actually contained a green run on inspection;
it remains preserved and is not evidence of a failing test.

## Astra fallback

/root/s1_g02_fallback repaired the same three files, then stopped edits. Shared
source_closure_boundary checks the real inventory and fixtures with exact old
2-file/old-lib and new 3-file/new-lib pairing. Unknown/missing files, inverse
pair mismatches, invalid roots and symlinks are covered. RustLexer and the
parameterized existing terminal-wrapper checker remain shared. The fixed
fixture identifies 54 ordinary-string data positions and pins all 27 keys;
code structure remains exact. Only local closure/fixture helpers were added,
using existing production_sources/source_root and ManifestFixture facilities.
No new parser, scanner, dependency or production capability.

The genuine pre-fix RED is .harness/s1-g03/fallback-red.log:
catalog_gate_preserves_wrapper_and_source_closure failed for the valid new
pair (Err Inventory instead of Ok). Final package suite passed 32 tests
(11 unit, 7 manifest, 14 source), and Clippy/diff checks passed. Fallback changed
three files total, +570/-26 including a 35-line catalog fixture. This does not
claim the original Luna test-first history was established.

Independent review and full integration results follow before acceptance.
Full Goal remains models-and-goals.md section13.1; catalog product code and
browser/real-model integration are not complete.

## Independent acceptance and integration

/root/recovery_spec_card_review accepted the final three-file candidate against
HEAD782f93d3c7633262dd1110daf198c2ee6802113a: boundary blob prefix db33fde,
source bd671b3, fixture4ce69b9. Reviewer checked actual source, contract, true
fallback RED and passing package evidence. Controller's process44157 ran full
workspace tests/Clippy followed by scoped file-size/fmt/adversarial/package gates,
all exit0. Logs: .harness/s1-g03/integration/. No candidate code changed during
review. G03 is accepted; next is the S1-03 product catalog.

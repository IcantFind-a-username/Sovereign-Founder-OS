# S1-G05 supervised acceptance

Source base 858329440fc482b6282bd27234c9df8337721bcd; claim 479cf89;
card revision1 blob c0f480b579066d2fd177c96720262fea08349e9b.
Actor /root/s1_g05_architecture used founder_architect (Astra high).
Candidate2 accepted after one independent review rejection; no Luna attempt.
Timing and cumulative token budget unavailable, not reset or inferred.

Four test files changed, +777/-17 including two new files. The gate admits the
four-file HTTP stage while retaining earlier stages. Shared source closure,
RustLexer, wrapper/path checks and fixtures were reused; no parser/framework
added. Private HTTP header/route/snapshot helpers were declared and approved.
HTTP contract revision2 permits only the two specific large-enum Clippy
annotations, retaining the approved inline fixed DTO rather than changing its
interface. Fixture and lib ordering were checked with rustfmt; fixture was
compiled and linted in isolation against accepted domain/catalog code.

Candidate1 was rejected because serde derived struct deserialization also
accepts sequences: ["CorrectOfferPrice"] changed the price and ["Reset"] reset
the state. Candidate2 requires the first non-JSON-whitespace byte to be an
object opener before direct serde parsing. Duplicate and escaped duplicate keys
remain rejected. Four gate mutations pin removal/weakening of this restriction.
The literal uses b"{"[0] within the existing lexer's supported quoted syntax;
the lexer was not expanded.

Original old-gate/new-fixture RED was preserved separately. Additional actual
semantic RED in .harness/s1-g05/semantic-object-red.log has three failing tests;
semantic-object-green.log has four passing tests. Illegal arrays and other
non-object values return400 with the complete following GET state unchanged;
legal objects, standard JSON whitespace and escapes work. These logs remain
separate from candidate1 checks; no RED was manufactured from a malformed probe.

Final scoped process72223 exited0 (.harness/s1-g05/repair-scoped.log).
Final integration process18987 exited0: cargo test --workspace --locked,
cargo clippy --workspace --all-targets --locked -- -D warnings,
cargo fmt --all --check, git diff --check. Workspace logs are under
.harness/s1-g05/integration-repair/; earlier integration logs are candidate1 only.

Independent /root/recovery_spec_card_review accepted the stopped candidate:
fixture8475ec9f40c394c38d824b4162f36a70a963d855,
HTTP tests7cf74fac, boundary2e874f35, sourceca198e54.
Next is fresh Luna medium S1-05, limited to src/http.rs and src/lib.rs.
This accepts an architectural scaffold; no running HTTP service/browser or
real-model integration is delivered yet. Full Goal section13.1 remains active.

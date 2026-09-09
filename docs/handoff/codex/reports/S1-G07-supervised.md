# S1-G07 supervised acceptance

Source base18844cb203faf606ab611b71012704459cc845c2; claim
8e1e8402d12322a2825e26109e1ecacdf8bcdc52; card revision1 blob
2423eef503e17aa242ba9657ebb7a1bbfe479eb0. Fresh actor
/root/s1_g07_architecture used founder_architect (Astra high), candidate1,
no independent review rejection. Runtime token/time accounting unavailable.

Five owned test files +1161/-10, two new. Full server fixture implements the
approved bounded std TCP/httparse transport, existing handler/assets integration
and exact response framing. Shared source closure preserves prior stages and
pairs server/public-run lib. Shared manifest validator pairs exact pinned
parser dependency and actual server stage in both directions. No product code,
workspace manifest/lock, lexer, old fixtures or HTTP/domain/UI changed.

Additional bounded helpers were declared and approved: respond_to_request,
read_timeout/read_failure, write_bytes/reason_phrase, shared cargo_metadata,
dependency_metadata and server fixture/mutation/symlink assertions. Existing
scanner, lexer, path/root checks, ManifestFixture and metadata parser reused;
no second general parser or directory scanner.

Complete valid next-stage fixture produced old-gate Inventory RED, followed by
GREEN using identical fixture. Logs red-valid-next-stage/green-valid-next-stage.
Isolated fixture compilation, all-target Clippy and fmt passed; reviewer
verified server fixture equality and accepted domain/catalog/http input bytes.
First four-matrix run failed two test setup expectations: wrong source extension
hit before symlink rejection, and git dependency lacking version hit req before
source rejection. Fixtures corrected without changing transport or guard order;
matrices.log retained, matrices-corrected.log four tests pass.

Package tests/Clippy, fmt/diff and full scoped gate passed. Full output
.harness/s1-g07/scoped-full.log SHA256
8726972f5307331d992349beacb6cdf605065dd1f49bc047fe9397d8155fa42f.
Scope included workspace tests/Clippy, fmt, file size, gate self-test and older
CLI frontend tsc. Detailed worker evidence remains .harness/s1-g07/worker-report.md.

Independent /root/recovery_spec_card_review accepted actual logic and hashes:
fixture97ce844f607d758fa29ea6316c18bd596bc12ffd;
tests0175b609b5576171b47d0cf8a365bdaa6fafb434;
boundary72dc785e0aa301df9a58fb82d366061d39ccabdd;
closurec543b61d09a8f4d3d7025eeb8932924b323e5e0a;
manifest30bf51a5f5365c122c6611b888e595bdd2d6d68f.
Next card S1-07 implements product server and real TCP tests. Acceptance here
is architecture only; no product server/browser run or complete MVP claim.

Root separately reviewed launch/browser design from /root/s1_launch_design:
accepted order07→09→08→10→11. CLI smoke uses existing terminal/HTTP tools,
no cross-crate helper framework. Exact four CLI files and six browser-repair
files are frozen in their cards; independent actual browser evidence follows
real CLI launch. Full Goal13.1 continues beyond fixed S1 Playground.

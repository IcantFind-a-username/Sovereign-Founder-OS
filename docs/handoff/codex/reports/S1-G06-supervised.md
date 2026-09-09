# S1-G06 supervised acceptance

Source base fecab4c824c90e986a81be802f1fefd402905b9e; claim
f3641ee2ee479368f7e6a72046d9f1b940b062aa; card revision1 blob
96b29a3fcdfd988035137f17b554bb994b725819. Fresh
/root/s1_g06_architecture used founder_architect (Astra high), candidate1,
no reviewed-candidate rejection. Token/time accounting unavailable.

Exact five test files: +807/-121 including three new files. Shared source
closure/error/fixture helpers were extracted first, preserving all15 source
tests. Source test shrank1164 to1054lines. A transient import wiring compile
error was fixed before extraction validation; it is not claimed as semantic RED.

Complete valid five-source/six-asset stage failed the old gate with Inventory
(.harness/s1-g06/red.log). Separate green.log shows3 named new gate tests pass.
Mapping, MIME/include paths, wrapper/tail/module attacks, missing/extra/nested
source/assets and root/file symlinks are tested using the same closure checker
as actual production inventory. Asset contents are not pinned as a duplicate
website. Source closure and fixed six-file inventory are shared, no second
parser/scanner. Reused RustLexer, exact wrapper/path checks, source_root,
production_sources, ManifestFixture and symlink fixtures. New bounded helpers
asset_inventory_boundary and asset_source_fixture were declared and approved.

The35-line Rust mapping fixture was independently compiled against byte-verified
accepted domain/catalog/http inputs and six placeholder assets;26 unit tests,
all-target Clippy and rustfmt passed. Fixture input hashes and location are in
fixture-accepted-inputs.txt and fixture-location.txt. Package tests/Clippy and
fmt/diff checks passed. Final scoped session81809 exited0 with workspace tests,
workspace Clippy, frontend tsc, fmt and size checks. Complete underlying output
is .harness/s1-g06/scoped-detail.log. No need to reinterpret isolated compilation
as browser acceptance; no actual website/server behavior is delivered here.

Stopped candidate hashes:
- boundary e04877d0853ed3ecf222d9072b19d3a5e2eb0bc9
- source 22bd8d3dd12d13769612b10f1684de6be461ac52
- assets tests 9f55c0e51e6e92da6ea4f210c73a38a57df2c95e
- shared helper 7ce9a972bbb34bed7816c502ac57a8b23881ae77
- asset fixture e3ebf345ccffe5b511a1920a6146898f327f4048

Independent /root/recovery_spec_card_review accepted the exact stopped candidate
bound to claim f3641ee and the five hashes above. Next nominated card S1-06. Full Goal section13.1 remains
unfinished; browser UI, transport and full business/model/legal integration
must still be implemented and actually exercised.

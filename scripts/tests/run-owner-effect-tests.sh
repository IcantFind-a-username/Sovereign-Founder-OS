#!/usr/bin/env bash
# Self-test for scripts/run-owner-effect-tests.sh.
#
# A checked runner is only worth something if it actually rejects the shapes it
# claims to reject, and every one of those shapes is a transcript that reads as
# success: zero tests executed, a target skipped for a missing feature, a
# filter that matched nothing, a failure with the wrong diagnostic. So each
# case here feeds the real runner a real transcript through a stub `cargo` on
# PATH — the runner's own code path, not a copy of its logic — and only the one
# case that genuinely passed may exit 0.
#
# Portability: bash 3.2 (stock macOS).
set -euo pipefail

if [ -z "${BASH_VERSINFO:-}" ] || [ "${BASH_VERSINFO[0]}" -lt 3 ] ||
  { [ "${BASH_VERSINFO[0]}" -eq 3 ] && [ "${BASH_VERSINFO[1]}" -lt 2 ]; }; then
  echo "unsupported bash: need 3.2 or newer, got ${BASH_VERSION:-non-bash shell}" >&2
  exit 3
fi

RUNNER="$(cd "$(dirname "$0")/.." && pwd)/run-owner-effect-tests.sh"
[ -x "$RUNNER" ] || { echo "missing $RUNNER" >&2; exit 1; }

completed=0
on_exit() {
  status=$?
  if [ "$completed" -eq 0 ] && [ "$status" -eq 0 ]; then
    echo "run-owner-effect-tests self-test: INCOMPLETE — exited before finishing" >&2
    exit 1
  fi
}
trap on_exit EXIT

WORK=$(mktemp -d)
cleanup() { rm -rf "$WORK"; }
trap 'cleanup; on_exit' EXIT

checked=0
failed=0

# Build a stub cargo that prints $2 and exits $3, then run the runner against a
# one-row manifest with the given profile.
scenario() { # <name> <transcript> <exit> <expect: pass|fail> [extra runner args...]
  name="$1"; transcript="$2"; code="$3"; expect="$4"; shift 4
  checked=$((checked + 1))

  bin="$WORK/$checked/bin"
  mkdir -p "$bin"
  {
    echo '#!/usr/bin/env bash'
    echo "cat <<'TRANSCRIPT'"
    printf '%s\n' "$transcript"
    echo 'TRANSCRIPT'
    echo "exit $code"
  } > "$bin/cargo"
  chmod +x "$bin/cargo"

  manifest="$WORK/$checked/manifest.tsv"
  printf 'task\tpackage\ttarget\tprofile\ttest_name\n' > "$manifest"
  printf '9\tsovereign-stub\tstub_target\tno-default-features\ta_stub_test\n' >> "$manifest"

  set +e
  PATH="$bin:$PATH" OWNER_EFFECT_MANIFEST="$manifest" \
    "$RUNNER" --all "$@" > "$WORK/$checked/out" 2>&1
  status=$?
  set -e

  if [ "$expect" = "pass" ] && [ "$status" -ne 0 ]; then
    echo "FAIL  $name: expected the runner to accept this, it exited $status"
    sed 's/^/        /' "$WORK/$checked/out"
    failed=1
  elif [ "$expect" = "fail" ] && [ "$status" -eq 0 ]; then
    echo "FAIL  $name: the runner accepted a transcript it must reject"
    sed 's/^/        /' "$WORK/$checked/out"
    failed=1
  else
    echo "ok    $name"
  fi
}

# The one transcript that genuinely ran the test and passed.
scenario "a real passing run is accepted" \
'running 1 test
test a_stub_test ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s' \
  0 pass

# Zero executed. Cargo exits 0 and prints "ok" — the failure this whole runner
# exists for.
scenario "zero executed tests is refused" \
'running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out; finished in 0.00s' \
  0 fail

# A filter that matched nothing looks identical to a clean run.
scenario "a filtered-to-nothing run is refused" \
'running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s' \
  0 fail

# A target skipped for a missing required-feature: exit 0, no tests, no result.
scenario "a target skipped for required-features is refused" \
'   Compiling sovereign-stub v0.1.0
warning: skipping due to unsatisfied required-features: `owner-effect-fixture`' \
  0 fail

# Cargo could not honour the profile at all.
scenario "an unknown feature is refused" \
'error: none of the selected packages contains these features: owner-effect-fixture
the package sovereign-stub does not have the feature owner-effect-fixture' \
  101 fail

# Cargo could not find the target named in the manifest.
scenario "an unknown target is refused" \
'error: no target named `stub_target` in default-run packages' \
  101 fail

# No result line at all: the command produced nothing to judge.
scenario "output with no test result line is refused" \
'   Compiling sovereign-stub v0.1.0
    Finished test profile in 0.4s' \
  0 fail

# A genuine failure carrying the diagnostic the task expects: a valid RED.
scenario "a RED with the expected diagnostic is accepted" \
'running 1 test
test a_stub_test ... FAILED

failures:
---- a_stub_test stdout ----
assertion failed: the owner was not authenticated

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out' \
  101 pass --red "the owner was not authenticated"

# A failure for some other reason must not be read as the expected RED.
scenario "a failure with the wrong diagnostic is refused" \
'running 1 test
test a_stub_test ... FAILED

failures:
---- a_stub_test stdout ----
assertion failed: something entirely unrelated broke

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out' \
  101 fail --red "the owner was not authenticated"

# A test that passes while a RED was expected is not a RED.
scenario "a pass where a RED was expected is refused" \
'running 1 test
test a_stub_test ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out' \
  0 fail --red "the owner was not authenticated"

# The final command must declare the manifest's complete feature set.
scenario "an incomplete --require-profile set is refused" \
'running 1 test
test a_stub_test ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out' \
  0 fail --require-profile fault-injection

scenario "a matching --require-profile set is accepted" \
'running 1 test
test a_stub_test ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out' \
  0 pass --require-profile no-default-features

echo
if [ "$checked" -lt 10 ]; then
  echo "run-owner-effect-tests self-test: only $checked scenarios ran; the suite is not intact" >&2
  exit 1
fi
if [ "$failed" -ne 0 ]; then
  echo "run-owner-effect-tests self-test: FAILED" >&2
  exit 1
fi

completed=1
echo "run-owner-effect-tests self-test: OK — $checked scenarios"

#!/usr/bin/env bash
# Self-test for scripts/run-owner-effect-regression.sh.
#
# The wrapper has two jobs that pull in opposite directions: refuse transcripts
# that only look like success, and get out of the way of a genuine failure by
# preserving its exit status. A self-test that checked "did it exit nonzero"
# would conflate them, so each case here asserts *which* outcome happened —
# the wrapper's own refusal, a preserved underlying status, or a clean pass.
#
# Transcripts are fed through a stub `cargo` on PATH, so the real script runs.
#
# Portability: bash 3.2 (stock macOS).
set -euo pipefail

if [ -z "${BASH_VERSINFO:-}" ] || [ "${BASH_VERSINFO[0]}" -lt 3 ] ||
  { [ "${BASH_VERSINFO[0]}" -eq 3 ] && [ "${BASH_VERSINFO[1]}" -lt 2 ]; }; then
  echo "unsupported bash: need 3.2 or newer, got ${BASH_VERSION:-non-bash shell}" >&2
  exit 3
fi

WRAPPER="$(cd "$(dirname "$0")/.." && pwd)/run-owner-effect-regression.sh"
[ -x "$WRAPPER" ] || { echo "missing $WRAPPER" >&2; exit 1; }

completed=0
WORK=$(mktemp -d)
on_exit() {
  status=$?
  rm -rf "$WORK"
  if [ "$completed" -eq 0 ] && [ "$status" -eq 0 ]; then
    echo "run-owner-effect-regression self-test: INCOMPLETE — exited before finishing" >&2
    exit 1
  fi
}
trap on_exit EXIT

checked=0
failed=0

# <name> <transcript> <cargo exit> <expected wrapper exit> [command args...]
scenario() {
  name="$1"; transcript="$2"; code="$3"; want="$4"; shift 4
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

  set +e
  PATH="$bin:$PATH" "$WRAPPER" -- "$@" > "$WORK/$checked/out" 2>&1
  got=$?
  set -e

  if [ "$got" -ne "$want" ]; then
    echo "FAIL  $name: expected exit $want, got $got"
    sed 's/^/        /' "$WORK/$checked/out"
    failed=1
  else
    echo "ok    $name (exit $got)"
  fi
}

PASSING='running 3 tests
test a ... ok
test b ... ok
test c ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s'

# A real run that passed.
scenario "a real passing run exits 0" "$PASSING" 0 0 \
  cargo test -p sovereign-stub --no-default-features

# A real failure must reach the caller unchanged, not be replaced by the
# wrapper's own status. This is the case the wrapper must not swallow.
scenario "a genuine failure preserves the underlying exit status" \
'running 2 tests
test a ... ok
test b ... FAILED

test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out' \
  101 101 cargo test -p sovereign-stub --no-default-features

# Zero executed: cargo exits 0 and says "ok".
scenario "zero executed tests is the wrapper's own failure" \
'running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 9 filtered out; finished in 0.00s' \
  0 1 cargo test -p sovereign-stub --no-default-features

# Several binaries, none of which ran anything.
scenario "zero across every binary is refused" \
'test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 7 filtered out; finished in 0.00s' \
  0 1 cargo test --workspace --no-default-features

# A target skipped for a missing required-feature prints no result at all.
scenario "a skipped required-features target is refused" \
'   Compiling sovereign-stub v0.1.0
warning: skipping due to unsatisfied required-features: `owner-effect-fixture`' \
  0 1 cargo test -p sovereign-stub --no-default-features

# Cargo could not honour the requested feature set.
scenario "an unknown feature is refused" \
'error: none of the selected packages contains these features: owner-effect-fixture' \
  101 1 cargo test -p sovereign-stub --no-default-features --features owner-effect-fixture

# The command did not declare its feature set: refused before running.
scenario "a command without --no-default-features is refused before it runs" \
  "$PASSING" 0 2 cargo test -p sovereign-stub

# Only `cargo test` may be wrapped.
scenario "a non-cargo-test command is refused" "$PASSING" 0 2 \
  cargo build --no-default-features
scenario "a non-cargo command is refused" "$PASSING" 0 2 \
  make --no-default-features

echo
if [ "$checked" -lt 8 ]; then
  echo "run-owner-effect-regression self-test: only $checked scenarios ran" >&2
  exit 1
fi
if [ "$failed" -ne 0 ]; then
  echo "run-owner-effect-regression self-test: FAILED" >&2
  exit 1
fi

completed=1
echo "run-owner-effect-regression self-test: OK — $checked scenarios"

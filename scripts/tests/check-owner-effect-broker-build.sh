#!/usr/bin/env bash
# Self-test for scripts/check-owner-effect-broker-build.sh.
#
# The real gate spends two clean CARGO_TARGET_DIR builds plus
# broker_bootstrap. These scenarios never do that: they put a stub `cargo`
# first on PATH and feed the real script the transcripts and artefacts it
# claims to judge. Only the one complete passing shape may exit 0.
#
# Portability: bash 3.2 (stock macOS).
set -euo pipefail

if [ -z "${BASH_VERSINFO:-}" ] || [ "${BASH_VERSINFO[0]}" -lt 3 ] ||
  { [ "${BASH_VERSINFO[0]}" -eq 3 ] && [ "${BASH_VERSINFO[1]}" -lt 2 ]; }; then
  echo "unsupported bash: need 3.2 or newer, got ${BASH_VERSION:-non-bash shell}" >&2
  exit 3
fi

ROOT=$(cd "$(dirname "$0")/../.." && pwd) || exit 1
cd "$ROOT" || exit 1
GATE="$ROOT/scripts/check-owner-effect-broker-build.sh"
[ -x "$GATE" ] || { echo "missing $GATE" >&2; exit 1; }

write_release_stub() { # <path> <kind: clean|help-leak|binary-leak>
  dest=$1
  kind=$2
  {
    echo '#!/bin/sh'
    if [ "$kind" = "help-leak" ]; then
      echo 'if [ "$1" = "--help" ]; then'
      echo '  echo "Usage: sovereign"'
      echo '  echo "__owner-effect-broker"'
      echo '  exit 0'
      echo 'fi'
    elif [ "$kind" = "binary-leak" ]; then
      echo '# run_owner_effect_fixture_broker'
      echo 'if [ "$1" = "--help" ]; then'
      echo '  echo "Usage: sovereign"'
      echo '  exit 0'
      echo 'fi'
    else
      echo 'if [ "$1" = "--help" ]; then'
      echo '  echo "Usage: sovereign"'
      echo '  exit 0'
      echo 'fi'
    fi
    echo 'echo "error: unrecognized subcommand" >&2'
    echo 'exit 2'
  } >"$dest"
  chmod +x "$dest"
}

# Cargo stubs call back into this file to emit a release-shaped fake binary
# without duplicating the leak strings in two places.
if [ "${1:-}" = "--write-release-stub" ]; then
  write_release_stub "$2" "$3"
  exit 0
fi

completed=0
on_exit() {
  status=$?
  if [ "$completed" -eq 0 ] && [ "$status" -eq 0 ]; then
    echo "check-owner-effect-broker-build self-test: INCOMPLETE — exited before finishing" >&2
    exit 1
  fi
}
trap on_exit EXIT

WORK=$(mktemp -d "${TMPDIR:-/tmp}/sfo-broker-build-selftest.XXXXXX")
cleanup() { rm -rf "$WORK"; }
trap 'cleanup; on_exit' EXIT

checked=0
failed=0

write_cargo_stub() { # <path> <mode>
  dest=$1
  mode=$2
  {
    echo '#!/usr/bin/env bash'
    echo "mode='$mode'"
    echo 'set -e'
    echo 'is_test=0'
    echo 'is_release=0'
    echo 'is_fixture=0'
    echo 'for arg in "$@"; do'
    echo '  case "$arg" in'
    echo '  test) is_test=1 ;;'
    echo '  --release) is_release=1 ;;'
    echo '  owner-effect-fixture) is_fixture=1 ;;'
    echo '  esac'
    echo 'done'
    echo 'if [ -z "${CARGO_TARGET_DIR:-}" ]; then'
    echo '  echo "stub cargo: CARGO_TARGET_DIR is unset" >&2'
    echo '  exit 2'
    echo 'fi'
    echo 'if [ "$is_test" -eq 1 ]; then'
    echo '  case "$mode" in'
    echo '  zero-tests)'
    echo "    cat <<'T'"
    echo 'running 0 tests'
    echo ''
    echo 'test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out; finished in 0.00s'
    echo 'T'
    echo '    exit 0'
    echo '    ;;'
    echo '  skipped-features)'
    echo "    cat <<'T'"
    echo 'warning: skipping due to unsatisfied required-features: `owner-effect-fixture`'
    echo 'T'
    echo '    exit 0'
    echo '    ;;'
    echo '  ignored-tests)'
    echo "    cat <<'T'"
    echo 'running 2 tests'
    echo 'test a_parent_starts_a_broker_and_becomes_its_supervisor ... ok'
    echo 'test default_binary_has_no_hidden_broker_mode_or_symbols ... ignored'
    echo ''
    echo 'test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.01s'
    echo 'T'
    echo '    exit 0'
    echo '    ;;'
    echo '  no-result)'
    echo "    cat <<'T'"
    echo '    Finished test profile in 0.40s'
    echo 'T'
    echo '    exit 0'
    echo '    ;;'
    echo '  *)'
    echo "    cat <<'T'"
    echo 'running 6 tests'
    echo 'test a_parent_starts_a_broker_and_becomes_its_supervisor ... ok'
    echo 'test a_valid_bootstrap_with_no_supervisor_times_out_having_claimed_nothing ... ok'
    echo 'test a_valid_parent_over_a_product_root_claims_nothing ... ok'
    echo 'test default_binary_has_no_hidden_broker_mode_or_symbols ... ok'
    echo 'test direct_hidden_broker_without_valid_stdin_exits_before_examining_a_root ... ok'
    echo 'test direct_valid_bootstrap_rejects_a_product_root_before_opening_anything ... ok'
    echo ''
    echo 'test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s'
    echo 'T'
    echo '    exit 0'
    echo '    ;;'
    echo '  esac'
    echo 'fi'
    echo 'if [ "$mode" = "build-fails" ] && [ "$is_release" -eq 0 ]; then'
    echo '  echo "error: stub fixture build failed" >&2'
    echo '  exit 101'
    echo 'fi'
    echo 'if [ "$is_release" -eq 1 ]; then'
    echo '  mkdir -p "$CARGO_TARGET_DIR/release"'
    echo '  case "$mode" in'
    echo '  help-leak) kind=help-leak ;;'
    echo '  binary-leak) kind=binary-leak ;;'
    echo '  *) kind=clean ;;'
    echo '  esac'
    echo "  \"$ROOT/scripts/tests/check-owner-effect-broker-build.sh\" --write-release-stub \"\$CARGO_TARGET_DIR/release/sovereign\" \"\$kind\""
    echo '  exit 0'
    echo 'fi'
    echo 'mkdir -p "$CARGO_TARGET_DIR/debug"'
    echo 'if [ "$mode" = "missing-bin" ]; then'
    echo '  exit 0'
    echo 'fi'
    echo ': > "$CARGO_TARGET_DIR/debug/sovereign"'
    echo 'chmod +x "$CARGO_TARGET_DIR/debug/sovereign"'
    echo 'if [ "$mode" = "sibling" ]; then'
    echo ': > "$CARGO_TARGET_DIR/debug/sovereign-broker"'
    echo '  chmod +x "$CARGO_TARGET_DIR/debug/sovereign-broker"'
    echo 'fi'
    echo 'exit 0'
  } >"$dest"
  chmod +x "$dest"
}

scenario() { # <name> <mode> <expect: pass|fail>
  name=$1
  mode=$2
  expect=$3
  checked=$((checked + 1))

  bin="$WORK/$checked/bin"
  mkdir -p "$bin"
  write_cargo_stub "$bin/cargo" "$mode"

  set +e
  PATH="$bin:$PATH" BROKER_BUILD_KEEP= \
    "$GATE" >"$WORK/$checked/out" 2>&1
  status=$?
  set -e

  if [ "$expect" = "pass" ] && [ "$status" -ne 0 ]; then
    echo "FAIL  $name: expected the gate to accept this, it exited $status"
    sed 's/^/        /' "$WORK/$checked/out"
    failed=1
  elif [ "$expect" = "fail" ] && [ "$status" -eq 0 ]; then
    echo "FAIL  $name: the gate accepted a shape it must reject"
    sed 's/^/        /' "$WORK/$checked/out"
    failed=1
  else
    echo "ok    $name"
  fi
}

# Static contract: the gate must still be the plan's Task 4 command, not a
# renamed or emptied cousin. These are fixed strings so a rewrite that drops
# one is a failed self-test, not a silent narrower check.
static_require() { # <needle>
  checked=$((checked + 1))
  if ! grep -Fq -- "$1" "$GATE"; then
    echo "FAIL  gate no longer contains: $1"
    failed=1
  else
    echo "ok    static: $1"
  fi
}

static_require 'CARGO_TARGET_DIR'
static_require 'cargo build'
static_require '--features owner-effect-fixture'
static_require '--test broker_bootstrap'
static_require '--test-threads=1'
static_require '--no-default-features --release'
static_require 'trap on_exit EXIT'
static_require 'completed=1'

if ! grep -v '^[[:space:]]*#' "$GATE" | grep -Eq 'trap[[:space:]]+[^#]*[[:space:]]EXIT'; then
  echo "FAIL  no EXIT trap guards the premature-exit path"
  failed=1
fi
checked=$((checked + 1))
echo "ok    static: EXIT trap is live"

scenario "a complete same-target run is accepted" happy pass
scenario "a missing fixture binary is refused" missing-bin fail
scenario "a sibling broker artifact is refused" sibling fail
scenario "zero executed tests is refused" zero-tests fail
scenario "a required-features skip is refused" skipped-features fail
scenario "ignored tests are refused" ignored-tests fail
scenario "output with no test result line is refused" no-result fail
scenario "release help that names the hidden mode is refused" help-leak fail
scenario "a release binary that contains the broker entrypoint is refused" binary-leak fail
scenario "a failed fixture build is refused" build-fails fail

# An early `exit 0` must not read as a pass — the same false-green the
# completion marker exists to stop.
checked=$((checked + 1))
ANCHOR='# ---- fixture debug image'
if ! grep -q "$ANCHOR" "$GATE"; then
  echo "FAIL  aborted-gate-never-exits-zero: anchor '$ANCHOR' is gone"
  failed=1
else
  poisoned="$WORK/poisoned.sh"
  awk -v anchor="$ANCHOR" '
    index($0, anchor) == 1 && !done {
      print
      print "exit 0"
      done = 1
      next
    }
    { print }
  ' "$GATE" >"$poisoned"
  chmod +x "$poisoned"
  set +e
  PATH="$WORK/poison-bin:$PATH" "$poisoned" >"$WORK/poisoned.out" 2>&1
  status=$?
  set -e
  if [ "$status" -eq 0 ]; then
    echo "FAIL  a gate that ran no checks exited 0 — this is the false green"
    sed 's/^/        /' "$WORK/poisoned.out"
    failed=1
  else
    echo "ok    aborted-gate-never-exits-zero (exit $status)"
  fi
fi

echo
if [ "$checked" -lt 18 ]; then
  echo "check-owner-effect-broker-build self-test: only $checked checks ran; the suite is not intact" >&2
  exit 1
fi
if [ "$failed" -ne 0 ]; then
  echo "check-owner-effect-broker-build self-test: FAILED" >&2
  exit 1
fi

completed=1
echo "check-owner-effect-broker-build self-test: OK — $checked checks"

#!/usr/bin/env bash
# Same-image fixture broker build gate (owner-session plan Task 4).
#
# The broker is a hidden mode of the `sovereign` binary, not a sibling
# executable. A workspace `target/` that already holds yesterday's fixture
# build can hide a newly introduced extra artifact, or let
# `CARGO_BIN_EXE_sovereign` point at an image this invocation did not just
# produce. This script allocates a fresh CARGO_TARGET_DIR so both claims
# are about this run.
#
# After the fixture debug image exists with no sibling broker artifact, it
# runs `broker_bootstrap` against that same target directory and refuses a
# zero-test, skipped, or ignored transcript. A second fresh directory then
# clean-builds `--no-default-features --release` and checks help/symbols:
# a shipped image must not carry the hidden mode or the broker entrypoint.
#
# Portability: bash 3.2 (stock macOS).
# Honesty (backlog lesson 8): EXIT-trap completion marker; a run that
# built or tested nothing fails rather than printing OK.
set -euo pipefail

if [ -z "${BASH_VERSINFO:-}" ] || [ "${BASH_VERSINFO[0]}" -lt 3 ] ||
  { [ "${BASH_VERSINFO[0]}" -eq 3 ] && [ "${BASH_VERSINFO[1]}" -lt 2 ]; }; then
  echo "unsupported bash: need 3.2 or newer, got ${BASH_VERSION:-non-bash shell}" >&2
  exit 3
fi

ROOT=$(cd "$(dirname "$0")/.." && pwd) || exit 1
cd "$ROOT" || exit 1

HIDDEN_MODE="__owner-effect-broker"
BROKER_ENTRY="run_owner_effect_fixture_broker"

completed=0
checked=0
FIXTURE_TARGET=""
RELEASE_TARGET=""

on_exit() {
  status=$?
  if [ -z "${BROKER_BUILD_KEEP:-}" ]; then
    if [ -n "$FIXTURE_TARGET" ] && [ -d "$FIXTURE_TARGET" ]; then
      rm -rf "$FIXTURE_TARGET"
    fi
    if [ -n "$RELEASE_TARGET" ] && [ -d "$RELEASE_TARGET" ]; then
      rm -rf "$RELEASE_TARGET"
    fi
  fi
  if [ "$completed" -eq 0 ] && [ "$status" -eq 0 ]; then
    echo "check-owner-effect-broker-build: INCOMPLETE — exited before finishing" >&2
    exit 1
  fi
}
trap on_exit EXIT

fail() {
  echo "check-owner-effect-broker-build: FAIL — $1" >&2
  exit 1
}

# Locate the cargo bin image Cargo writes for `--bin sovereign`.
sovereign_bin() { # <target-dir> <profile>
  dir="$1/$2"
  if [ -f "$dir/sovereign.exe" ]; then
    printf '%s\n' "$dir/sovereign.exe"
    return 0
  fi
  if [ -f "$dir/sovereign" ]; then
    printf '%s\n' "$dir/sovereign"
    return 0
  fi
  return 1
}

# Same-directory regular files whose names name a broker. `.d` dep-info
# and the `sovereign` image itself are not siblings.
sibling_broker_artifacts() { # <bin-dir>
  dir="$1"
  found=""
  for f in "$dir"/*; do
    [ -e "$f" ] || continue
    [ -f "$f" ] || continue
    base=$(basename "$f")
    case "$base" in
    sovereign | sovereign.exe | *.d) continue ;;
    esac
    case "$base" in
    *[Bb][Rr][Oo][Kk][Ee][Rr]*)
      found="$found $base"
      ;;
    esac
  done
  printf '%s\n' "$found"
}

judge_test_transcript() { # <output> <cargo-status>
  output=$1
  cargo_status=$2

  printf '%s' "$output" | grep -Fq "does not have the feature" &&
    fail "Cargo rejected a feature — the requested profile never ran"
  printf '%s' "$output" | grep -Fq "none of the selected packages contains these features" &&
    fail "Cargo rejected the feature set — the requested profile never ran"
  printf '%s' "$output" | grep -Fq "no target named" &&
    fail "Cargo could not find --test broker_bootstrap"
  printf '%s' "$output" | grep -Fq "did not match any targets" &&
    fail "the target filter matched nothing"
  printf '%s' "$output" | grep -Fq "skipping due to unsatisfied required-features" &&
    fail "broker_bootstrap was skipped for unsatisfied required-features"

  if ! printf '%s' "$output" | grep -Eq "^test result:"; then
    fail "no test result line — nothing ran"
  fi
  if printf '%s' "$output" | grep -Eq "^running 0 tests"; then
    fail "zero tests executed"
  fi

  counts=$(printf '%s\n' "$output" |
    sed -n 's/^test result: [a-zA-Z]*\. \([0-9][0-9]*\) passed; \([0-9][0-9]*\) failed; \([0-9][0-9]*\) ignored.*/\1 \2 \3/p')
  if [ -z "$counts" ]; then
    fail "could not parse a test result line"
  fi
  passed=$(printf '%s\n' "$counts" | awk '{s += $1} END {print s + 0}')
  failed=$(printf '%s\n' "$counts" | awk '{s += $2} END {print s + 0}')
  ignored=$(printf '%s\n' "$counts" | awk '{s += $3} END {print s + 0}')
  executed=$((passed + failed))

  if [ "$executed" -eq 0 ]; then
    fail "zero tests executed across the broker_bootstrap target"
  fi
  if [ "$ignored" -ne 0 ]; then
    fail "$ignored broker_bootstrap test(s) were ignored/skipped"
  fi
  if [ "$cargo_status" -ne 0 ]; then
    fail "broker_bootstrap failed ($executed test(s) ran; cargo exit $cargo_status)"
  fi
  if [ "$failed" -ne 0 ]; then
    fail "$failed broker_bootstrap test(s) failed"
  fi
  echo "ok    broker_bootstrap: $passed passed (same CARGO_TARGET_DIR)"
}

# ---- fixture debug image -------------------------------------------------
echo "== fixture debug build"
FIXTURE_TARGET=$(mktemp -d "${TMPDIR:-/tmp}/sfo-broker-fixture.XXXXXX") ||
  fail "could not allocate a fixture CARGO_TARGET_DIR"
checked=$((checked + 1))

echo "    CARGO_TARGET_DIR=$FIXTURE_TARGET"
# ---- fixture debug image
if ! CARGO_TARGET_DIR="$FIXTURE_TARGET" cargo build \
  -p sovereign-cli --bin sovereign \
  --no-default-features --features owner-effect-fixture --locked; then
  fail "fixture debug build failed"
fi
checked=$((checked + 1))

if ! FIXTURE_BIN=$(sovereign_bin "$FIXTURE_TARGET" debug); then
  fail "expected $FIXTURE_TARGET/debug/sovereign[.exe]; nothing was built"
fi
checked=$((checked + 1))
echo "ok    fixture image: $FIXTURE_BIN"

siblings=$(sibling_broker_artifacts "$(dirname "$FIXTURE_BIN")")
checked=$((checked + 1))
if [ -n "$siblings" ]; then
  fail "sibling broker artifact(s) next to the fixture image:$siblings"
fi
echo "ok    no sibling broker artifact"

# ---- same-target broker_bootstrap ----------------------------------------
echo "== broker_bootstrap (same CARGO_TARGET_DIR)"
set +e
bootstrap_out=$(CARGO_TARGET_DIR="$FIXTURE_TARGET" cargo test \
  -p sovereign-cli --test broker_bootstrap \
  --no-default-features --features owner-effect-fixture --locked \
  -- --test-threads=1 2>&1)
bootstrap_status=$?
set -e
printf '%s\n' "$bootstrap_out"
checked=$((checked + 1))
judge_test_transcript "$bootstrap_out" "$bootstrap_status"

# ---- default/release exclusion -------------------------------------------
echo "== default release build"
RELEASE_TARGET=$(mktemp -d "${TMPDIR:-/tmp}/sfo-broker-release.XXXXXX") ||
  fail "could not allocate a release CARGO_TARGET_DIR"
checked=$((checked + 1))

echo "    CARGO_TARGET_DIR=$RELEASE_TARGET"
if ! CARGO_TARGET_DIR="$RELEASE_TARGET" cargo build \
  -p sovereign-cli --bin sovereign \
  --no-default-features --release --locked; then
  fail "default release build failed"
fi
checked=$((checked + 1))

if ! RELEASE_BIN=$(sovereign_bin "$RELEASE_TARGET" release); then
  fail "expected $RELEASE_TARGET/release/sovereign[.exe]; nothing was built"
fi
checked=$((checked + 1))
echo "ok    release image: $RELEASE_BIN"

set +e
help_out=$("$RELEASE_BIN" --help 2>&1)
help_status=$?
set -e
checked=$((checked + 1))
if [ "$help_status" -ne 0 ]; then
  fail "release --help exited $help_status"
fi
if printf '%s' "$help_out" | grep -Fq -- "$HIDDEN_MODE"; then
  fail "release --help names the hidden broker mode"
fi
echo "ok    release --help has no hidden broker mode"

set +e
probe_out=$("$RELEASE_BIN" "$HIDDEN_MODE" 2>&1)
probe_status=$?
set -e
checked=$((checked + 1))
if [ "$probe_status" -eq 0 ]; then
  fail "release binary accepted $HIDDEN_MODE"
fi
if ! printf '%s' "$probe_out" | grep -Eq 'unrecognized subcommand|unexpected argument'; then
  fail "release binary did not reject $HIDDEN_MODE as unknown (got: $probe_out)"
fi
echo "ok    release binary rejects the hidden mode as unknown"

needles="$HIDDEN_MODE
$BROKER_ENTRY"
while IFS= read -r needle; do
  [ -n "$needle" ] || continue
  checked=$((checked + 1))
  set +e
  grep -aFq -- "$needle" "$RELEASE_BIN"
  scan=$?
  set -e
  if [ "$scan" -eq 0 ]; then
    fail "release binary contains '$needle'"
  elif [ "$scan" -ge 2 ]; then
    echo "check-owner-effect-broker-build: scanner error looking for '$needle'" >&2
    exit 2
  fi
  echo "ok    release symbols lack $needle"
done <<NEEDLES_EOF
$needles
NEEDLES_EOF

if [ "$checked" -lt 12 ]; then
  echo "check-owner-effect-broker-build: only $checked checks ran" >&2
  exit 1
fi

completed=1
echo "check-owner-effect-broker-build: OK — $checked checks"

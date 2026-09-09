#!/usr/bin/env bash
# Bounded repetition gate for the authority subprocess-claim tests.
#
# These tests race real processes over one store. A race that is broken one
# time in twenty passes a single run, so a single run proves very little; the
# answer is to repeat it a bounded number of times and stop at the first
# iteration that is anything other than a clean pass.
#
#   ./scripts/run-authority-subprocess-claims.sh --iterations 20
#
# Every iteration runs the manifest-checked `subprocess_claims` target with the
# exact profile the tests need, single-threaded so the processes race the
# filesystem rather than each other's thread scheduler, and every Task 3 test
# must have run in each iteration. Zero executed, a skipped target, or a
# failure ends the run immediately — none of those may be averaged away.
#
# Portability: bash 3.2 (stock macOS).
set -euo pipefail

if [ -z "${BASH_VERSINFO:-}" ] || [ "${BASH_VERSINFO[0]}" -lt 3 ] ||
  { [ "${BASH_VERSINFO[0]}" -eq 3 ] && [ "${BASH_VERSINFO[1]}" -lt 2 ]; }; then
  echo "unsupported bash: need 3.2 or newer, got ${BASH_VERSION:-non-bash shell}" >&2
  exit 3
fi

completed=0
on_exit() {
  status=$?
  if [ "$completed" -eq 0 ] && [ "$status" -eq 0 ]; then
    echo "run-authority-subprocess-claims: INCOMPLETE — exited before finishing" >&2
    exit 1
  fi
}
trap on_exit EXIT

ITERATIONS=""
while [ $# -gt 0 ]; do
  case "$1" in
    --iterations) shift; [ $# -gt 0 ] || { echo "--iterations needs a value" >&2; exit 2; }; ITERATIONS="$1" ;;
    *) echo "usage: run-authority-subprocess-claims.sh --iterations <1..100>" >&2; exit 2 ;;
  esac
  shift
done

# An integer in 1..100 and nothing else: an unbounded or unparsable count is a
# way to run zero iterations and call it a pass.
case "$ITERATIONS" in
  ''|*[!0-9]*) echo "run-authority-subprocess-claims: --iterations must be an integer 1..100" >&2; exit 2 ;;
esac
if [ "$ITERATIONS" -lt 1 ] || [ "$ITERATIONS" -gt 100 ]; then
  echo "run-authority-subprocess-claims: --iterations must be between 1 and 100" >&2
  exit 2
fi

# Every Task 3 test, by name. A run that did not execute all of them did not
# exercise the property, whatever its exit status said.
REQUIRED="real_subprocess_token_claim_has_one_winner
real_subprocess_approval_survives_restart
real_subprocess_idempotency_distinguishes_replay_and_conflict
real_subprocess_mixed_claims_record_current_partial_consumption
kill_after_legacy_temp_sync_before_publish_exposes_no_partial_record"

iteration=1
while [ "$iteration" -le "$ITERATIONS" ]; do
  printf 'iteration %d/%d ... ' "$iteration" "$ITERATIONS"
  set +e
  output=$(cargo test -p sovereign-authority --test subprocess_claims \
    --no-default-features --features fault-injection --locked \
    -- --test-threads=1 2>&1)
  status=$?
  set -e

  if printf '%s' "$output" | grep -Fq "skipping due to unsatisfied required-features"; then
    echo "FAILED"
    echo "the target was skipped for unsatisfied required-features" >&2
    printf '%s\n' "$output" >&2
    exit 1
  fi
  if ! printf '%s' "$output" | grep -Eq "^test result:"; then
    echo "FAILED"
    echo "no test result line — nothing ran" >&2
    printf '%s\n' "$output" >&2
    exit 1
  fi
  if [ "$status" -ne 0 ]; then
    echo "FAILED"
    printf '%s\n' "$output" >&2
    exit 1
  fi

  missing=""
  for name in $REQUIRED; do
    if ! printf '%s' "$output" | grep -Eq "^test $name \.\.\. ok"; then
      missing="$missing $name"
    fi
  done
  if [ -n "$missing" ]; then
    echo "FAILED"
    echo "these tests did not run and pass in this iteration:$missing" >&2
    exit 1
  fi

  echo "ok"
  iteration=$((iteration + 1))
done

completed=1
echo "run-authority-subprocess-claims: OK — $ITERATIONS iteration(s), all five tests each time"

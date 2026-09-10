#!/usr/bin/env bash
# Bounded repetition for the broker kill matrix.
#
# Each case in the matrix spawns a broker, drives a handshake, waits for a
# barrier and kills the process. That is four moving parts and a real
# filesystem, so one green run says less than it looks like: a matrix that
# passes nineteen times in twenty passes a single run.
#
# The barrier makes the kill land at an exact point rather than at a sleep, so
# these are not expected to be flaky — which is the reason to repeat them. A
# test that is *believed* not to be flaky and never checked is
# indistinguishable from one that is.
#
# Any iteration that is not a clean pass ends the run. Zero executed, a
# skipped target, or a missing test all stop it, because none of those may be
# averaged into a pass.
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
    echo "exact-effect-kill-matrix: INCOMPLETE — exited before finishing" >&2
    exit 1
  fi
}
trap on_exit EXIT

ITERATIONS=""
while [ $# -gt 0 ]; do
  case "$1" in
    --iterations) shift; [ $# -gt 0 ] || { echo "--iterations needs a value" >&2; exit 2; }; ITERATIONS="$1" ;;
    *) echo "usage: exact-effect-kill-matrix.sh --iterations <1..50>" >&2; exit 2 ;;
  esac
  shift
done

case "$ITERATIONS" in
  ''|*[!0-9]*) echo "exact-effect-kill-matrix: --iterations must be an integer 1..50" >&2; exit 2 ;;
esac
if [ "$ITERATIONS" -lt 1 ] || [ "$ITERATIONS" -gt 50 ]; then
  echo "exact-effect-kill-matrix: --iterations must be between 1 and 50" >&2
  exit 2
fi

# Every case, by name. A run that did not execute all of them did not exercise
# the matrix, whatever its exit status said.
REQUIRED="killed_after_address_before_hello_leaves_no_lock_and_no_store
killed_after_hello_before_lock_leaves_no_lock_and_no_store
killed_after_lock_before_redb_leaves_a_releasable_lock_and_no_store
killed_after_redb_open_leaves_a_store_the_next_broker_can_take
every_broker_barrier_is_reachable"

iteration=1
while [ "$iteration" -le "$ITERATIONS" ]; do
  printf 'iteration %d/%d ... ' "$iteration" "$ITERATIONS"
  set +e
  output=$(cargo test -p sovereign-cli --test broker_kill_matrix \
    --no-default-features --features owner-effect-fixture,fault-injection --locked \
    -- --test-threads=1 2>&1)
  status=$?
  set -e

  if printf '%s' "$output" | grep -Fq "skipping due to unsatisfied required-features"; then
    echo "FAILED"; echo "the target was skipped for unsatisfied required-features" >&2
    printf '%s\n' "$output" >&2; exit 1
  fi
  if ! printf '%s' "$output" | grep -Eq "^test result:"; then
    echo "FAILED"; echo "no test result line — nothing ran" >&2
    printf '%s\n' "$output" >&2; exit 1
  fi
  if [ "$status" -ne 0 ]; then
    echo "FAILED"; printf '%s\n' "$output" >&2; exit 1
  fi

  missing=""
  for name in $REQUIRED; do
    if ! printf '%s' "$output" | grep -Eq "^test $name \.\.\. ok"; then
      missing="$missing $name"
    fi
  done
  if [ -n "$missing" ]; then
    echo "FAILED"
    echo "these cases did not run and pass in this iteration:$missing" >&2
    exit 1
  fi

  echo "ok"
  iteration=$((iteration + 1))
done

completed=1
echo "exact-effect-kill-matrix: OK — $ITERATIONS iteration(s), all five cases each time"

#!/usr/bin/env bash
# Bounded soak for the v2 synthetic owner-effect fixture (v01-D07).
#
# Each crash boundary is a real process-kill/reopen, and each named same-
# process race is a thread race inside one fixture process. One green run
# says less than it looks like: a case that is broken one time in twenty
# still passes a single invocation.
#
# Zero executed, a skipped target, or a missing named test all stop the
# run. None of those may be averaged into a pass.
#
# This qualifies a synthetic unqualified plaintext fixture. It cannot
# change product status. Full cross-process validator race stays Target.
#
# Portability: bash 3.2 (stock macOS).
# Honesty (backlog lesson 8): EXIT-trap completion marker; a run that
# executed zero iterations fails rather than printing OK.
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
    echo "run-synthetic-owner-effect-soak: INCOMPLETE — exited before finishing" >&2
    exit 1
  fi
}
trap on_exit EXIT

CRASH_ITERATIONS=""
RACE_ITERATIONS=""
while [ $# -gt 0 ]; do
  case "$1" in
    --crash-iterations)
      shift
      [ $# -gt 0 ] || { echo "--crash-iterations needs a value" >&2; exit 2; }
      CRASH_ITERATIONS="$1"
      ;;
    --race-iterations)
      shift
      [ $# -gt 0 ] || { echo "--race-iterations needs a value" >&2; exit 2; }
      RACE_ITERATIONS="$1"
      ;;
    *)
      echo "usage: run-synthetic-owner-effect-soak.sh --crash-iterations <1..50> --race-iterations <1..200>" >&2
      exit 2
      ;;
  esac
  shift
done

case "$CRASH_ITERATIONS" in
  ''|*[!0-9]*) echo "run-synthetic-owner-effect-soak: --crash-iterations must be an integer 1..50" >&2; exit 2 ;;
esac
case "$RACE_ITERATIONS" in
  ''|*[!0-9]*) echo "run-synthetic-owner-effect-soak: --race-iterations must be an integer 1..200" >&2; exit 2 ;;
esac
if [ "$CRASH_ITERATIONS" -lt 1 ] || [ "$CRASH_ITERATIONS" -gt 50 ]; then
  echo "run-synthetic-owner-effect-soak: --crash-iterations must be between 1 and 50" >&2
  exit 2
fi
if [ "$RACE_ITERATIONS" -lt 1 ] || [ "$RACE_ITERATIONS" -gt 200 ]; then
  echo "run-synthetic-owner-effect-soak: --race-iterations must be between 1 and 200" >&2
  exit 2
fi

PACKAGE="sovereign-synthetic-owner-effect"
FEATURES="--no-default-features --features owner-effect-fixture --locked"

# target<TAB>test_name  (space-safe: names have no spaces)
CRASH_CASES="reserve_kill	real_process_kill_before_commit_leaves_none_of_the_reservation
reserve_kill	real_process_kill_after_commit_leaves_all_of_the_reservation
publish_kill	real_process_kill_before_dispatching_commit_stays_pre_dispatch
publish_kill	real_process_kill_after_dispatching_before_write_is_indeterminate
publish_kill	real_process_kill_after_publication_reopens_succeeded"

RACE_CASES="reserve_races	same_process_concurrent_http_or_thread_reservation_has_one_winner
reserve_races	logout_race_refuses_and_commits_nothing
publish_races	same_process_concurrent_dispatch_reconcile_has_one_terminal_winner"

run_named() { # <target> <test_name>
  target="$1"
  name="$2"
  set +e
  # shellcheck disable=SC2086
  output=$(cargo test -p "$PACKAGE" --test "$target" $FEATURES \
    -- --exact "$name" --test-threads=1 2>&1)
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
  if printf '%s' "$output" | grep -Eq "^test result: ok\\. 0 passed"; then
    echo "FAILED"
    echo "zero tests executed for $name" >&2
    printf '%s\n' "$output" >&2
    exit 1
  fi
  if ! printf '%s' "$output" | grep -Eq "^test $name \\.\\.\\. ok"; then
    echo "FAILED"
    echo "named test $name did not run and pass" >&2
    printf '%s\n' "$output" >&2
    exit 1
  fi
  if [ "$status" -ne 0 ]; then
    echo "FAILED"
    printf '%s\n' "$output" >&2
    exit 1
  fi
}

executed=0

echo "== crash boundaries ($CRASH_ITERATIONS iteration(s) each)"
iteration=1
while [ "$iteration" -le "$CRASH_ITERATIONS" ]; do
  printf 'crash iteration %d/%d\n' "$iteration" "$CRASH_ITERATIONS"
  while IFS='	' read -r target name; do
    [ -n "$target" ] || continue
    printf '  %s::%s ... ' "$target" "$name"
    run_named "$target" "$name"
    echo "ok"
  done <<EOF
$CRASH_CASES
EOF
  executed=$((executed + 1))
  iteration=$((iteration + 1))
done

echo "== same-process races ($RACE_ITERATIONS iteration(s) each)"
iteration=1
while [ "$iteration" -le "$RACE_ITERATIONS" ]; do
  printf 'race iteration %d/%d\n' "$iteration" "$RACE_ITERATIONS"
  while IFS='	' read -r target name; do
    [ -n "$target" ] || continue
    printf '  %s::%s ... ' "$target" "$name"
    run_named "$target" "$name"
    echo "ok"
  done <<EOF
$RACE_CASES
EOF
  executed=$((executed + 1))
  iteration=$((iteration + 1))
done

if [ "$executed" -lt 1 ]; then
  echo "run-synthetic-owner-effect-soak: executed zero iterations" >&2
  exit 1
fi

completed=1
echo "run-synthetic-owner-effect-soak: OK — crash $CRASH_ITERATIONS, race $RACE_ITERATIONS (owner-effect-fixture)"

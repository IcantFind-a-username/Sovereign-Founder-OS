#!/usr/bin/env bash
# Checked wrapper for the supplemental Cargo commands the owner-session plan
# runs outside the manifest:
#
#   run-owner-effect-regression.sh -- cargo test -p <pkg> --no-default-features ...
#
# The manifest runner covers registered rows. This covers the package- and
# workspace-wide commands beside them, and exists for the same reason: a raw
# `cargo test` in a RED, GREEN, checkpoint, or final block can report success
# having run nothing at all. It judges the command before running it and the
# transcript afterwards, and preserves the underlying exit status so a real
# failure still fails.
#
# Refused: a command that is not `cargo test`; one without
# `--no-default-features`, since the plan requires every command to declare its
# feature set rather than inherit one; Cargo feature/target diagnostics; a
# target skipped for unsatisfied required-features; a fully filtered run; and
# an aggregate of zero executed tests.
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
    echo "run-owner-effect-regression: INCOMPLETE — exited before finishing" >&2
    exit 1
  fi
}
trap on_exit EXIT

if [ "${1:-}" != "--" ]; then
  echo "usage: run-owner-effect-regression.sh -- cargo test [args...]" >&2
  exit 2
fi
shift

if [ $# -eq 0 ]; then
  echo "run-owner-effect-regression: no command given" >&2
  exit 2
fi
if [ "$1" != "cargo" ] || [ "${2:-}" != "test" ]; then
  echo "run-owner-effect-regression: only 'cargo test' may be wrapped, got: $*" >&2
  exit 2
fi

# The command must declare its feature set. Inheriting the default set is
# exactly how a command silently runs something other than what was intended.
saw_no_default=0
for arg in "$@"; do
  if [ "$arg" = "--no-default-features" ]; then
    saw_no_default=1
  fi
done
if [ "$saw_no_default" -eq 0 ]; then
  echo "run-owner-effect-regression: the command must carry --no-default-features" >&2
  echo "  got: $*" >&2
  exit 2
fi

echo "run-owner-effect-regression: $*"
set +e
output=$("$@" 2>&1)
status=$?
set -e
printf '%s\n' "$output"

fail() { echo "run-owner-effect-regression: FAIL — $1" >&2; exit 1; }

# Cargo could not run what was asked; the status alone does not say so.
printf '%s' "$output" | grep -Fq "does not have the feature" &&
  fail "Cargo rejected a feature — the requested profile never ran"
printf '%s' "$output" | grep -Fq "none of the selected packages contains these features" &&
  fail "Cargo rejected the feature set — the requested profile never ran"
printf '%s' "$output" | grep -Fq "no target named" &&
  fail "Cargo could not find the named target"
printf '%s' "$output" | grep -Fq "did not match any targets" &&
  fail "the target filter matched nothing"
printf '%s' "$output" | grep -Fq "skipping due to unsatisfied required-features" &&
  fail "a target was skipped for unsatisfied required-features"

# Aggregate the result lines. Zero executed across every binary is the
# transcript that reads as a clean pass while proving nothing.
if ! printf '%s' "$output" | grep -Eq "^test result:"; then
  fail "no test result line — nothing ran"
fi
executed=$(printf '%s\n' "$output" |
  sed -n 's/^test result: [a-zA-Z]*\. \([0-9][0-9]*\) passed; \([0-9][0-9]*\) failed.*/\1 \2/p' |
  awk '{total += $1 + $2} END {print total + 0}')
if [ "${executed:-0}" -eq 0 ]; then
  fail "zero tests executed across every binary"
fi

completed=1
if [ "$status" -ne 0 ]; then
  echo "run-owner-effect-regression: the command failed ($executed test(s) ran); preserving exit $status" >&2
  exit "$status"
fi
echo "run-owner-effect-regression: OK — $executed test(s) executed"

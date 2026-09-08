#!/usr/bin/env bash
# Guardrail against god files: fail CI when any single source file grows past
# the limit, so oversized modules get split at the source instead of accreting.
# Raising a limit or extending the allowlist is a deliberate, reviewed change.
#
# Portability: bash 3.2 (stock macOS) — no associative arrays, no `mapfile`.
# A guardrail that cannot run on a developer's machine is not a guardrail.
set -euo pipefail

if [ -z "${BASH_VERSINFO:-}" ] || [ "${BASH_VERSINFO[0]}" -lt 3 ] ||
  { [ "${BASH_VERSINFO[0]}" -eq 3 ] && [ "${BASH_VERSINFO[1]}" -lt 2 ]; }; then
  echo "unsupported bash: need 3.2 or newer, got ${BASH_VERSION:-non-bash shell}" >&2
  exit 3
fi

LIMIT_RUST=1200
LIMIT_FRONTEND=800

# Files tolerated above the limit at a pinned ceiling. Empty today — keep it
# that way; split modules instead of adding entries. A `case` rather than an
# associative array, so this runs on bash 3.2.
allowlist_limit() { # <path> -> prints a pinned limit, or nothing
  case "$1" in
  # apps/cli/assets/example.js) echo 900 ;;
  *) ;;
  esac
}

fail=0
checked=0
while IFS= read -r file; do
  lines=$(wc -l <"$file")
  case "$file" in
  *.rs) limit=$LIMIT_RUST ;;
  *) limit=$LIMIT_FRONTEND ;;
  esac
  pinned=$(allowlist_limit "$file")
  if [ -n "$pinned" ]; then
    limit=$pinned
  fi
  checked=$((checked + 1))
  if [ "$lines" -gt "$limit" ]; then
    echo "FAIL  $file: $lines lines (limit $limit) — split it before it becomes a god file"
    fail=1
  fi
done < <(git ls-files 'crates/*.rs' 'crates/**/*.rs' 'apps/*.rs' 'apps/**/*.rs' 'apps/**/*.js' 'apps/**/*.css' 'apps/**/*.html')

# A run that inspected nothing is a broken run, not a passing one: it means we
# are outside the repo, or the glob list stopped matching the tree.
if [ "$checked" -eq 0 ]; then
  echo "FAIL  no source files matched — the guardrail did not actually check anything" >&2
  exit 1
fi

if [ "$fail" -eq 0 ]; then
  echo "OK    every source file is within its size limit ($checked files checked)"
fi
exit $fail

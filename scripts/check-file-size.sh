#!/usr/bin/env bash
# Guardrail against god files: fail CI when any single source file grows past
# the limit, so oversized modules get split at the source instead of accreting.
# Raising a limit or extending the allowlist is a deliberate, reviewed change.
set -euo pipefail

LIMIT_RUST=1200
LIMIT_FRONTEND=800

# Pre-existing files already over the limit, tolerated at their current scale
# with a tighter ceiling than they'd need to keep growing. Split, then remove.
declare -A ALLOWLIST=(
  ["crates/identity/src/lib.rs"]=1500
)

fail=0
while IFS= read -r file; do
  lines=$(wc -l <"$file")
  case "$file" in
  *.rs) limit=$LIMIT_RUST ;;
  *) limit=$LIMIT_FRONTEND ;;
  esac
  if [[ -n "${ALLOWLIST[$file]:-}" ]]; then
    limit=${ALLOWLIST[$file]}
  fi
  if ((lines > limit)); then
    echo "FAIL  $file: $lines lines (limit $limit) — split it before it becomes a god file"
    fail=1
  fi
done < <(git ls-files 'crates/*.rs' 'crates/**/*.rs' 'apps/*.rs' 'apps/**/*.rs' 'apps/**/*.js' 'apps/**/*.css' 'apps/**/*.html')

if ((fail == 0)); then
  echo "OK    every source file is within its size limit"
fi
exit $fail

#!/usr/bin/env bash
# Claude Code Stop hook: refuse to end a session while the change set fails
# the scoped quality gate (scripts/test_changed.sh). Exit 2 + stderr sends the
# failure back into the session so the agent keeps fixing instead of stopping.
set -uo pipefail

cd "$(git rev-parse --show-toplevel)" || exit 0

INPUT=$(cat 2>/dev/null || true)

# Loop guard: when this stop attempt was itself caused by a previous block
# from this hook, stop_hook_active is true — let the session end rather than
# ping-ponging forever on an unfixable failure.
if command -v jq >/dev/null 2>&1; then
  ACTIVE=$(printf '%s' "$INPUT" | jq -r '.stop_hook_active // false' 2>/dev/null)
else
  case "$INPUT" in *'"stop_hook_active"'*true*) ACTIVE=true ;; *) ACTIVE=false ;; esac
fi
if [[ "$ACTIVE" == "true" ]]; then
  exit 0
fi

if OUT=$(./scripts/test_changed.sh 2>&1); then
  exit 0
fi

{
  echo "Quality gate failed — the session's changes are not green yet."
  echo "Fix the failure below (or revert the offending change), then stop again."
  echo "If it cannot be fixed this round, record the diagnosis in docs/backlog.md"
  echo "under the task you were working on before stopping."
  printf '%s\n' "$OUT" | tail -n 30
} >&2
exit 2

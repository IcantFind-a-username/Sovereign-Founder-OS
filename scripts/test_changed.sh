#!/usr/bin/env bash
# Scoped quality gate: run only the checks that the current change set needs.
#
# Change set = committed diff vs merge-base with main
#            + staged and unstaged working-tree changes
#            + untracked files (git ls-files --others — easy to forget, so
#              a brand-new file with a failing test still gates the session).
#
# Output discipline (enforced here, not by convention):
#   - the full build/test log goes to .harness/test_changed.log (gitignored)
#   - success prints ONE summary line, naming every step that ran
#   - failure prints only the last 25 log lines
# so a large cargo log never floods an agent's context window.
#
# Scope mapping:
#   crates/<x>/**            -> cargo package sovereign-<x> (+ adversarial suite)
#   apps/cli/**/*.rs         -> sovereign-cli (+ adversarial suite)
#   apps/cli/assets/**       -> frontend type-check (tsc --checkJs, pinned)
#   tests/adversarial/**     -> sovereign-adversarial-tests
#   Cargo.toml / Cargo.lock / rust-toolchain.toml / scripts/ / .github/
#                            -> full workspace
#   docs, rfcs, markdown     -> no test run (fmt/file-size still checked)
#
# Scoped runs do NOT rebuild reverse dependencies of a changed crate; CI runs
# the full workspace on every push and is the backstop for cross-crate breakage.
#
# Two properties this script owes its callers (the Stop hook trusts its exit
# status, and scripts/tests/gate_portability_test.sh pins both):
#   - it runs on bash 3.2, the stock macOS shell: no associative arrays, no
#     `mapfile`, no case-modifying expansions;
#   - it never exits 0 without having run its checks. Any abort before the
#     completion marker is converted to a nonzero exit by the EXIT trap below.
set -uo pipefail

if [ -z "${BASH_VERSINFO:-}" ] || [ "${BASH_VERSINFO[0]}" -lt 3 ] ||
  { [ "${BASH_VERSINFO[0]}" -eq 3 ] && [ "${BASH_VERSINFO[1]}" -lt 2 ]; }; then
  echo "unsupported bash: need 3.2 or newer, got ${BASH_VERSION:-non-bash shell}" >&2
  exit 3
fi

# ---- never report success without having finished -------------------------
GATE_STATE=running # running -> failed (a check failed) | completed
STEPS_RUN=""
on_exit() {
  local status=$?
  if [ "$GATE_STATE" = "running" ]; then
    {
      echo "GATE ABORTED before finishing (raw exit status $status) —"
      echo "refusing to report success. Steps that had run:${STEPS_RUN:- none}"
    } >&2
    if [ "$status" -eq 0 ]; then
      status=1
    fi
    exit "$status"
  fi
}
trap on_exit EXIT

cd "$(git rev-parse --show-toplevel)" || exit 1

LOG="${TEST_CHANGED_LOG:-.harness/test_changed.log}"
mkdir -p "$(dirname "$LOG")"
: >"$LOG"

fail() {
  GATE_STATE=failed
  echo "GATE FAILED at step: $1 — last 25 lines of $LOG:" >&2
  tail -n 25 "$LOG" >&2
  exit 1
}

run_step() { # run_step <label> <cmd...>
  local label=$1
  shift
  echo "==== [$label] \$ $*" >>"$LOG"
  "$@" >>"$LOG" 2>&1 || fail "$label"
  STEPS_RUN="$STEPS_RUN $label"
}

# ---- collect the change set ---------------------------------------------
# TEST_CHANGED_BASE overrides the diff base (default: merge-base with main).
BASE=""
if [ -n "${TEST_CHANGED_BASE:-}" ]; then
  BASE=$(git rev-parse --verify "$TEST_CHANGED_BASE") || exit 1
else
  for ref in origin/main main; do
    if git rev-parse --verify -q "$ref" >/dev/null; then
      BASE=$(git merge-base HEAD "$ref" 2>/dev/null) && break
    fi
  done
fi

CHANGED=$(
  {
    [ -n "$BASE" ] && git diff --name-only "$BASE" HEAD
    git diff --name-only            # unstaged
    git diff --name-only --cached   # staged
    git ls-files --others --exclude-standard # untracked — do not drop this
  } | sort -u
)

# ---- map paths to scopes -------------------------------------------------
# PKGS is a space-delimited, space-padded list of cargo package names rather
# than an associative array, because bash 3.2 has none.
FULL=0
FRONTEND=0
PKGS=""
add_pkg() {
  case " $PKGS " in
  *" $1 "*) ;;             # already queued
  *) PKGS="$PKGS $1" ;;
  esac
}
while IFS= read -r f; do
  case "$f" in
  Cargo.toml | Cargo.lock | rust-toolchain.toml | scripts/* | .github/*)
    FULL=1
    ;;
  crates/*/*)
    rest=${f#crates/}
    add_pkg "sovereign-${rest%%/*}"
    ;;
  apps/cli/assets/*)
    FRONTEND=1
    ;;
  apps/cli/*)
    add_pkg "sovereign-cli"
    ;;
  tests/adversarial/*)
    add_pkg "sovereign-adversarial-tests"
    ;;
  esac
done <<EOF
$CHANGED
EOF
# any runtime-crate change also runs the cross-crate security invariants
if [ -n "$PKGS" ]; then
  add_pkg "sovereign-adversarial-tests"
fi

# ---- always-on cheap gates ----------------------------------------------
# These run even when the tree is clean: "nothing changed" is not a reason to
# exit 0 without checking anything, and the cost is seconds.
if [ "${GATE_SELFTEST_RUNNING:-0}" != "1" ]; then
  run_step "gate-self-test" env GATE_SELFTEST_RUNNING=1 \
    ./scripts/tests/gate_portability_test.sh
fi
run_step "file-size" ./scripts/check-file-size.sh
run_step "fmt" cargo fmt --all --check

# ---- scoped clippy + tests ----------------------------------------------
RAN=""
if [ "$FULL" -eq 1 ]; then
  run_step "clippy(workspace)" cargo clippy --workspace --all-targets --locked -- -D warnings
  run_step "test(workspace)" cargo test --workspace --locked
  RAN="workspace"
elif [ -n "$PKGS" ]; then
  P_FLAGS=()
  for p in $PKGS; do P_FLAGS+=(-p "$p"); done
  run_step "clippy(scoped)" cargo clippy "${P_FLAGS[@]}" --all-targets --locked -- -D warnings
  run_step "test(scoped)" cargo test "${P_FLAGS[@]}" --locked
  RAN="${PKGS# }"
fi

# ---- frontend type-check (environment-dependent: needs npx + network) ----
FE_NOTE=""
if [ "$FRONTEND" -eq 1 ] || [ "$FULL" -eq 1 ]; then
  if command -v npx >/dev/null 2>&1; then
    run_step "tsc(frontend)" env npm_config_ignore_scripts=true \
      npx -y -p typescript@5.5.4 tsc -p apps/cli/assets/tsconfig.json
    FE_NOTE=", frontend tsc ok"
  else
    # explicit skip, never silent: CI still runs this check on push
    FE_NOTE=", SKIPPED frontend tsc (npx not installed in this environment; CI covers it)"
  fi
fi

# The cheap gates above are unconditional, so reaching here without them is a
# bug in this script rather than a possible state — check anyway, since the
# whole point of this file is that a gate must not be able to lie.
case "$STEPS_RUN" in
*" file-size "* | *" file-size") ;;
*)
  echo "GATE BUG: reached the summary without running file-size" >&2
  exit 1
  ;;
esac

GATE_STATE=completed
if [ -z "$RAN" ] && [ "$FRONTEND" -eq 0 ]; then
  echo "test_changed: ALL GREEN — steps:$STEPS_RUN — no cargo test scope in this change set (full log: $LOG)"
else
  echo "test_changed: ALL GREEN — steps:$STEPS_RUN — scope: ${RAN:-frontend-only}${FE_NOTE} (full log: $LOG)"
fi

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
#   - success prints ONE summary line
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
set -uo pipefail

cd "$(git rev-parse --show-toplevel)" || exit 1

LOG_DIR=".harness"
LOG="$LOG_DIR/test_changed.log"
mkdir -p "$LOG_DIR"
: >"$LOG"

fail() {
  echo "GATE FAILED at step: $1 — last 25 lines of $LOG:" >&2
  tail -n 25 "$LOG" >&2
  exit 1
}

run_step() { # run_step <label> <cmd...>
  local label=$1
  shift
  echo "==== [$label] \$ $*" >>"$LOG"
  "$@" >>"$LOG" 2>&1 || fail "$label"
}

# ---- collect the change set ---------------------------------------------
# TEST_CHANGED_BASE overrides the diff base (default: merge-base with main).
BASE=""
if [[ -n "${TEST_CHANGED_BASE:-}" ]]; then
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
    [[ -n "$BASE" ]] && git diff --name-only "$BASE" HEAD
    git diff --name-only            # unstaged
    git diff --name-only --cached   # staged
    git ls-files --others --exclude-standard # untracked — do not drop this
  } | sort -u
)

if [[ -z "$CHANGED" ]]; then
  echo "test_changed: no changes vs main and clean tree — nothing to test."
  exit 0
fi

# ---- map paths to scopes -------------------------------------------------
FULL=0
FRONTEND=0
declare -A PKGS=()
while IFS= read -r f; do
  case "$f" in
  Cargo.toml | Cargo.lock | rust-toolchain.toml | scripts/* | .github/*)
    FULL=1
    ;;
  crates/*/*)
    PKGS["sovereign-${f#crates/}"]=1
    ;;
  apps/cli/assets/*)
    FRONTEND=1
    ;;
  apps/cli/*)
    PKGS["sovereign-cli"]=1
    ;;
  tests/adversarial/*)
    PKGS["sovereign-adversarial-tests"]=1
    ;;
  esac
done <<<"$CHANGED"
# normalize crates/<x>/rest-of-path -> sovereign-<x>
for k in "${!PKGS[@]}"; do
  if [[ "$k" == sovereign-*/* ]]; then
    unset "PKGS[$k]"
    PKGS["${k%%/*}"]=1
  fi
done
# any runtime-crate change also runs the cross-crate security invariants
if ((${#PKGS[@]} > 0)); then
  PKGS["sovereign-adversarial-tests"]=1
fi

# ---- always-on cheap gates ----------------------------------------------
run_step "file-size" ./scripts/check-file-size.sh
run_step "fmt" cargo fmt --all --check

# ---- scoped clippy + tests ----------------------------------------------
RAN=""
if ((FULL)); then
  run_step "clippy(workspace)" cargo clippy --workspace --all-targets --locked -- -D warnings
  run_step "test(workspace)" cargo test --workspace --locked
  RAN="workspace"
elif ((${#PKGS[@]} > 0)); then
  P_FLAGS=()
  for p in "${!PKGS[@]}"; do P_FLAGS+=(-p "$p"); done
  run_step "clippy(scoped)" cargo clippy "${P_FLAGS[@]}" --all-targets --locked -- -D warnings
  run_step "test(scoped)" cargo test "${P_FLAGS[@]}" --locked
  RAN="${!PKGS[*]}"
fi

# ---- frontend type-check (environment-dependent: needs npx + network) ----
FE_NOTE=""
if ((FRONTEND || FULL)); then
  if command -v npx >/dev/null 2>&1; then
    run_step "tsc(frontend)" env npm_config_ignore_scripts=true \
      npx -y -p typescript@5.5.4 tsc -p apps/cli/assets/tsconfig.json
    FE_NOTE=", frontend tsc ok"
  else
    # explicit skip, never silent: CI still runs this check on push
    FE_NOTE=", SKIPPED frontend tsc (npx not installed in this environment; CI covers it)"
  fi
fi

if [[ -z "$RAN" && $FRONTEND -eq 0 ]]; then
  echo "test_changed: docs/config-only change — fmt + file-size ok, no test scope."
else
  echo "test_changed: ALL GREEN — scope: ${RAN:-frontend-only}${FE_NOTE} (full log: $LOG)"
fi

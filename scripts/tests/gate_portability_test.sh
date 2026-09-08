#!/usr/bin/env bash
# Self-test for the quality gate scripts themselves.
#
# Two properties, both learned the hard way (docs/backlog.md, 2026-08-15):
#
#   1. The gate must run on the oldest bash we target. Stock macOS ships
#      bash 3.2.57, so one bash-4 construct (`declare -A`) made both gate
#      scripts abort before a single check ran.
#   2. A gate that cannot do its job must never exit 0. The damaging half of
#      that bug was not the abort — it was the abort *plus* a zero exit
#      status, which the Stop hook and CI read as a passing session.
#
# This script is itself written for bash 3.2. It deliberately contains the
# poison string `declare -A`, so the construct scan below skips it and covers
# the gate scripts in scripts/ instead.
set -uo pipefail

ROOT=$(cd "$(dirname "$0")/../.." && pwd) || exit 1
cd "$ROOT" || exit 1

# Prefer the system bash when it is older than the one running us: on macOS
# that is the real 3.2.57, which makes the portability check an execution
# result rather than an aspiration. On Linux (bash 5) the construct scan and
# `bash -n` parse below are what carry the 3.2 claim, so say so out loud.
OLD_BASH=$(command -v bash)
if [ -x /bin/bash ]; then
  OLD_BASH=/bin/bash
fi
OLD_BASH_VERSION=$("$OLD_BASH" -c 'echo "$BASH_VERSION"')

FAILURES=0
pass() { echo "  ok   $1"; }
fail() {
  echo "  FAIL $1: $2" >&2
  FAILURES=$((FAILURES + 1))
}

GATE_SCRIPTS="scripts/test_changed.sh scripts/check-file-size.sh scripts/stop_gate.sh"

# ---- 1. no bash-4-only constructs in the gate scripts --------------------
# Whole-line comments are stripped first: the gate scripts document which
# constructs they avoid, and naming one must not read as using one.
scan_file() { # <path> <grep-flag> <pattern> -> 0 found, 1 clean, 2+ scan broke
  grep -v '^[[:space:]]*#' "$1" | grep -q "$2" -e "$3"
}

CONSTRUCT_HITS=0
scan_for() { # <grep-flag> <pattern> <human name>
  local flag=$1 pattern=$2 name=$3
  local script rc
  for script in $GATE_SCRIPTS; do
    scan_file "$script" "$flag" "$pattern"
    rc=$?
    case "$rc" in
    0)
      fail "no-bash4-constructs" "$script uses $name (bash 4+; stock macOS is 3.2)"
      CONSTRUCT_HITS=$((CONSTRUCT_HITS + 1))
      ;;
    1) ;;
    *)
      # A pattern grep cannot compile scans nothing and finds nothing — the
      # same silent-pass shape this whole file exists to prevent.
      fail "no-bash4-constructs" "the scan for $name did not run (grep exit $rc)"
      CONSTRUCT_HITS=$((CONSTRUCT_HITS + 1))
      ;;
    esac
  done
  # Positive control: the same pattern must fire on a file that does use the
  # construct, so a typo in the regex cannot masquerade as a clean tree.
  scan_file "$CONTROL" "$flag" "$pattern"
  if [ $? -ne 0 ]; then
    fail "no-bash4-constructs" "the pattern for $name does not match its own control line"
    CONSTRUCT_HITS=$((CONSTRUCT_HITS + 1))
  fi
}

CONTROL="$ROOT/.harness/gate-self-test-control.$$"
{
  echo 'declare -A assoc=()'
  echo 'typeset -A other=()'
  echo 'local -gA third=()'
  echo 'readarray -t lines <input'
  echo 'mapfile -t lines <input'
  echo 'producer |& consumer'
  echo 'echo "${name^^} ${name,,}"'
} >"$CONTROL"

scan_for -E '(declare|typeset|local)[[:space:]]+-[A-Za-z]*A' 'an associative array declaration'
scan_for -F 'readarray' 'readarray'
scan_for -F 'mapfile' 'mapfile'
scan_for -F '|&' 'the |& pipe'
scan_for -E '[$][{][A-Za-z_][A-Za-z_0-9]*(,,|\^\^)' 'a case-modifying expansion'
rm -f "$CONTROL"
if [ "$CONSTRUCT_HITS" -eq 0 ]; then
  pass "no-bash4-constructs ($GATE_SCRIPTS)"
fi

# ---- 2. every gate script parses under the oldest available bash ---------
for script in $GATE_SCRIPTS; do
  if "$OLD_BASH" -n "$script" 2>/dev/null; then
    pass "parses-under-bash-$OLD_BASH_VERSION ($script)"
  else
    fail "parses-under-bash-$OLD_BASH_VERSION" "$script has a syntax error"
  fi
done

# ---- 3. check-file-size.sh runs to completion on the oldest bash ---------
OUT=$("$OLD_BASH" ./scripts/check-file-size.sh 2>&1)
STATUS=$?
if [ "$STATUS" -eq 0 ] && printf '%s' "$OUT" | grep -q '^OK'; then
  pass "check-file-size-runs (bash $OLD_BASH_VERSION)"
else
  fail "check-file-size-runs" "exit $STATUS, output: $(printf '%s' "$OUT" | tail -n 2)"
fi

# ---- 4. check-file-size.sh has teeth: an oversized file must fail --------
# A throwaway git repo, because the script's file list comes from git ls-files.
# It lives under the gitignored .harness/ rather than $TMPDIR so the test also
# works where the process is confined to the workspace.
FIXTURE="$ROOT/.harness/gate-self-test.$$"
rm -rf "$FIXTURE"
mkdir -p "$FIXTURE" || exit 1
trap 'rm -rf "$FIXTURE"' EXIT
# Every step is checked: if `git init` fails, a later bare `git add` would
# reach the *parent* repository instead of the fixture.
build_fixture() (
  cd "$FIXTURE" || return 1
  git init -q . >/dev/null 2>&1 || return 1
  [ -d "$FIXTURE/.git" ] || return 1
  mkdir -p crates/oversized/src || return 1
  i=0
  while [ "$i" -lt 1201 ]; do
    echo "// line $i"
    i=$((i + 1))
  done >crates/oversized/src/lib.rs
  git add -- crates >/dev/null 2>&1 || return 1
  [ -n "$(git ls-files)" ] || return 1
)
if build_fixture; then
  OUT=$(cd "$FIXTURE" && "$OLD_BASH" "$ROOT/scripts/check-file-size.sh" 2>&1)
  STATUS=$?
  if [ "$STATUS" -ne 0 ] && printf '%s' "$OUT" | grep -q 'crates/oversized/src/lib.rs'; then
    pass "check-file-size-has-teeth (1201-line file rejected)"
  else
    fail "check-file-size-has-teeth" "a 1201-line file passed: exit $STATUS, output: $OUT"
  fi
else
  fail "check-file-size-has-teeth" \
    "could not build the fixture git repo under $FIXTURE — the check could not be proven"
fi

# ---- 5. an aborted gate never exits 0 ------------------------------------
# Inject a premature success into a copy of the real gate, right before its
# first check, and require the copy to still exit nonzero. `declare -A` covers
# the historical bash-3.2 abort; the bare `exit 0` covers every other way a
# future edit could bail out early on a newer bash.
ANCHOR='# ---- map paths to scopes'
if ! grep -q "$ANCHOR" scripts/test_changed.sh; then
  fail "aborted-gate-never-exits-zero" "anchor '$ANCHOR' is gone from test_changed.sh"
else
  POISONED="$FIXTURE/poisoned_gate.sh"
  awk -v anchor="$ANCHOR" '
    index($0, anchor) == 1 && !done {
      print "declare -A GATE_SELFTEST_POISON=()"
      print "exit 0"
      done = 1
    }
    { print }
  ' scripts/test_changed.sh >"$POISONED"
  OUT=$(GATE_SELFTEST_RUNNING=1 TEST_CHANGED_LOG="$FIXTURE/poisoned.log" \
    "$OLD_BASH" "$POISONED" 2>&1)
  STATUS=$?
  if [ "$STATUS" -ne 0 ]; then
    pass "aborted-gate-never-exits-zero (exit $STATUS)"
  else
    fail "aborted-gate-never-exits-zero" \
      "a gate that ran no checks exited 0 — this is the false green. Output: $OUT"
  fi
fi

# ---- 6. the success path is not allowed to be silent ---------------------
# Whatever the mechanism, the gate must name the steps it ran and must carry
# an explicit completion tripwire; both are what makes property 2 checkable.
if grep -q 'steps:' scripts/test_changed.sh; then
  pass "success-names-its-steps"
else
  fail "success-names-its-steps" "the summary line does not list the steps that ran"
fi
if scan_file scripts/test_changed.sh -E 'trap[[:space:]]+[^#]*[[:space:]]EXIT'; then
  pass "completion-tripwire-present"
else
  fail "completion-tripwire-present" "no EXIT trap guards the premature-exit path"
fi

echo
if [ "$FAILURES" -eq 0 ]; then
  echo "gate self-test: ALL GREEN (oldest bash exercised: $OLD_BASH_VERSION)"
  exit 0
fi
echo "gate self-test: $FAILURES FAILED" >&2
exit 1

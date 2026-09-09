#!/usr/bin/env bash
# Checked runner for the owner-session / exact-effect test manifest
# (scripts/owner-effect-tests.tsv). The owner-session implementation plan
# requires that no RED, GREEN, checkpoint, or final block can report a vacuous
# success, so this runner refuses to infer anything:
#
#   - a row's `profile` column is the literal flag set the command must carry;
#     the runner never supplies a default feature set;
#   - Cargo's own "no such feature" / "no target named" diagnostics are
#     failures, not noise — they mean the command did not run what was asked;
#   - zero executed tests, a filtered-to-nothing run, or a target skipped for a
#     missing required-feature is a failure, because each looks like success;
#   - on a nonzero run, the task-specific RED diagnostic must appear, so a test
#     that fails for an unrelated reason cannot be read as the expected RED;
#   - `--require-profile <p>` (repeatable) makes an `--all` run fail unless the
#     manifest's profile set is exactly the profiles named, so the final
#     command declares its complete feature set instead of inheriting one.
#
# Portability: bash 3.2 (stock macOS) — no associative arrays, no `mapfile`.
#
# Honesty rules this runner itself obeys (backlog lesson 8):
#   - an EXIT-trap completion marker, so a premature abort cannot exit 0;
#   - a run that executed zero rows fails rather than reporting success.
set -euo pipefail

if [ -z "${BASH_VERSINFO:-}" ] || [ "${BASH_VERSINFO[0]}" -lt 3 ] ||
  { [ "${BASH_VERSINFO[0]}" -eq 3 ] && [ "${BASH_VERSINFO[1]}" -lt 2 ]; }; then
  echo "unsupported bash: need 3.2 or newer, got ${BASH_VERSION:-non-bash shell}" >&2
  exit 3
fi

MANIFEST="${OWNER_EFFECT_MANIFEST:-scripts/owner-effect-tests.tsv}"
VALID_PROFILES="no-default-features owner-effect-fixture owner-effect-fixture,fault-injection fault-injection legacy-experimental"

completed=0
on_exit() {
  status=$?
  if [ "$completed" -eq 0 ] && [ "$status" -eq 0 ]; then
    echo "run-owner-effect-tests: INCOMPLETE — exited before finishing" >&2
    exit 1
  fi
}
trap on_exit EXIT

usage() {
  cat >&2 <<'USAGE'
usage: run-owner-effect-tests.sh --all [--require-profile <profile>]...
       run-owner-effect-tests.sh --task <n> [--red <diagnostic>] [--require-profile <p>]...

  --all              run every row in the manifest
  --task <n>         run only the rows whose task column is <n>
  --red <text>       expect failure; the text must appear in the output
  --require-profile  the manifest's profile set must be exactly these
USAGE
  exit 2
}

mode=""
task=""
red=""
required_profiles=""
while [ $# -gt 0 ]; do
  case "$1" in
    --all) mode="all" ;;
    --task) shift; [ $# -gt 0 ] || usage; mode="task"; task="$1" ;;
    --red) shift; [ $# -gt 0 ] || usage; red="$1" ;;
    --require-profile)
      shift; [ $# -gt 0 ] || usage
      required_profiles="$required_profiles $1" ;;
    *) usage ;;
  esac
  shift
done
[ -n "$mode" ] || usage

if [ ! -f "$MANIFEST" ]; then
  echo "run-owner-effect-tests: missing manifest $MANIFEST" >&2
  exit 1
fi

# --- read the manifest ----------------------------------------------------
rows=""
profiles_seen=""
while IFS=$'\t' read -r r_task r_package r_target r_profile r_test; do
  case "$r_task" in ''|'#'*|task) continue ;; esac
  if [ -z "$r_package" ] || [ -z "$r_target" ] || [ -z "$r_profile" ] || [ -z "$r_test" ]; then
    echo "run-owner-effect-tests: incomplete row for task $r_task" >&2
    exit 1
  fi
  case " $VALID_PROFILES " in
    *" $r_profile "*) ;;
    *) echo "run-owner-effect-tests: unknown profile '$r_profile' (row: $r_test)" >&2; exit 1 ;;
  esac
  case " $profiles_seen " in
    *" $r_profile "*) ;;
    *) profiles_seen="$profiles_seen $r_profile" ;;
  esac
  if [ "$mode" = "all" ] || [ "$r_task" = "$task" ]; then
    rows="$rows$r_task|$r_package|$r_target|$r_profile|$r_test
"
  fi
done < "$MANIFEST"

# --- the declared feature set must match the manifest exactly --------------
if [ -n "$required_profiles" ]; then
  for p in $profiles_seen; do
    case " $required_profiles " in
      *" $p "*) ;;
      *) echo "FAIL  manifest uses profile '$p', which --require-profile did not declare" >&2
         exit 1 ;;
    esac
  done
  for p in $required_profiles; do
    case " $profiles_seen " in
      *" $p "*) ;;
      *) echo "FAIL  --require-profile named '$p', which the manifest does not use" >&2
         exit 1 ;;
    esac
  done
fi

if [ -z "$rows" ]; then
  echo "run-owner-effect-tests: no rows selected — a run that tests nothing is a failure" >&2
  exit 1
fi

# --- list before executing, so the reader sees what was asked for ---------
echo "run-owner-effect-tests: registered rows"
printf '%s' "$rows" | while IFS='|' read -r t pkg tgt prof name; do
  [ -n "$t" ] || continue
  echo "  task $t  $pkg::$tgt  [$prof]  $name"
done

profile_flags() { # <profile> -> the literal flags a command must carry
  if [ "$1" = "no-default-features" ]; then
    echo "--no-default-features"
  else
    echo "--no-default-features --features $1"
  fi
}

executed=0
failures=0
printf '%s' "$rows" > "${TMPDIR:-/tmp}/owner-effect-rows.$$"
while IFS='|' read -r t pkg tgt prof name; do
  [ -n "$t" ] || continue
  flags=$(profile_flags "$prof")
  echo
  echo "== task $t: cargo test -p $pkg --test $tgt $flags -- --exact $name"
  set +e
  output=$(cargo test -p "$pkg" --test "$tgt" $flags -- --exact "$name" --nocapture 2>&1)
  status=$?
  set -e
  printf '%s\n' "$output"

  # Cargo could not run what was asked. Never a pass, whatever the status.
  if printf '%s' "$output" | grep -Fq "does not have the feature" ||
     printf '%s' "$output" | grep -Fq "no target named" ||
     printf '%s' "$output" | grep -Fq "error: package ID specification" ||
     printf '%s' "$output" | grep -Fq "did not match any targets"; then
    echo "FAIL  Cargo rejected the profile or target for $name"
    failures=$((failures + 1))
    continue
  fi
  # A target skipped for a missing required-feature prints nothing and exits 0.
  if printf '%s' "$output" | grep -Fq "skipping due to unsatisfied required-features"; then
    echo "FAIL  $tgt was skipped for unsatisfied required-features"
    failures=$((failures + 1))
    continue
  fi
  # Zero executed, or filtered to nothing, both look exactly like success.
  if printf '%s' "$output" | grep -Eq "^test result: ok\. 0 passed"; then
    echo "FAIL  $name matched no test — the filter or the name is wrong"
    failures=$((failures + 1))
    continue
  fi
  if ! printf '%s' "$output" | grep -Eq "^test result:"; then
    echo "FAIL  no test result line — nothing ran"
    failures=$((failures + 1))
    continue
  fi

  executed=$((executed + 1))
  if [ "$status" -ne 0 ]; then
    if [ -z "$red" ]; then
      echo "FAIL  $name failed and no --red diagnostic was expected"
      failures=$((failures + 1))
    elif printf '%s' "$output" | grep -Fq -- "$red"; then
      echo "RED   $name failed with the expected diagnostic"
    else
      echo "FAIL  $name failed, but not with the expected RED diagnostic: $red"
      failures=$((failures + 1))
    fi
  elif [ -n "$red" ]; then
    echo "FAIL  $name passed, but a RED diagnostic was expected: $red"
    failures=$((failures + 1))
  fi
done < "${TMPDIR:-/tmp}/owner-effect-rows.$$"
rm -f "${TMPDIR:-/tmp}/owner-effect-rows.$$"

echo
if [ "$executed" -eq 0 ]; then
  echo "run-owner-effect-tests: executed nothing — that is a failure, not a pass" >&2
  exit 1
fi
if [ "$failures" -ne 0 ]; then
  echo "run-owner-effect-tests: FAILED ($failures of $executed rows)" >&2
  exit 1
fi

completed=1
echo "run-owner-effect-tests: OK — $executed row(s), profiles:$profiles_seen"

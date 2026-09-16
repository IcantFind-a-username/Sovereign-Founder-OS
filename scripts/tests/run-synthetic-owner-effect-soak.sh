#!/usr/bin/env bash
# Self-test for scripts/run-synthetic-owner-effect-soak.sh.
#
# The soak runner exists to stop a vacuous cargo success from being read as
# a pass. Each case here feeds the real runner a real transcript through a
# stub `cargo` on PATH.
#
# Portability: bash 3.2 (stock macOS).
set -euo pipefail

if [ -z "${BASH_VERSINFO:-}" ] || [ "${BASH_VERSINFO[0]}" -lt 3 ] ||
  { [ "${BASH_VERSINFO[0]}" -eq 3 ] && [ "${BASH_VERSINFO[1]}" -lt 2 ]; }; then
  echo "unsupported bash: need 3.2 or newer, got ${BASH_VERSION:-non-bash shell}" >&2
  exit 3
fi

RUNNER="$(cd "$(dirname "$0")/.." && pwd)/run-synthetic-owner-effect-soak.sh"
[ -x "$RUNNER" ] || { echo "missing $RUNNER" >&2; exit 1; }

completed=0
on_exit() {
  status=$?
  if [ "$completed" -eq 0 ] && [ "$status" -eq 0 ]; then
    echo "run-synthetic-owner-effect-soak self-test: INCOMPLETE — exited before finishing" >&2
    exit 1
  fi
}
trap on_exit EXIT

WORK=$(mktemp -d)
cleanup() { rm -rf "$WORK"; }
trap 'cleanup; on_exit' EXIT

checked=0
failed=0

write_echo_stub() { # <dir> <body-file>
  bin="$1/bin"
  mkdir -p "$bin"
  {
    echo '#!/usr/bin/env bash'
    echo "cat <<'TRANSCRIPT'"
    cat "$2"
    echo 'TRANSCRIPT'
    echo 'exit 0'
  } > "$bin/cargo"
  chmod +x "$bin/cargo"
}

write_exact_ok_stub() { # <dir>
  bin="$1/bin"
  mkdir -p "$bin"
  cat > "$bin/cargo" <<'STUB'
#!/usr/bin/env bash
name=""
prev=""
for arg in "$@"; do
  if [ "$prev" = "--exact" ]; then
    name="$arg"
  fi
  prev="$arg"
done
cat <<TRANSCRIPT
running 1 test
test ${name} ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
TRANSCRIPT
exit 0
STUB
  chmod +x "$bin/cargo"
}

run_case() { # <dir> <name> <expect: pass|fail>
  dir="$1"; name="$2"; expect="$3"
  checked=$((checked + 1))
  set +e
  PATH="$dir/bin:$PATH" \
    "$RUNNER" --crash-iterations 1 --race-iterations 1 \
    > "$dir/out" 2>&1
  status=$?
  set -e
  if [ "$expect" = "pass" ] && [ "$status" -ne 0 ]; then
    echo "FAIL  $name: expected accept, exited $status"
    sed 's/^/        /' "$dir/out"
    failed=1
  elif [ "$expect" = "fail" ] && [ "$status" -eq 0 ]; then
    echo "FAIL  $name: the runner accepted a transcript it must reject"
    sed 's/^/        /' "$dir/out"
    failed=1
  else
    echo "ok    $name"
  fi
}

checked=$((checked + 1))
set +e
"$RUNNER" --crash-iterations 0 --race-iterations 1 > "$WORK/zero-crash.out" 2>&1
status=$?
set -e
if [ "$status" -eq 0 ]; then
  echo "FAIL  crash-iterations 0 was accepted"
  failed=1
else
  echo "ok    crash-iterations 0 is refused"
fi

checked=$((checked + 1))
set +e
"$RUNNER" --crash-iterations 1 --race-iterations 0 > "$WORK/zero-race.out" 2>&1
status=$?
set -e
if [ "$status" -eq 0 ]; then
  echo "FAIL  race-iterations 0 was accepted"
  failed=1
else
  echo "ok    race-iterations 0 is refused"
fi

write_exact_ok_stub "$WORK/pass"
run_case "$WORK/pass" "a real passing 1/1 soak is accepted" pass

mkdir -p "$WORK/skip"
printf '%s\n' 'warning: skipping due to unsatisfied required-features: `owner-effect-fixture`' \
  > "$WORK/skip/body"
write_echo_stub "$WORK/skip" "$WORK/skip/body"
run_case "$WORK/skip" "a target skipped for required-features is refused" fail

mkdir -p "$WORK/zero"
printf '%s\n' 'running 0 tests' '' \
  'test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 8 filtered out; finished in 0.00s' \
  > "$WORK/zero/body"
write_echo_stub "$WORK/zero" "$WORK/zero/body"
run_case "$WORK/zero" "zero executed tests is refused" fail

mkdir -p "$WORK/missing"
printf '%s\n' 'running 1 test' 'test some_other_test ... ok' '' \
  'test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out' \
  > "$WORK/missing/body"
write_echo_stub "$WORK/missing" "$WORK/missing/body"
run_case "$WORK/missing" "a missing named test is refused" fail

mkdir -p "$WORK/noresult"
printf '%s\n' '   Compiling sovereign-synthetic-owner-effect v0.1.0' \
  '    Finished test profile in 0.4s' \
  > "$WORK/noresult/body"
write_echo_stub "$WORK/noresult" "$WORK/noresult/body"
run_case "$WORK/noresult" "output with no test result line is refused" fail

echo
if [ "$checked" -lt 7 ]; then
  echo "run-synthetic-owner-effect-soak self-test: only $checked scenarios ran; the suite is not intact" >&2
  exit 1
fi
if [ "$failed" -ne 0 ]; then
  echo "run-synthetic-owner-effect-soak self-test: FAILED" >&2
  exit 1
fi

completed=1
echo "run-synthetic-owner-effect-soak self-test: OK — $checked scenarios"

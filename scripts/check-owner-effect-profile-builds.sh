#!/usr/bin/env bash
# Every feature profile the owner-effect work uses must actually build.
#
# Cargo features are additive and unify across a build, which makes two
# opposite mistakes easy and both invisible day to day:
#
#   - a fixture module quietly becomes *required*, because something outside
#     the gate started referring to it. The default build still works on a
#     developer's machine, where the fixture feature has been on all day, and
#     breaks for everyone else;
#   - two features that are fine alone stop compiling together, because each
#     assumed it was the only one enabled. Nothing exercises the combination
#     until the first person needs both.
#
# The manifest names five profiles. This builds each one and fails on the
# first that does not, so "it compiles" is a checked claim about every
# combination rather than about whichever one was last used.
#
# It builds tests as well as libraries: a `required-features` target that no
# longer compiles is exactly as broken as a library that does not, and is
# easier to miss because nothing runs it.
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
    echo "check-owner-effect-profile-builds: INCOMPLETE — exited before finishing" >&2
    exit 1
  fi
}
trap on_exit EXIT

PACKAGES="-p sovereign-authority -p sovereign-owner -p sovereign-cli"

# The profiles the manifest's `profile` column may name, plus the default
# build and the two-feature combination nothing else exercises.
#
# Each line: <label>|<extra cargo flags>
PROFILES="default|
no-default-features|--no-default-features
fault-injection|--no-default-features --features fault-injection
owner-effect-fixture|--no-default-features --features owner-effect-fixture
owner-effect-fixture,fault-injection|--no-default-features --features owner-effect-fixture,fault-injection"

fail=0
checked=0

while IFS='|' read -r label flags; do
  [ -n "$label" ] || continue
  checked=$((checked + 1))
  printf '%-38s ' "$label"
  # `fault-injection` exists on authority only, so a profile naming it is
  # built for the package that has it rather than the whole set.
  case "$label" in
    *fault-injection*) packages="-p sovereign-authority" ;;
    *) packages="$PACKAGES" ;;
  esac
  # shellcheck disable=SC2086 — the flags are a deliberate word list.
  if cargo build $packages --all-targets $flags --locked >/dev/null 2>&1; then
    echo "ok"
  else
    echo "FAILED"
    echo "    cargo build $packages --all-targets $flags --locked" >&2
    # Re-run visibly so the failure is in the log rather than only its name.
    # shellcheck disable=SC2086
    cargo build $packages --all-targets $flags --locked 2>&1 | tail -20 >&2
    fail=1
  fi
done <<PROFILES_EOF
$PROFILES
PROFILES_EOF

echo
if [ "$checked" -lt 5 ]; then
  echo "check-owner-effect-profile-builds: only $checked profiles built" >&2
  exit 1
fi
if [ "$fail" -ne 0 ]; then
  echo "check-owner-effect-profile-builds: FAILED" >&2
  exit 1
fi

completed=1
echo "check-owner-effect-profile-builds: OK — $checked profiles"

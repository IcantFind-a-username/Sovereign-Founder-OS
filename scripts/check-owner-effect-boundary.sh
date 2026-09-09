#!/usr/bin/env bash
# The fixture must not be in anything that ships.
#
# Every module of RFC 0006's work is behind a non-default feature, and that is
# an arrangement rather than a guarantee. Cargo features are additive and
# unify across a build, so a single crate naming the feature in its own
# defaults — or one forwarded dependency edge — puts the whole fixture into
# the release binary, and nothing about a normal day's work would show it.
#
# So this checks the artefact three ways, each of which can fail while the
# others pass:
#
#   1. no package resolves either fixture feature in a default build;
#   2. the release binary contains none of the fixture's distinctive strings —
#      its hidden subcommand, its root marker, its canaries, its signer tag,
#      its routes. This catches a module that lost its `cfg` and so is
#      compiled in without any feature being enabled;
#   3. the fixture still builds when explicitly asked for, so the gate cannot
#      be satisfied by deleting the thing it guards.
#
# The strings in (2) were chosen to be ones that cannot occur by accident.
# A check for a common word would be noise, and noise gets ignored.
#
# What (2) can and cannot see, established by trying both: a fixture string
# on a reachable path is found, and one written into an unused constant is
# not, because the compiler removes it. That is the right boundary — an
# eliminated constant is not in the binary in any sense that matters — but it
# means this check answers "is the fixture reachable" and not "was fixture
# source ever written". Check (1) is what answers the second.
#
# Portability: bash 3.2 (stock macOS).
set -euo pipefail

if [ -z "${BASH_VERSINFO:-}" ] || [ "${BASH_VERSINFO[0]}" -lt 3 ] ||
  { [ "${BASH_VERSINFO[0]}" -eq 3 ] && [ "${BASH_VERSINFO[1]}" -lt 2 ]; }; then
  echo "unsupported bash: need 3.2 or newer, got ${BASH_VERSION:-non-bash shell}" >&2
  exit 3
fi

FEATURES="owner-effect-fixture
fault-injection"

# Distinctive by construction. Every one of these exists only inside the
# fixture, so a hit is unambiguous.
NEEDLES="__owner-effect-broker
synthetic-owner-effect-fixture-v1
synthetic-owner-effect-fixture-signer
SFO-FIXTURE-SUBJECT-CANARY
SFO-FIXTURE-BODY-CANARY
SFO-BOOTSTRAP-1
fixture-recipient@example.test
/fixture/register/start
.owner-effect-broker.lock"

BIN="target/release/sovereign"

completed=0
on_exit() {
  status=$?
  if [ "$completed" -eq 0 ] && [ "$status" -eq 0 ]; then
    echo "check-owner-effect-boundary: INCOMPLETE — exited before finishing" >&2
    exit 1
  fi
}
trap on_exit EXIT

fail=0
checked=0

# --- 1. the resolved default feature graph ---------------------------------
echo "== default feature graph"
tree=$(cargo tree --workspace --edges features --locked 2>&1) || {
  echo "check-owner-effect-boundary: cargo tree failed" >&2
  exit 2
}
while IFS= read -r feature; do
  [ -n "$feature" ] || continue
  checked=$((checked + 1))
  if printf '%s' "$tree" | grep -Fq "feature \"$feature\""; then
    echo "FAIL  a default build resolves $feature:"
    printf '%s' "$tree" | grep -F "feature \"$feature\"" | sed 's/^/        /'
    fail=1
  else
    echo "ok    $feature is absent from every default build"
  fi
done <<FEATURES_EOF
$FEATURES
FEATURES_EOF

# --- 2. the release artefact ------------------------------------------------
echo "== release binary"
if [ "${SKIP_RELEASE_BUILD:-0}" != "1" ]; then
  cargo build -p sovereign-cli --release --locked >/dev/null 2>&1 || {
    echo "check-owner-effect-boundary: release build failed" >&2
    exit 2
  }
fi
if [ ! -f "$BIN" ]; then
  echo "check-owner-effect-boundary: $BIN not built; nothing was inspected" >&2
  exit 1
fi
found=0
while IFS= read -r needle; do
  [ -n "$needle" ] || continue
  checked=$((checked + 1))
  set +e
  grep -aFq -- "$needle" "$BIN"
  status=$?
  set -e
  if [ "$status" -eq 0 ]; then
    echo "FAIL  the release binary contains '$needle'"
    fail=1
    found=$((found + 1))
  elif [ "$status" -ge 2 ]; then
    echo "check-owner-effect-boundary: scanner error looking for '$needle'" >&2
    exit 2
  fi
done <<NEEDLES_EOF
$NEEDLES
NEEDLES_EOF
if [ "$found" -eq 0 ]; then
  echo "ok    none of the fixture's strings appear in $BIN"
fi

# --- 3. the fixture still exists when asked for ----------------------------
echo "== the fixture builds when requested"
checked=$((checked + 1))
if cargo build -p sovereign-owner -p sovereign-authority \
  --no-default-features --features owner-effect-fixture --locked >/dev/null 2>&1; then
  echo "ok    the fixture builds under its own feature"
else
  echo "FAIL  the fixture no longer builds — it was removed, not excluded"
  fail=1
fi

echo
if [ "$checked" -lt 12 ]; then
  echo "check-owner-effect-boundary: only $checked checks ran" >&2
  exit 1
fi
if [ "$fail" -ne 0 ]; then
  echo "check-owner-effect-boundary: FAILED" >&2
  exit 1
fi

completed=1
echo "check-owner-effect-boundary: OK — $checked checks"

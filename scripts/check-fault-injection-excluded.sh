#!/usr/bin/env bash
# Prove that fault injection cannot reach a shipped artifact.
#
# `crates/authority` compiles named crash points under a `fault-injection`
# feature. A comment saying "non-default" is not evidence: Cargo features are
# additive and unify across a build, so one crate quietly forwarding the
# feature would put a barrier — and the environment variable that trips it —
# inside the release binary, where anything that can set an environment
# variable could stop the process mid-write.
#
# This gate checks the artifact rather than the intent, two ways:
#
#   1. the resolved feature graph of a default build must not contain
#      `fault-injection` for any package;
#   2. the actual release binary must not contain the barrier name or the
#      environment variable string.
#
# The second matters on its own: the first would still pass if someone added
# the module without the feature gate.
#
# Portability: bash 3.2 (stock macOS).
set -euo pipefail

if [ -z "${BASH_VERSINFO:-}" ] || [ "${BASH_VERSINFO[0]}" -lt 3 ] ||
  { [ "${BASH_VERSINFO[0]}" -eq 3 ] && [ "${BASH_VERSINFO[1]}" -lt 2 ]; }; then
  echo "unsupported bash: need 3.2 or newer, got ${BASH_VERSION:-non-bash shell}" >&2
  exit 3
fi

FEATURE="fault-injection"
BARRIER="LegacyAfterTempSyncBeforePublish"
BARRIER_ENV="SOVEREIGN_AUTHORITY_BARRIER"

completed=0
on_exit() {
  status=$?
  if [ "$completed" -eq 0 ] && [ "$status" -eq 0 ]; then
    echo "check-fault-injection-excluded: INCOMPLETE — exited before finishing" >&2
    exit 1
  fi
}
trap on_exit EXIT

fail=0
checked=0

# --- 1. the resolved feature graph of a default build ---------------------
# `cargo tree -e features` prints every feature Cargo actually resolved, which
# is the only place a forwarded feature becomes visible.
echo "== resolved feature graph (default build)"
checked=$((checked + 1))
tree=$(cargo tree --workspace --edges features --locked 2>&1) || {
  echo "check-fault-injection-excluded: cargo tree failed" >&2
  exit 2
}
if printf '%s' "$tree" | grep -Fq "feature \"$FEATURE\""; then
  echo "FAIL  a default build resolves the $FEATURE feature:"
  printf '%s' "$tree" | grep -F "feature \"$FEATURE\"" | sed 's/^/        /'
  fail=1
else
  echo "ok    no package pulls in $FEATURE by default"
fi

# --- 2. the artifact itself ------------------------------------------------
echo "== release binary"
BIN="target/release/sovereign"
if [ "${SKIP_RELEASE_BUILD:-0}" != "1" ]; then
  cargo build -p sovereign-cli --release --locked >/dev/null 2>&1 || {
    echo "check-fault-injection-excluded: release build failed" >&2
    exit 2
  }
fi
if [ ! -f "$BIN" ]; then
  # A gate that could not inspect anything must fail, never pass silently.
  echo "check-fault-injection-excluded: $BIN not built; nothing was inspected" >&2
  exit 1
fi

# `strings` is not on every host; `grep -a` over the binary is equivalent here
# and always available.
for needle in "$BARRIER" "$BARRIER_ENV"; do
  checked=$((checked + 1))
  if grep -aFq -- "$needle" "$BIN"; then
    echo "FAIL  the release binary contains '$needle'"
    fail=1
  else
    status=$?
    if [ "$status" -ge 2 ]; then
      echo "check-fault-injection-excluded: scanner error looking for '$needle'" >&2
      exit 2
    fi
    echo "ok    '$needle' is absent from $BIN"
  fi
done

# --- 3. the feature must still work when asked for explicitly --------------
# Otherwise this gate could be satisfied by deleting the mechanism, which is
# not the same as excluding it.
echo "== the feature still exists when requested"
checked=$((checked + 1))
if cargo build -p sovereign-authority --features "$FEATURE" --locked >/dev/null 2>&1; then
  echo "ok    $FEATURE builds when explicitly enabled"
else
  echo "FAIL  $FEATURE no longer builds — the mechanism was removed, not excluded"
  fail=1
fi

echo
if [ "$checked" -lt 4 ]; then
  echo "check-fault-injection-excluded: only $checked checks ran" >&2
  exit 1
fi
if [ "$fail" -ne 0 ]; then
  echo "check-fault-injection-excluded: FAILED" >&2
  exit 1
fi

completed=1
echo "check-fault-injection-excluded: OK — $checked checks"

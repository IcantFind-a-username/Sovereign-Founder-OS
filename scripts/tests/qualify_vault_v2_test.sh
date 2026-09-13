#!/usr/bin/env bash
# Self-test for scripts/qualify-vault-v2.sh — portability and fail-closed behavior.
set -uo pipefail

ROOT=$(cd "$(dirname "$0")/../.." && pwd)
WRAPPER="$ROOT/scripts/qualify-vault-v2.sh"
FAILURES=0

pass() { echo "qualify-vault-v2 self-test: PASS $1"; }
fail() { echo "qualify-vault-v2 self-test: FAIL $1" >&2; FAILURES=$((FAILURES + 1)); }

[ -x "$WRAPPER" ] || fail "wrapper is not executable"

# Zero arguments must not succeed.
if "$WRAPPER" >/dev/null 2>&1; then
  fail "zero-argument invocation exited 0"
else
  pass "zero-argument invocation is rejected"
fi

# Ambient shaping input must be rejected before Cargo runs.
if OPENSSL_DIR=/opt/attacker "$WRAPPER" cargo metadata -p sovereign-vault-v2-engine --frozen --offline >/dev/null 2>&1; then
  fail "OPENSSL_DIR was accepted"
else
  pass "OPENSSL_DIR is rejected"
fi

# Unknown mode must not succeed.
if "$WRAPPER" shell -c true >/dev/null 2>&1; then
  fail "unknown mode exited 0"
else
  pass "unknown mode is rejected"
fi

# Parses under the oldest bash we can find (gate portability contract).
OLD_BASH="${OLDEST_BASH:-}"
if [ -z "$OLD_BASH" ]; then
  for candidate in /bin/bash /usr/bin/bash bash; do
    if command -v "$candidate" >/dev/null 2>&1; then
      OLD_BASH=$("$candidate" -c 'echo "${BASH_VERSION}"' 2>/dev/null) && OLD_BASH=$candidate && break
    fi
  done
fi
if [ -n "$OLD_BASH" ] && "$OLD_BASH" -n "$WRAPPER" 2>/dev/null; then
  pass "parses under $OLD_BASH"
else
  fail "bash syntax check failed"
fi

if [ "$FAILURES" -eq 0 ]; then
  echo "qualify-vault-v2 self-test: ALL GREEN"
  exit 0
fi
echo "qualify-vault-v2 self-test: $FAILURES FAILED" >&2
exit 1

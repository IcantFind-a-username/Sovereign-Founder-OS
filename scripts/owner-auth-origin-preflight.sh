#!/usr/bin/env bash
# Wrapper for the RFC 0006 origin preflight.
#
#   ./scripts/owner-auth-origin-preflight.sh --virtual   # virtual authenticator
#   ./scripts/owner-auth-origin-preflight.sh --real      # attended, real key
#
# A `--virtual` run measures the protocol with a CDP virtual authenticator and
# produces `protocol_fixture_only` rows. A `--real` run needs a person and a
# real authenticator and produces `mechanism_qualified_only` rows; it is never
# run in CI. An empty real matrix is allowed and leaves the mechanism
# unqualified — see docs/security/owner-auth-mechanism-matrix.md.
#
# No production owner or session code is involved, and no remote script is
# fetched: the harness serves its own page from its own process.
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
    echo "owner-auth-origin-preflight: INCOMPLETE — exited before finishing" >&2
    exit 1
  fi
}
trap on_exit EXIT

MODE=""
OUT=""
while [ $# -gt 0 ]; do
  case "$1" in
    --virtual|--real) MODE="$1" ;;
    --out) shift; [ $# -gt 0 ] || { echo "--out needs a path" >&2; exit 2; }; OUT="$1" ;;
    *) echo "usage: owner-auth-origin-preflight.sh (--virtual | --real) [--out <file>]" >&2; exit 2 ;;
  esac
  shift
done
if [ -z "$MODE" ]; then
  echo "usage: owner-auth-origin-preflight.sh (--virtual | --real) [--out <file>]" >&2
  exit 2
fi

if ! command -v node >/dev/null 2>&1; then
  echo "owner-auth-origin-preflight: node is required (Node 22+, for its global WebSocket)" >&2
  exit 4
fi

# Node 22 or newer: the harness drives Chrome over the DevTools protocol using
# the global WebSocket, which older releases do not have.
major=$(node -p 'process.versions.node.split(".")[0]')
if [ "$major" -lt 22 ]; then
  echo "owner-auth-origin-preflight: need Node 22 or newer, found $(node -v)" >&2
  exit 4
fi

# The fixture port is frozen. If something already holds it the run would
# measure that process instead, which is worse than not running.
if command -v lsof >/dev/null 2>&1; then
  if lsof -nP -iTCP:7787 -sTCP:LISTEN >/dev/null 2>&1; then
    echo "owner-auth-origin-preflight: 127.0.0.1:7787 is already in use; stop it first" >&2
    exit 5
  fi
fi

HARNESS="$(cd "$(dirname "$0")" && pwd)/owner-auth-origin-preflight.mjs"
[ -f "$HARNESS" ] || { echo "missing $HARNESS" >&2; exit 1; }

set +e
if [ -n "$OUT" ]; then
  node "$HARNESS" "$MODE" | tee "$OUT"
  status=${PIPESTATUS[0]}
else
  node "$HARNESS" "$MODE"
  status=$?
fi
set -e

completed=1
case "$status" in
  0) echo "owner-auth-origin-preflight: PASS ($MODE)" ;;
  4) echo "owner-auth-origin-preflight: UNQUALIFIED — no browser on this platform" >&2 ;;
  *) echo "owner-auth-origin-preflight: FAIL ($MODE), exit $status" >&2 ;;
esac
exit "$status"

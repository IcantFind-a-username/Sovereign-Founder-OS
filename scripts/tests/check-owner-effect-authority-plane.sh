#!/usr/bin/env bash
# Self-test for scripts/check-owner-effect-authority-plane.sh.
#
# The real gate spends several cargo metadata/tree walks. These scenarios
# never do that: they put a stub `cargo` first on PATH and feed the real
# script the graphs it claims to judge. Only the one complete inverted
# shape may exit 0.
#
# Portability: bash 3.2 (stock macOS).
set -euo pipefail

if [ -z "${BASH_VERSINFO:-}" ] || [ "${BASH_VERSINFO[0]}" -lt 3 ] ||
  { [ "${BASH_VERSINFO[0]}" -eq 3 ] && [ "${BASH_VERSINFO[1]}" -lt 2 ]; }; then
  echo "unsupported bash: need 3.2 or newer, got ${BASH_VERSION:-non-bash shell}" >&2
  exit 3
fi

ROOT=$(cd "$(dirname "$0")/../.." && pwd) || exit 1
cd "$ROOT" || exit 1
GATE="$ROOT/scripts/check-owner-effect-authority-plane.sh"
[ -x "$GATE" ] || { echo "missing $GATE" >&2; exit 1; }

completed=0
on_exit() {
  status=$?
  if [ "$completed" -eq 0 ] && [ "$status" -eq 0 ]; then
    echo "check-owner-effect-authority-plane self-test: INCOMPLETE — exited before finishing" >&2
    exit 1
  fi
}
trap on_exit EXIT

WORK=$(mktemp -d "${TMPDIR:-/tmp}/sfo-authority-plane-selftest.XXXXXX")
cleanup() { rm -rf "$WORK"; }
trap 'cleanup; on_exit' EXIT

checked=0
failed=0

write_cargo_stub() { # <path> <mode>
  dest=$1
  mode=$2
  {
    echo '#!/usr/bin/env bash'
    echo "mode='$mode'"
    echo 'set -e'
    echo 'is_metadata=0'
    echo 'is_tree=0'
    echo 'pkg=""'
    echo 'prev=""'
    echo 'for arg in "$@"; do'
    echo '  case "$arg" in'
    echo '  metadata) is_metadata=1 ;;'
    echo '  tree) is_tree=1 ;;'
    echo '  esac'
    echo '  if [ "$prev" = "-p" ]; then pkg=$arg; fi'
    echo '  prev=$arg'
    echo 'done'
    echo 'if [ "$is_metadata" -eq 1 ]; then'
    echo '  case "$mode" in'
    echo '  empty-meta)'
    echo "    echo '{\"packages\":[]}'"
    echo '    exit 0'
    echo '    ;;'
    echo '  not-json)'
    echo "    echo 'not-json'"
    echo '    exit 0'
    echo '    ;;'
    echo '  cap-depends-auth)'
    echo "    cat <<'J'"
    echo '{"packages":['
    echo '{"name":"sovereign-capability","features":{"default":[]},"dependencies":[{"name":"sovereign-authority","kind":null}]},'
    echo '{"name":"sovereign-authority","features":{"default":[]},"dependencies":[]},'
    echo '{"name":"sovereign-cli","features":{"default":[]},"dependencies":[]}'
    echo ']}'
    echo 'J'
    echo '    exit 0'
    echo '    ;;'
    echo '  no-reverse)'
    echo "    cat <<'J'"
    echo '{"packages":['
    echo '{"name":"sovereign-capability","features":{"default":[]},"dependencies":[]},'
    echo '{"name":"sovereign-authority","features":{"default":[]},"dependencies":[]},'
    echo '{"name":"sovereign-cli","features":{"default":[]},"dependencies":[]}'
    echo ']}'
    echo 'J'
    echo '    exit 0'
    echo '    ;;'
    echo '  metadata-fails)'
    echo '    echo "error: stub metadata failed" >&2'
    echo '    exit 101'
    echo '    ;;'
    echo '  *)'
    echo "    cat <<'J'"
    echo '{"packages":['
    echo '{"name":"sovereign-capability","features":{"default":[],"owner-effect-fixture":[]},"dependencies":[]},'
    echo '{"name":"sovereign-authority","features":{"default":[],"owner-effect-fixture":["sovereign-capability/owner-effect-fixture"]},"dependencies":[{"name":"sovereign-capability","kind":null}]},'
    echo '{"name":"sovereign-cli","features":{"default":[],"owner-effect-fixture":["sovereign-authority/owner-effect-fixture"]},"dependencies":[]}'
    echo ']}'
    echo 'J'
    echo '    exit 0'
    echo '    ;;'
    echo '  esac'
    echo 'fi'
    echo 'if [ "$is_tree" -eq 1 ]; then'
    echo '  if [ "$mode" = "tree-fails" ]; then'
    echo '    echo "error: stub tree failed" >&2'
    echo '    exit 101'
    echo '  fi'
    echo '  if [ "$mode" = "both-planes" ] && [ "$pkg" = "sovereign-cli" ]; then'
    echo "    cat <<'T'"
    echo 'sovereign-cli v0.1.0'
    echo '├── sovereign-capability v0.1.0'
    echo '│   └── sovereign-capability feature "owner-effect-fixture"'
    echo '└── sovereign-authority v0.1.0'
    echo '    ├── sovereign-authority feature "owner-effect-fixture"'
    echo '    └── sovereign-authority feature "legacy-experimental"'
    echo 'T'
    echo '    exit 0'
    echo '  fi'
    echo '  if [ "$mode" = "cap-tree-edge" ] && [ "$pkg" = "sovereign-capability" ]; then'
    echo "    cat <<'T'"
    echo 'sovereign-capability v0.1.0'
    echo '└── sovereign-authority v0.1.0'
    echo 'T'
    echo '    exit 0'
    echo '  fi'
    echo '  if [ "$mode" = "cli-nested-edge" ] && [ "$pkg" = "sovereign-cli" ]; then'
    echo "    cat <<'T'"
    echo 'sovereign-cli v0.1.0'
    echo '└── sovereign-capability v0.1.0'
    echo '    └── sovereign-authority v0.1.0'
    echo 'T'
    echo '    exit 0'
    echo '  fi'
    echo '  case "$pkg" in'
    echo '  sovereign-capability)'
    echo "    cat <<'T'"
    echo 'sovereign-capability v0.1.0'
    echo '└── sovereign-contracts v0.1.0'
    echo 'T'
    echo '    ;;'
    echo '  sovereign-authority)'
    echo "    cat <<'T'"
    echo 'sovereign-authority v0.1.0'
    echo '└── sovereign-capability v0.1.0'
    echo 'T'
    echo '    ;;'
    echo '  *)'
    echo "    cat <<'T'"
    echo 'sovereign-cli v0.1.0'
    echo '├── sovereign-capability v0.1.0'
    echo '└── sovereign-authority v0.1.0'
    echo '    └── sovereign-capability v0.1.0 (*)'
    echo 'T'
    echo '    ;;'
    echo '  esac'
    echo '  exit 0'
    echo 'fi'
    echo 'echo "stub cargo: unexpected invocation: $*" >&2'
    echo 'exit 2'
  } >"$dest"
  chmod +x "$dest"
}

scenario() { # <name> <mode> <expect: pass|fail>
  name=$1
  mode=$2
  expect=$3
  checked=$((checked + 1))

  bin="$WORK/$checked/bin"
  mkdir -p "$bin"
  write_cargo_stub "$bin/cargo" "$mode"

  set +e
  PATH="$bin:$PATH" "$GATE" >"$WORK/$checked/out" 2>&1
  status=$?
  set -e

  if [ "$expect" = "pass" ] && [ "$status" -ne 0 ]; then
    echo "FAIL  $name: expected the gate to accept this, it exited $status"
    sed 's/^/        /' "$WORK/$checked/out"
    failed=1
  elif [ "$expect" = "fail" ] && [ "$status" -eq 0 ]; then
    echo "FAIL  $name: the gate accepted a shape it must reject"
    sed 's/^/        /' "$WORK/$checked/out"
    failed=1
  else
    echo "ok    $name"
  fi
}

static_require() { # <needle>
  checked=$((checked + 1))
  if ! grep -Fq -- "$1" "$GATE"; then
    echo "FAIL  gate no longer contains: $1"
    failed=1
  else
    echo "ok    static: $1"
  fi
}

static_require 'cargo metadata --no-deps --format-version 1 --locked'
static_require '--features sovereign-cli/owner-effect-fixture'
static_require 'cargo tree -p sovereign-cli -e features'
static_require 'cargo tree -p sovereign-capability'
static_require 'with_authority_store('
static_require 'trap on_exit EXIT'
static_require 'completed=1'
static_require 'legacy-experimental'

if ! grep -v '^[[:space:]]*#' "$GATE" | grep -Eq 'trap[[:space:]]+[^#]*[[:space:]]EXIT'; then
  echo "FAIL  no EXIT trap guards the premature-exit path"
  failed=1
fi
checked=$((checked + 1))
echo "ok    static: EXIT trap is live"

scenario "an inverted graph is accepted" happy pass
scenario "capability → authority metadata is refused" cap-depends-auth fail
scenario "missing authority → capability is refused" no-reverse fail
scenario "empty metadata is refused" empty-meta fail
scenario "non-JSON metadata is refused" not-json fail
scenario "a failed metadata invocation is refused" metadata-fails fail
scenario "a failed tree invocation is refused" tree-fails fail
scenario "simultaneous fixture+legacy features are refused" both-planes fail
scenario "capability package tree still naming authority is refused" cap-tree-edge fail
scenario "cli tree nesting authority under capability is refused" cli-nested-edge fail

checked=$((checked + 1))
ANCHOR='# ---- metadata / tree profiles'
if ! grep -q "$ANCHOR" "$GATE"; then
  echo "FAIL  aborted-gate-never-exits-zero: anchor '$ANCHOR' is gone"
  failed=1
else
  poisoned="$WORK/poisoned.sh"
  awk -v anchor="$ANCHOR" '
    index($0, anchor) == 1 && !done {
      print
      print "exit 0"
      done = 1
      next
    }
    { print }
  ' "$GATE" >"$poisoned"
  chmod +x "$poisoned"
  set +e
  PATH="$WORK/poison-bin:$PATH" "$poisoned" >"$WORK/poisoned.out" 2>&1
  status=$?
  set -e
  if [ "$status" -eq 0 ]; then
    echo "FAIL  a gate that ran no checks exited 0 — this is the false green"
    sed 's/^/        /' "$WORK/poisoned.out"
    failed=1
  else
    echo "ok    aborted-gate-never-exits-zero (exit $status)"
  fi
fi

echo
if [ "$checked" -lt 18 ]; then
  echo "check-owner-effect-authority-plane self-test: only $checked checks ran; the suite is not intact" >&2
  exit 1
fi
if [ "$failed" -ne 0 ]; then
  echo "check-owner-effect-authority-plane self-test: FAILED" >&2
  exit 1
fi

completed=1
echo "check-owner-effect-authority-plane self-test: OK — $checked checks"

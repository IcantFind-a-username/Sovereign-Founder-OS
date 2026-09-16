#!/usr/bin/env bash
# v2 fixture boundary: the upper crate must not enter a product graph.
#
# The v1 sibling (check-owner-effect-boundary.sh) watches the non-default
# feature that still lives on authority/CLI. This script watches the new
# publish=false package created by v01-D02:
#
#   1. product cargo tree and metadata contain no fixture package, redb from
#      that package, or WebAuthn;
#   2. the upper crate depends on capability and authority, with no cycle;
#   3. source inventory has one listener and no internal transport module;
#   4. the product release binary contains none of this crate's strings;
#   5. the fixture still builds when its own feature is requested.
#
# Portability: bash 3.2 (stock macOS).
# Honesty (backlog lesson 8): EXIT-trap completion marker; a run that
# inspected nothing fails rather than printing OK.
set -euo pipefail

if [ -z "${BASH_VERSINFO:-}" ] || [ "${BASH_VERSINFO[0]}" -lt 3 ] ||
  { [ "${BASH_VERSINFO[0]}" -eq 3 ] && [ "${BASH_VERSINFO[1]}" -lt 2 ]; }; then
  echo "unsupported bash: need 3.2 or newer, got ${BASH_VERSION:-non-bash shell}" >&2
  exit 3
fi

ROOT=$(cd "$(dirname "$0")/.." && pwd) || exit 1
cd "$ROOT" || exit 1

if ! command -v python3 >/dev/null 2>&1; then
  echo "check-synthetic-owner-effect-boundary: python3 is required to parse cargo metadata" >&2
  exit 2
fi

PACKAGE="sovereign-synthetic-owner-effect"
SRC="crates/synthetic-owner-effect/src"
BIN="target/release/sovereign"

NEEDLES="sovereign-synthetic-owner-effect-v2-boundary
sovereign-synthetic-owner-effect
webauthn-rs
webauthn_rs"

completed=0
checked=0

on_exit() {
  status=$?
  if [ "$completed" -eq 0 ] && [ "$status" -eq 0 ]; then
    echo "check-synthetic-owner-effect-boundary: INCOMPLETE — exited before finishing" >&2
    exit 1
  fi
}
trap on_exit EXIT

fail() {
  echo "check-synthetic-owner-effect-boundary: FAIL — $1" >&2
  exit 1
}

# ---- 1. product cargo tree ------------------------------------------------
echo "== product cargo tree"
set +e
tree=$(cargo tree -p sovereign-cli --locked 2>&1)
tree_status=$?
set -e
checked=$((checked + 1))
if [ "$tree_status" -ne 0 ]; then
  printf '%s\n' "$tree" >&2
  fail "cargo tree -p sovereign-cli --locked failed"
fi
[ -n "$tree" ] || fail "cargo tree -p sovereign-cli produced no output"

for needle in "$PACKAGE" "redb" "webauthn"; do
  checked=$((checked + 1))
  if printf '%s\n' "$tree" | grep -Eq -- "$needle"; then
    printf '%s\n' "$tree" | grep -E -- "$needle" | sed 's/^/        /' >&2
    fail "product cargo tree contains $needle"
  fi
  echo "ok    product tree has no $needle"
done

# ---- 2. metadata seam / no cycle -----------------------------------------
echo "== cargo metadata seam"
set +e
meta=$(cargo metadata --no-deps --format-version 1 --locked 2>&1)
meta_status=$?
set -e
checked=$((checked + 1))
if [ "$meta_status" -ne 0 ]; then
  printf '%s\n' "$meta" >&2
  fail "cargo metadata --no-deps failed"
fi

printf '%s\n' "$meta" | python3 -c '
import json, sys
meta = json.load(sys.stdin)
packages = meta.get("packages")
if not isinstance(packages, list) or not packages:
    sys.stderr.write("FAIL  metadata listed no packages\n")
    sys.exit(1)
by_name = {p.get("name"): p for p in packages if isinstance(p, dict) and p.get("name")}
required = (
    "sovereign-synthetic-owner-effect",
    "sovereign-capability",
    "sovereign-authority",
    "sovereign-cli",
)
for name in required:
    if name not in by_name:
        sys.stderr.write("FAIL  metadata is missing package %s\n" % name)
        sys.exit(1)

def normal_deps(pkg, name):
    return [
        d for d in (by_name[pkg].get("dependencies") or [])
        if d.get("name") == name and (d.get("kind") or "normal") == "normal"
    ]

fixture = "sovereign-synthetic-owner-effect"
if not normal_deps(fixture, "sovereign-capability"):
    sys.stderr.write("FAIL  fixture does not depend on sovereign-capability\n")
    sys.exit(1)
if not normal_deps(fixture, "sovereign-authority"):
    sys.stderr.write("FAIL  fixture does not depend on sovereign-authority\n")
    sys.exit(1)
if not normal_deps(fixture, "sovereign-owner"):
    sys.stderr.write("FAIL  fixture does not depend on sovereign-owner\n")
    sys.exit(1)
if normal_deps("sovereign-capability", fixture):
    sys.stderr.write("FAIL  capability depends on the fixture crate\n")
    sys.exit(1)
if normal_deps("sovereign-authority", fixture):
    sys.stderr.write("FAIL  authority depends on the fixture crate\n")
    sys.exit(1)
if normal_deps("sovereign-cli", fixture):
    sys.stderr.write("FAIL  sovereign-cli depends on the fixture crate\n")
    sys.exit(1)
if normal_deps("sovereign-capability", "sovereign-authority"):
    sys.stderr.write("FAIL  capability -> authority edge returned\n")
    sys.exit(1)
print("ok    metadata: fixture sits above capability and authority; no cycle")
' || fail "metadata seam contract"

checked=$((checked + 1))

# publish = false is the release-graph exclusion, not a comment.
checked=$((checked + 1))
if ! grep -Eq '^publish = false' crates/synthetic-owner-effect/Cargo.toml; then
  fail "crates/synthetic-owner-effect/Cargo.toml is not publish = false"
fi
echo "ok    fixture package is publish = false"

# ---- 3. source inventory --------------------------------------------------
echo "== source inventory"
[ -d "$SRC" ] || fail "missing $SRC"
src_files=$(find "$SRC" -name '*.rs' -type f | sort)
[ -n "$src_files" ] || fail "source inventory found no .rs files"

checked=$((checked + 1))
set +e
bind_lines=$(grep -R --include='*.rs' -F 'TcpListener::bind' "$SRC" 2>/dev/null)
bind_status=$?
set -e
if [ "$bind_status" -ge 2 ]; then
  echo "check-synthetic-owner-effect-boundary: scanner error looking for TcpListener::bind" >&2
  exit 2
fi
bind_hits=$(printf '%s\n' "$bind_lines" | grep -c . || true)
if [ "$bind_hits" -ne 1 ]; then
  fail "source inventory expected one TcpListener::bind, found $bind_hits"
fi
echo "ok    one TcpListener::bind"

checked=$((checked + 1))
if [ ! -f "$SRC/listener.rs" ]; then
  fail "missing the one listener module $SRC/listener.rs"
fi
echo "ok    listener.rs exists"

for forbidden in 'hmac::' 'use hmac' '__owner-effect-broker' \
  'run_owner_effect_fixture_broker' 'mod supervisor' 'mod connections' \
  'mod protocol' 'Ipv4Addr::UNSPECIFIED' 'Ipv6Addr::UNSPECIFIED' \
  'BROKER_SUBCOMMAND'; do
  checked=$((checked + 1))
  set +e
  grep -R --include='*.rs' -nF -- "$forbidden" "$SRC" >/tmp/sfo-v2-src.$$ 2>/tmp/sfo-v2-src-err.$$
  scan=$?
  set -e
  if [ "$scan" -ge 2 ]; then
    cat /tmp/sfo-v2-src-err.$$ >&2
    rm -f /tmp/sfo-v2-src.$$ /tmp/sfo-v2-src-err.$$
    echo "check-synthetic-owner-effect-boundary: scanner error looking for '$forbidden'" >&2
    exit 2
  fi
  if [ "$scan" -eq 0 ]; then
    cat /tmp/sfo-v2-src.$$ >&2
    rm -f /tmp/sfo-v2-src.$$ /tmp/sfo-v2-src-err.$$
    fail "source still contains '$forbidden'"
  fi
  rm -f /tmp/sfo-v2-src.$$ /tmp/sfo-v2-src-err.$$
  echo "ok    source lacks $forbidden"
done

# ---- 4. product release symbols ------------------------------------------
echo "== product release symbols"
if [ "${SKIP_RELEASE_BUILD:-0}" != "1" ]; then
  cargo build -p sovereign-cli --release --locked >/dev/null 2>&1 || {
    fail "release build of sovereign-cli failed"
  }
fi
checked=$((checked + 1))
if [ ! -f "$BIN" ]; then
  fail "$BIN not built; nothing was inspected"
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
    echo "FAIL  the release binary contains '$needle'" >&2
    fail "release symbols contain $needle"
  elif [ "$status" -ge 2 ]; then
    echo "check-synthetic-owner-effect-boundary: scanner error looking for '$needle'" >&2
    exit 2
  fi
  echo "ok    release symbols lack $needle"
done <<NEEDLES_EOF
$NEEDLES
NEEDLES_EOF
if [ "$found" -eq 0 ]; then
  echo "ok    none of this crate's needles appear in $BIN"
fi

# ---- 5. the fixture still exists when asked for --------------------------
echo "== the fixture builds when requested"
checked=$((checked + 1))
if cargo build -p "$PACKAGE" --no-default-features \
  --features owner-effect-fixture --locked >/dev/null 2>&1; then
  echo "ok    $PACKAGE builds under owner-effect-fixture"
else
  fail "the fixture no longer builds — it was removed, not excluded"
fi

echo
if [ "$checked" -lt 16 ]; then
  echo "check-synthetic-owner-effect-boundary: only $checked checks ran" >&2
  exit 1
fi

completed=1
echo "check-synthetic-owner-effect-boundary: OK — $checked checks"

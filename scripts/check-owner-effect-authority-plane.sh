#!/usr/bin/env bash
# Fixture authority-plane graph/API gate (owner-session plan Task 10 remainder).
#
# Capability verification is a pure plane: it must not depend on
# sovereign-authority and must not carry with_authority_store. Authority may
# depend on capability (the inverted edge). This is the fixture graph check,
# not a product RP1-03 claim — the product still uses AuthorityStore from
# the CLI/broker path already on main.
#
# Three metadata profiles (adapted to features this repo actually declares)
# plus cargo tree -p sovereign-cli -e features for each. Source and
# public-symbol checks. Compile-fail targets run only when they exist.
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
  echo "check-owner-effect-authority-plane: python3 is required to parse cargo metadata" >&2
  exit 2
fi

completed=0
checked=0

on_exit() {
  status=$?
  if [ "$completed" -eq 0 ] && [ "$status" -eq 0 ]; then
    echo "check-owner-effect-authority-plane: INCOMPLETE — exited before finishing" >&2
    exit 1
  fi
}
trap on_exit EXIT

fail() {
  echo "check-owner-effect-authority-plane: FAIL — $1" >&2
  exit 1
}

# ---- feature inventory (adapt to what the repo actually declares) --------
echo "== declared features"
cli_toml=apps/cli/Cargo.toml
cap_toml=crates/capability/Cargo.toml
auth_toml=crates/authority/Cargo.toml
for f in "$cli_toml" "$cap_toml" "$auth_toml"; do
  [ -f "$f" ] || fail "missing $f"
  checked=$((checked + 1))
done

has_legacy=0
for f in "$cli_toml" "$cap_toml" "$auth_toml"; do
  if grep -Eq '^[[:space:]]*legacy-experimental[[:space:]]*=' "$f"; then
    has_legacy=1
  fi
done
checked=$((checked + 1))
if [ "$has_legacy" -eq 0 ]; then
  echo "ok    legacy-experimental is not declared (third profile is the absence check)"
else
  echo "ok    legacy-experimental is declared; the third metadata/tree profile will run"
fi

# Combined enablement in a feature list would merge the planes.
checked=$((checked + 1))
if grep -R --include='Cargo.toml' -nE 'owner-effect-fixture[[:space:]]*=[[:space:]]*\[[^]]*legacy-experimental' \
  apps crates >/dev/null 2>&1; then
  fail "a Cargo.toml enables owner-effect-fixture and legacy-experimental together"
fi
echo "ok    no Cargo.toml enables fixture and legacy together"

# ---- metadata / tree profiles --------------------------------------------
judge_metadata() { # <label>  (JSON on stdin)
  label=$1
  python3 -c '
import json, sys
label = sys.argv[1]
try:
    meta = json.load(sys.stdin)
except Exception as exc:
    sys.stderr.write("FAIL  %s metadata is not JSON: %s\n" % (label, exc))
    sys.exit(1)
packages = meta.get("packages")
if not isinstance(packages, list) or not packages:
    sys.stderr.write("FAIL  %s metadata listed no packages\n" % label)
    sys.exit(1)
by_name = {p.get("name"): p for p in packages if isinstance(p, dict) and p.get("name")}
for required in ("sovereign-capability", "sovereign-authority", "sovereign-cli"):
    if required not in by_name:
        sys.stderr.write("FAIL  %s metadata is missing package %s\n" % (label, required))
        sys.exit(1)

def deps(pkg, name):
    return [d for d in (by_name[pkg].get("dependencies") or []) if d.get("name") == name]

cap_auth = deps("sovereign-capability", "sovereign-authority")
if cap_auth:
    kinds = sorted({(d.get("kind") or "normal") for d in cap_auth})
    sys.stderr.write("FAIL  %s: sovereign-capability depends on sovereign-authority (%s)\n" % (label, ",".join(kinds)))
    sys.exit(1)

auth_cap = [d for d in deps("sovereign-authority", "sovereign-capability") if (d.get("kind") or "normal") == "normal"]
if not auth_cap:
    sys.stderr.write("FAIL  %s: sovereign-authority does not depend on sovereign-capability\n" % label)
    sys.exit(1)
print("ok    %s metadata: capability does not depend on authority; authority depends on capability" % label)
' "$label"
}

judge_cli_tree() { # <label> <tree>
  label=$1
  tree=$2
  [ -n "$tree" ] || fail "$label cargo tree produced no output"
  printf '%s\n' "$tree" | python3 -c '
import re, sys
label = sys.argv[1]
text = sys.stdin.read()
if not text.strip():
    sys.stderr.write("FAIL  %s tree is empty\n" % label)
    sys.exit(1)
if (re.search(r"feature \"owner-effect-fixture\"", text)
        and re.search(r"feature \"legacy-experimental\"", text)):
    sys.stderr.write("FAIL  %s tree resolves fixture and legacy-experimental together\n" % label)
    sys.exit(1)
# Under a sovereign-capability node, a more-indented authority is the old edge.
cap_indent = None
for line in text.splitlines():
    prefix = re.match(r"^[^A-Za-z0-9_-]*", line).group(0)
    stripped = line[len(prefix):]
    indent = len(prefix)
    if stripped.startswith("sovereign-capability"):
        cap_indent = indent
        continue
    if cap_indent is None:
        continue
    if indent <= cap_indent:
        cap_indent = None
        if stripped.startswith("sovereign-capability"):
            cap_indent = indent
        continue
    if stripped.startswith("sovereign-authority"):
        sys.stderr.write("FAIL  %s tree shows capability -> authority\n" % label)
        sys.exit(1)
' "$label" || fail "$label tree shows a forbidden edge or merged planes"
  echo "ok    $label tree: no capability → authority"
}

run_profile() { # <label> <metadata extra flags> <tree extra flags>
  label=$1
  meta_flags=$2
  tree_flags=$3
  echo "== profile $label"

  set +e
  # shellcheck disable=SC2086
  meta_out=$(cargo metadata --no-deps --format-version 1 --locked $meta_flags 2>&1)
  meta_status=$?
  set -e
  checked=$((checked + 1))
  if [ "$meta_status" -ne 0 ]; then
    printf '%s\n' "$meta_out" >&2
    fail "$label cargo metadata failed"
  fi
  printf '%s\n' "$meta_out" | judge_metadata "$label" || fail "$label metadata contract"
  checked=$((checked + 1))

  set +e
  # shellcheck disable=SC2086
  tree_out=$(cargo tree -p sovereign-cli -e features --locked $tree_flags 2>&1)
  tree_status=$?
  set -e
  checked=$((checked + 1))
  if [ "$tree_status" -ne 0 ]; then
    printf '%s\n' "$tree_out" >&2
    fail "$label cargo tree -p sovereign-cli failed"
  fi
  judge_cli_tree "$label" "$tree_out"
  checked=$((checked + 1))
}

run_profile "no-default-features" \
  "--no-default-features" \
  "--no-default-features"

run_profile "owner-effect-fixture" \
  "--no-default-features --features sovereign-cli/owner-effect-fixture" \
  "--no-default-features --features owner-effect-fixture"

if [ "$has_legacy" -eq 1 ]; then
  run_profile "legacy-experimental" \
    "--no-default-features --features sovereign-cli/legacy-experimental" \
    "--no-default-features --features legacy-experimental"
else
  checked=$((checked + 1))
  echo "ok    legacy-experimental profile skipped (feature not declared)"
fi

# Package-local trees pin the inverted edge without relying on CLI indent.
echo "== package trees"
set +e
cap_tree=$(cargo tree -p sovereign-capability -e normal --no-default-features --locked 2>&1)
cap_status=$?
set -e
checked=$((checked + 1))
if [ "$cap_status" -ne 0 ]; then
  printf '%s\n' "$cap_tree" >&2
  fail "cargo tree -p sovereign-capability failed"
fi
if printf '%s\n' "$cap_tree" | grep -Eq 'sovereign-authority'; then
  fail "sovereign-capability's tree still contains sovereign-authority"
fi
echo "ok    sovereign-capability tree has no sovereign-authority"

set +e
auth_tree=$(cargo tree -p sovereign-authority -e normal --no-default-features --locked 2>&1)
auth_status=$?
set -e
checked=$((checked + 1))
if [ "$auth_status" -ne 0 ]; then
  printf '%s\n' "$auth_tree" >&2
  fail "cargo tree -p sovereign-authority failed"
fi
if ! printf '%s\n' "$auth_tree" | grep -Eq 'sovereign-capability'; then
  fail "sovereign-authority's tree does not contain sovereign-capability"
fi
echo "ok    sovereign-authority tree contains sovereign-capability"

# ---- source / symbol checks ----------------------------------------------
echo "== source"
# Callable form only: comments may name the removed API.
src_scan() { # <needle> <paths...>
  needle=$1
  shift
  set +e
  grep -R --include='*.rs' -nF -- "$needle" "$@" >/tmp/sfo-plane-src.$$ 2>/tmp/sfo-plane-src-err.$$
  scan=$?
  set -e
  if [ "$scan" -ge 2 ]; then
    cat /tmp/sfo-plane-src-err.$$ >&2
    rm -f /tmp/sfo-plane-src.$$ /tmp/sfo-plane-src-err.$$
    echo "check-owner-effect-authority-plane: scanner error looking for '$needle'" >&2
    exit 2
  fi
  if [ "$scan" -eq 0 ]; then
    cat /tmp/sfo-plane-src.$$ >&2
    rm -f /tmp/sfo-plane-src.$$ /tmp/sfo-plane-src-err.$$
    fail "source still contains '$needle'"
  fi
  rm -f /tmp/sfo-plane-src.$$ /tmp/sfo-plane-src-err.$$
  echo "ok    source lacks $needle"
}

checked=$((checked + 1))
src_scan 'with_authority_store(' crates apps
checked=$((checked + 1))
src_scan 'fn with_authority_store' crates apps
checked=$((checked + 1))
src_scan 'use sovereign_authority' crates/capability
checked=$((checked + 1))
src_scan 'AuthorityStore' crates/capability
checked=$((checked + 1))
src_scan 'LegacyClaimCoordinator' crates apps
checked=$((checked + 1))
src_scan 'LegacyValidatedCapability' crates apps

# Authority's fixture feature must forward capability's same feature now
# that the reverse edge exists.
checked=$((checked + 1))
if ! grep -Fq 'sovereign-capability/owner-effect-fixture' "$auth_toml"; then
  fail "authority's owner-effect-fixture does not forward sovereign-capability/owner-effect-fixture"
fi
echo "ok    authority fixture feature forwards capability's same feature"

# ---- compile-fail (only if the target already exists) --------------------
echo "== compile-fail"
if grep -Fq 'name = "authority_plane_boundary"' apps/cli/Cargo.toml ||
  [ -f apps/cli/tests/authority_plane_boundary.rs ]; then
  set +e
  cf_out=$(cargo test -p sovereign-cli --test authority_plane_boundary \
    --no-default-features --locked -- --test-threads=1 2>&1)
  cf_status=$?
  set -e
  checked=$((checked + 1))
  printf '%s\n' "$cf_out"
  if [ "$cf_status" -ne 0 ]; then
    fail "authority_plane_boundary failed"
  fi
  echo "ok    authority_plane_boundary"
else
  checked=$((checked + 1))
  echo "ok    compile-fail target not registered (graph/source checks stand alone)"
fi

if [ "$checked" -lt 16 ]; then
  echo "check-owner-effect-authority-plane: only $checked checks ran" >&2
  exit 1
fi

completed=1
echo "check-owner-effect-authority-plane: OK — $checked checks"

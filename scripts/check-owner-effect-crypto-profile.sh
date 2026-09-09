#!/usr/bin/env bash
# Pin the crypto the fixture broker's supervisor MAC resolves to.
#
# The launch key is the only thing separating the parent from any other
# process that finds the open port, and `Mac::verify_slice` is the only thing
# that checks it. A silent bump of hmac, sha2, digest or crypto-common changes
# that code path without changing a line of this repository, so the reviewed
# graph is stated once, here, and the build is made to match it.
#
# Three checks, because they fail independently:
#
#   1. the manifest pins exact versions with `=`, so a caret range cannot
#      quietly float;
#   2. the lock file holds exactly those versions;
#   3. the graph Cargo actually resolves under the fixture feature is exactly
#      the reviewed set — no more crates, no fewer.
#
# The third is the one that matters. A manifest and a lock can both be right
# while a transitive dependency pulls in a second hasher.
#
# Redb, when it lands, is ACID. It is not authenticity or encryption, and
# nothing here should be read as claiming otherwise.
#
# Portability: bash 3.2 (stock macOS).
set -euo pipefail

if [ -z "${BASH_VERSINFO:-}" ] || [ "${BASH_VERSINFO[0]}" -lt 3 ] ||
  { [ "${BASH_VERSINFO[0]}" -eq 3 ] && [ "${BASH_VERSINFO[1]}" -lt 2 ]; }; then
  echo "unsupported bash: need 3.2 or newer, got ${BASH_VERSION:-non-bash shell}" >&2
  exit 3
fi

# The reviewed profile, frozen by RFC 0006. One line per crate, exact version.
REVIEWED="crypto-common 0.1.7
digest 0.10.7
hmac 0.12.1
sha2 0.10.9"

completed=0
on_exit() {
  status=$?
  if [ "$completed" -eq 0 ] && [ "$status" -eq 0 ]; then
    echo "check-owner-effect-crypto-profile: INCOMPLETE — exited before finishing" >&2
    exit 1
  fi
}
trap on_exit EXIT

fail=0
checked=0

# --- 1. the manifest pins exactly ------------------------------------------
echo "== manifest pins"
checked=$((checked + 1))
if grep -Fq 'hmac = { version = "=0.12.1"' crates/authority/Cargo.toml; then
  echo "ok    hmac is pinned with ="
else
  echo "FAIL  crates/authority/Cargo.toml must pin hmac as =0.12.1"
  fail=1
fi
checked=$((checked + 1))
if grep -Fq 'default-features = false' crates/authority/Cargo.toml; then
  echo "ok    hmac's default features are off"
else
  echo "FAIL  hmac must be taken with default-features = false"
  fail=1
fi
checked=$((checked + 1))
if grep -Fq 'redb = { version = "=4.1.0"' crates/authority/Cargo.toml; then
  echo "ok    redb is pinned with ="
else
  echo "FAIL  crates/authority/Cargo.toml must pin redb as =4.1.0"
  fail=1
fi

# --- 2. the lock holds those versions --------------------------------------
echo "== lock file"
while read -r crate version; do
  [ -n "$crate" ] || continue
  checked=$((checked + 1))
  # The two lines must be adjacent: a name followed by its own version.
  if grep -A 1 "^name = \"$crate\"\$" Cargo.lock | grep -Fq "version = \"$version\""; then
    echo "ok    $crate $version"
  else
    echo "FAIL  Cargo.lock does not hold $crate $version"
    actual=$(grep -A 1 "^name = \"$crate\"\$" Cargo.lock | grep '^version' | head -1)
    [ -n "$actual" ] && echo "        found: $actual"
    fail=1
  fi
done <<REVIEWED_EOF
$REVIEWED
REVIEWED_EOF

# --- 3. the resolved graph is exactly the reviewed set ---------------------
echo "== resolved graph under the fixture feature"
checked=$((checked + 1))
tree=$(cargo tree -p sovereign-authority --no-default-features \
  --features owner-effect-fixture -e normal --locked 2>&1) || {
  echo "check-owner-effect-crypto-profile: cargo tree failed" >&2
  exit 2
}
# Every crate in the reviewed set, and every crate whose name looks like a MAC
# or hash primitive, so an added one is caught rather than ignored.
resolved=$(printf '%s\n' "$tree" |
  sed 's/^[^a-zA-Z]*//' |
  grep -E '^(hmac|sha2|sha3|sha1|md-5|blake[0-9]*|digest|crypto-common|subtle|generic-array|hkdf|pbkdf2) v' |
  sed 's/ (\*)$//' |
  sed 's/^\([^ ]*\) v\([^ ]*\).*$/\1 \2/' |
  sort -u)
# generic-array and subtle are unavoidable transitive parts of the digest
# stack; they are named here so their presence is a recorded decision.
expected=$(printf '%s\ngeneric-array 0.14.7\nsubtle 2.6.1\n' "$REVIEWED" | sort -u)

extra=$(comm -23 <(printf '%s\n' "$resolved") <(printf '%s\n' "$expected") || true)
missing=$(comm -13 <(printf '%s\n' "$resolved") <(printf '%s\n' "$expected") || true)
if [ -z "$extra" ] && [ -z "$missing" ]; then
  echo "ok    the graph is exactly the reviewed set"
else
  [ -n "$extra" ] && { echo "FAIL  unreviewed crates in the MAC path:"; printf '%s\n' "$extra" | sed 's/^/        /'; }
  [ -n "$missing" ] && { echo "FAIL  reviewed crates missing or at another version:"; printf '%s\n' "$missing" | sed 's/^/        /'; }
  fail=1
fi

# --- 4. a default build must not carry any of it ---------------------------
echo "== default build"
checked=$((checked + 1))
default_tree=$(cargo tree -p sovereign-authority -e normal --locked 2>&1) || {
  echo "check-owner-effect-crypto-profile: cargo tree (default) failed" >&2
  exit 2
}
for crate in hmac redb; do
  checked=$((checked + 1))
  if printf '%s' "$default_tree" | grep -qE "^[^a-zA-Z]*$crate v"; then
    echo "FAIL  $crate is in the default dependency graph; it must be fixture-only"
    fail=1
  else
    echo "ok    $crate is absent from a default build"
  fi
done

echo
if [ "$checked" -lt 8 ]; then
  echo "check-owner-effect-crypto-profile: only $checked checks ran" >&2
  exit 1
fi
if [ "$fail" -ne 0 ]; then
  echo "check-owner-effect-crypto-profile: FAILED" >&2
  exit 1
fi

completed=1
echo "check-owner-effect-crypto-profile: OK — $checked checks"

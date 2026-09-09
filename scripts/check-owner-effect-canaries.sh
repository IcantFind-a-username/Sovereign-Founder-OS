#!/usr/bin/env bash
# Keep the fixture's corpus synthetic.
#
# The fixture persists to redb, which is ACID and unencrypted. Whatever it
# holds sits in a plaintext file. So a real address reaching the fixture would
# mean this repository had produced a small unencrypted store of somebody's
# contacts as a side effect of proving a protocol — and it would happen the
# ordinary way, someone pasting a real address into a test while debugging and
# not taking it out again.
#
# This is the check that notices. It scans the fixture's own sources for
# anything address-shaped and fails on any domain that is not permanently
# unregistrable, and it verifies the frozen constants still say what RFC 0006
# says they say.
#
# Reserved and therefore safe: `.test` and `.invalid` (RFC 2606, RFC 6761),
# and the `example.*` family. Nothing else may appear.
#
# Portability: bash 3.2 (stock macOS).
set -euo pipefail

if [ -z "${BASH_VERSINFO:-}" ] || [ "${BASH_VERSINFO[0]}" -lt 3 ] ||
  { [ "${BASH_VERSINFO[0]}" -eq 3 ] && [ "${BASH_VERSINFO[1]}" -lt 2 ]; }; then
  echo "unsupported bash: need 3.2 or newer, got ${BASH_VERSION:-non-bash shell}" >&2
  exit 3
fi

CORPUS="crates/authority/src/broker/corpus.rs"

# Where fixture data could be written by hand. Widened as the fixture grows.
SCANNED="crates/authority/src/broker
crates/authority/tests
crates/owner/src
crates/owner/tests
apps/cli/tests"

completed=0
on_exit() {
  status=$?
  if [ "$completed" -eq 0 ] && [ "$status" -eq 0 ]; then
    echo "check-owner-effect-canaries: INCOMPLETE — exited before finishing" >&2
    exit 1
  fi
}
trap on_exit EXIT

fail=0
checked=0
files=0

# --- 1. the frozen constants still say what the RFC says -------------------
echo "== frozen corpus"
if [ ! -f "$CORPUS" ]; then
  echo "check-owner-effect-canaries: missing $CORPUS; nothing was verified" >&2
  exit 1
fi
for expected in \
  'pub const RECIPIENT: &str = "fixture-recipient@example.test";' \
  'pub const SENDER: &str = "fixture-sender@example.test";'; do
  checked=$((checked + 1))
  if grep -Fq -- "$expected" "$CORPUS"; then
    echo "ok    $(echo "$expected" | sed 's/pub const //;s/: &str.*= /= /')"
  else
    echo "FAIL  $CORPUS must contain: $expected"
    fail=1
  fi
done

# --- 2. no address-shaped string at a registrable domain -------------------
echo "== address scan"
for directory in $SCANNED; do
  [ -d "$directory" ] || continue
  # Every .rs file under the fixture's own trees.
  while IFS= read -r file; do
    files=$((files + 1))
    checked=$((checked + 1))
    # Address-shaped: something@something.tld. grep's exit 2 is a scanner
    # error and must not read as "no matches".
    set +e
    hits=$(grep -oE '[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}' "$file")
    status=$?
    set -e
    if [ "$status" -ge 2 ]; then
      echo "check-owner-effect-canaries: scanner error on $file" >&2
      exit 2
    fi
    [ -n "$hits" ] || continue
    for address in $hits; do
      # Domains are case-insensitive, so the comparison must be too.
      lower=$(printf '%s' "$address" | tr 'A-Z' 'a-z')
      case "$lower" in
        # RFC 2606 and RFC 6761 reserve all of these permanently.
        *.test|*.invalid|*.example|*@example.com|*@example.org|*@example.net) ;;
        *)
          echo "FAIL  $file holds $address, at a domain someone can register"
          fail=1
          ;;
      esac
    done
  done <<FILES_EOF
$(find "$directory" -name '*.rs' -type f)
FILES_EOF
done

if [ "$files" -eq 0 ]; then
  # A scan that inspected nothing must fail, never pass.
  echo "check-owner-effect-canaries: scanned zero files" >&2
  exit 1
fi
echo "ok    $files fixture source files scanned"

echo
if [ "$fail" -ne 0 ]; then
  echo "check-owner-effect-canaries: FAILED" >&2
  exit 1
fi

completed=1
echo "check-owner-effect-canaries: OK — $checked checks over $files files"

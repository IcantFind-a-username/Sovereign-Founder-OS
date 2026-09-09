#!/usr/bin/env bash
# The documents must not claim more than the fixture demonstrates.
#
# Code gates catch code. This catches the other way the boundary erodes:
# somebody summarises the work in a sentence that is nearly right, the
# sentence gets quoted, and six months later a decision rests on it.
#
# Two halves, and both are needed. Forbidding overstatement alone would pass a
# document that simply stopped mentioning the limit; requiring the limit alone
# would pass one that stated it and then contradicted itself.
#
# The forbidden phrases are affirmative constructions that cannot appear in an
# honest sentence. "is not owner admission" is fine and required; "is owner
# admission" is not. That is why the patterns are this specific rather than a
# ban on the words themselves — a ban on the words would make the honest
# sentence unwritable, which is how a check ends up training people to say
# less.
#
# Portability: bash 3.2 (stock macOS).
set -euo pipefail

if [ -z "${BASH_VERSINFO:-}" ] || [ "${BASH_VERSINFO[0]}" -lt 3 ] ||
  { [ "${BASH_VERSINFO[0]}" -eq 3 ] && [ "${BASH_VERSINFO[1]}" -lt 2 ]; }; then
  echo "unsupported bash: need 3.2 or newer, got ${BASH_VERSION:-non-bash shell}" >&2
  exit 3
fi

QUALIFICATION="docs/security/synthetic-owner-exact-effect-fixture-qualification.md"
MATRIX="docs/security/owner-auth-mechanism-matrix.md"
RFC="rfcs/0006-synthetic-owner-session-exact-effect-fixture.md"

completed=0
on_exit() {
  status=$?
  if [ "$completed" -eq 0 ] && [ "$status" -eq 0 ]; then
    echo "check-owner-effect-documentation: INCOMPLETE — exited before finishing" >&2
    exit 1
  fi
}
trap on_exit EXIT

fail=0
checked=0

# Prose wraps, so a phrase that reads as one sentence is several lines in the
# file. Every scan runs against the document with newlines collapsed to
# spaces, or a required phrase would fail purely because of where it wrapped.
normalised() { tr '\n' ' ' < "$1" | tr -s ' '; }

scan() { # <file> <needle> -> 0 present, 1 absent, aborts on scanner error
  if normalised "$1" | grep -Fqi -- "$2"; then
    return 0
  fi
  status=$?
  if [ "$status" -ge 2 ]; then
    echo "check-owner-effect-documentation: scanner error on '$2'" >&2
    exit 2
  fi
  return 1
}

# An honest sentence can contain an affirmative phrase — "no row here is owner
# admission" is exactly the sentence this gate wants to see, and it contains
# "is owner admission". Grep cannot read, so the rule is: the phrase is only a
# failure in a sentence with no negator in it.
#
# This is a heuristic and is stated as one. It would miss "this is owner
# admission, and nothing is not". It catches the way overstatement is actually
# written, which is plainly and without qualification.
claims_affirmatively() { # <file> <needle>
  normalised "$1" |
    tr '.' '\n' |
    grep -Fi -- "$2" |
    grep -qvEi 'not|never|no |cannot|nothing|neither'
}

require() { # <file> <needle>
  checked=$((checked + 1))
  if scan "$1" "$2"; then
    echo "ok    $1 states: $2"
  else
    echo "FAIL  $1 must state: $2"
    fail=1
  fi
}

forbid_all() { # <needle> — no document anywhere may claim this affirmatively
  checked=$((checked + 1))
  found=""
  for file in "$QUALIFICATION" "$MATRIX" "$RFC" README.md THREAT_MODEL.md ROADMAP.md; do
    [ -f "$file" ] || continue
    if claims_affirmatively "$file" "$1"; then
      found="$found $file"
    fi
  done
  if [ -n "$found" ]; then
    echo "FAIL  claims '$1':$found"
    fail=1
  else
    echo "ok    nothing claims: $1"
  fi
}

for file in "$QUALIFICATION" "$MATRIX" "$RFC"; do
  if [ ! -f "$file" ]; then
    echo "check-owner-effect-documentation: missing $file; nothing was verified" >&2
    exit 1
  fi
done

# --- the limit must be stated, not merely not-contradicted -----------------
echo "== the honest boundary is stated"
require "$QUALIFICATION" "is not owner admission"
require "$QUALIFICATION" "can win the empty-registry enrolment"
require "$QUALIFICATION" "Obscurity is not admission"
require "$QUALIFICATION" "connection authentication, not provenance"
require "$QUALIFICATION" "accepted, not mitigated"
require "$QUALIFICATION" "real-authenticator matrix is **empty**"
require "$QUALIFICATION" "not encrypted and not authenticated"
# The matrix's own words. Written to match the document rather than the
# document rewritten to match the check: a gate that dictates phrasing makes
# every document read like the gate.
require "$MATRIX" "the mechanism is unqualified"
require "$MATRIX" "Nothing may cite this document as evidence"

# --- and no document may claim otherwise -----------------------------------
echo "== no document overstates"
forbid_all "is owner admission"
forbid_all "provides owner admission"
forbid_all "achieves owner admission"
forbid_all "hostile native bootstrap is defeated"
forbid_all "hostile-native bootstrap is defeated"
forbid_all "prevents a same-account process"
forbid_all "the mechanism is qualified"
forbid_all "ready for product use"
forbid_all "production ready"

# --- and there is no status a later edit can flip --------------------------
echo "== no activation transition"
checked=$((checked + 1))
if grep -Eqi '^\*\*?status:?\*\*?[[:space:]]*(qualified|active|enabled|production)' "$QUALIFICATION"; then
  echo "FAIL  $QUALIFICATION carries a status that can be flipped to enable product use"
  fail=1
else
  echo "ok    no flippable status field"
fi

echo
if [ "$checked" -lt 18 ]; then
  echo "check-owner-effect-documentation: only $checked checks ran" >&2
  exit 1
fi
if [ "$fail" -ne 0 ]; then
  echo "check-owner-effect-documentation: FAILED" >&2
  exit 1
fi

completed=1
echo "check-owner-effect-documentation: OK — $checked checks"

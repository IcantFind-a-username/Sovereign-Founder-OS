#!/usr/bin/env bash
# Contract gate for RFC 0006, the synthetic owner-session / exact-effect
# fixture. The owner-session implementation plan
# (docs/superpowers/plans/2026-08-14-owner-session-exact-effect-v1-implementation.md,
# Task 1) requires the RFC to freeze every Global Constraint BEFORE any
# fixture code is written, so a later task cannot quietly relax the boundary.
# This script is that freeze check: it fails unless the RFC states each
# load-bearing constraint, names the conjunctive future gates, and makes no
# affirmative product-activation claim.
#
# Portability: bash 3.2 (stock macOS) — no associative arrays, no `mapfile`.
# A contract gate that cannot run on a developer's machine is not a gate.
#
# Honesty rules this gate itself must obey (backlog lesson 8):
#   - a run that inspected nothing must fail, never pass;
#   - a scanner error (grep exit >= 2) is a failure, not a clean "no match";
#   - fixed-string matching (grep -F) so a stray metacharacter cannot turn a
#     real requirement into a regex that matches nothing and reads as a pass.
set -euo pipefail

if [ -z "${BASH_VERSINFO:-}" ] || [ "${BASH_VERSINFO[0]}" -lt 3 ] ||
  { [ "${BASH_VERSINFO[0]}" -eq 3 ] && [ "${BASH_VERSINFO[1]}" -lt 2 ]; }; then
  echo "unsupported bash: need 3.2 or newer, got ${BASH_VERSION:-non-bash shell}" >&2
  exit 3
fi

RFC="rfcs/0006-synthetic-owner-session-exact-effect-fixture.md"

# Completion marker: any exit before the final line prints "INCOMPLETE", so a
# premature `set -e` abort can never be mistaken for a pass.
completed=0
on_exit() {
  status=$?
  if [ "$completed" -eq 0 ] && [ "$status" -eq 0 ]; then
    echo "check-owner-effect-rfc: INCOMPLETE — exited before finishing" >&2
    exit 1
  fi
}
trap on_exit EXIT

if [ ! -f "$RFC" ]; then
  echo "missing $RFC" >&2
  exit 1
fi

# grep -F for fixed strings; a grep error (>=2) must fail, not read as absent.
present() { # <needle> -> 0 if present, 1 if absent, aborts on scanner error
  if grep -Fq -- "$1" "$RFC"; then
    return 0
  fi
  status=$?
  if [ "$status" -ge 2 ]; then
    echo "check-owner-effect-rfc: scanner error on '$1'" >&2
    exit 2
  fi
  return 1
}

fail=0
checked=0

require() { # <needle> — the RFC MUST contain this exact string
  checked=$((checked + 1))
  if ! present "$1"; then
    echo "FAIL  RFC 0006 must state: $1"
    fail=1
  fi
}

forbid() { # <needle> — the RFC MUST NOT contain this exact string
  checked=$((checked + 1))
  if present "$1"; then
    echo "FAIL  RFC 0006 must not claim: $1"
    fail=1
  fi
}

# --- Scope and the conjunctive future gates -------------------------------
require "synthetic"
require "fixture-recipient@example.test"
require "Program 1B1"
require "Program 1C1"
require "Program 1D"
require "ActiveV2"
require "protected-payload"
require "conjunctive"

# --- Owner-admission non-claims (the honest boundary) ---------------------
require "ProductOwnerAdmission"
require "empty-registry"
require "hostile"
require "not owner admission"

# --- Exact origin, cookie, and WebAuthn ceremony --------------------------
require "http://localhost:7787"
require "__Host-sfo_fixture_session"
require "user verification"
require "300"

# --- One-use approval issuer ----------------------------------------------
require "one-use"
require "fresh challenge"

# --- Broker / redb / IPC --------------------------------------------------
require "redb"
require "HMAC-SHA-256"
require "supervisor"
require "five-second"

# --- Effect state machine and evidence ------------------------------------
require "AuthorityReserved"
require "FailedBeforeDispatch"
require "Indeterminate"
require "value-free"

# --- Residuals stated honestly --------------------------------------------
require "cross-port"
require "not encrypted"
require "runtime-plaintext allowlist"

# --- No affirmative product-activation transition -------------------------
forbid "1C0 is complete"
forbid "Program 2 is complete"
forbid "production-ready"
forbid "product enrollment is available"
forbid "marks the workspace ActiveV2"

# A run that checked nothing is broken, not clean.
if [ "$checked" -eq 0 ]; then
  echo "FAIL  the gate inspected zero requirements — it did not actually run" >&2
  completed=1
  exit 1
fi

if [ "$fail" -eq 0 ]; then
  echo "OK    RFC 0006 states every required constraint ($checked checks)"
fi

completed=1
exit "$fail"

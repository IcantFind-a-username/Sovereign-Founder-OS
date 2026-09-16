#!/usr/bin/env bash
# Scoped quality gate: run only the checks that the current change set needs.
#
# Change set = committed diff vs merge-base with main
#            + staged and unstaged working-tree changes
#            + untracked files (git ls-files --others — easy to forget, so
#              a brand-new file with a failing test still gates the session).
#
# Output discipline (enforced here, not by convention):
#   - the full build/test log goes to .harness/test_changed.log (gitignored)
#   - success prints ONE summary line, naming every step that ran
#   - failure prints only the last 25 log lines
# so a large cargo log never floods an agent's context window.
#
# Scope mapping:
#   crates/<x>/**            -> cargo package sovereign-<x> (+ adversarial suite)
#   apps/cli/**/*.rs         -> sovereign-cli (+ adversarial suite)
#   apps/cli/assets/**       -> frontend type-check (tsc --checkJs, pinned)
#   tests/adversarial/**     -> sovereign-adversarial-tests
#   Cargo.toml / Cargo.lock / rust-toolchain.toml / scripts/ / .github/
#                            -> full workspace
#   docs, rfcs, markdown     -> no test run (fmt/file-size still checked)
#   a file targeted by #[path] from another package also queues the includer
#                            (no Cargo edge; reverse-dep rebuild is still out)
#
# Scoped runs do NOT rebuild reverse dependencies of a changed crate; CI runs
# the full workspace on every push and is the backstop for cross-crate breakage.
# #[path] includes are the exception: the target has no Cargo edge to the
# includer, so even a future reverse-dep scan would miss them.
#
# Two properties this script owes its callers (the Stop hook trusts its exit
# status, and scripts/tests/gate_portability_test.sh pins both):
#   - it runs on bash 3.2, the stock macOS shell: no associative arrays, no
#     `mapfile`, no case-modifying expansions;
#   - it never exits 0 without having run its checks. Any abort before the
#     completion marker is converted to a nonzero exit by the EXIT trap below.
set -uo pipefail

if [ -z "${BASH_VERSINFO:-}" ] || [ "${BASH_VERSINFO[0]}" -lt 3 ] ||
  { [ "${BASH_VERSINFO[0]}" -eq 3 ] && [ "${BASH_VERSINFO[1]}" -lt 2 ]; }; then
  echo "unsupported bash: need 3.2 or newer, got ${BASH_VERSION:-non-bash shell}" >&2
  exit 3
fi

# ---- never report success without having finished -------------------------
GATE_STATE=running # running -> failed (a check failed) | completed
STEPS_RUN=""
on_exit() {
  local status=$?
  if [ "$GATE_STATE" = "running" ]; then
    {
      echo "GATE ABORTED before finishing (raw exit status $status) —"
      echo "refusing to report success. Steps that had run:${STEPS_RUN:- none}"
    } >&2
    if [ "$status" -eq 0 ]; then
      status=1
    fi
    exit "$status"
  fi
}
trap on_exit EXIT

cd "$(git rev-parse --show-toplevel)" || exit 1

LOG="${TEST_CHANGED_LOG:-.harness/test_changed.log}"
mkdir -p "$(dirname "$LOG")"
: >"$LOG"

fail() {
  GATE_STATE=failed
  echo "GATE FAILED at step: $1 — last 25 lines of $LOG:" >&2
  tail -n 25 "$LOG" >&2
  exit 1
}

run_step() { # run_step <label> <cmd...>
  local label=$1
  shift
  echo "==== [$label] \$ $*" >>"$LOG"
  "$@" >>"$LOG" 2>&1 || fail "$label"
  STEPS_RUN="$STEPS_RUN $label"
}

# ---- collect the change set ---------------------------------------------
# TEST_CHANGED_BASE overrides the diff base (default: merge-base with main).
BASE=""
if [ -n "${TEST_CHANGED_BASE:-}" ]; then
  BASE=$(git rev-parse --verify "$TEST_CHANGED_BASE") || exit 1
else
  for ref in origin/main main; do
    if git rev-parse --verify -q "$ref" >/dev/null; then
      BASE=$(git merge-base HEAD "$ref" 2>/dev/null) && break
    fi
  done
fi

CHANGED=$(
  {
    [ -n "$BASE" ] && git diff --name-only "$BASE" HEAD
    git diff --name-only            # unstaged
    git diff --name-only --cached   # staged
    git ls-files --others --exclude-standard # untracked — do not drop this
  } | sort -u
)

# ---- map paths to scopes -------------------------------------------------
# PKGS is a space-delimited, space-padded list of cargo package names rather
# than an associative array, because bash 3.2 has none. The mapping itself
# lives in scripts/lib/map_changed_paths.sh so the self-test can observe
# scope without an env var that prints it and exits 0 (the premature-success
# path property 2 / self-test #5 exist to forbid).
. scripts/lib/map_changed_paths.sh || exit 1
map_changed_paths_to_scope || exit 1

# Same-image broker gate: a fresh CARGO_TARGET_DIR cannot use the session
# cache, so this is not always-on. Run it when the fixture image, its
# bootstrap tests, the feature edge, or the gate itself moved.
NEED_BROKER_BUILD=0
while IFS= read -r f; do
  case "$f" in
  scripts/check-owner-effect-broker-build.sh | scripts/tests/check-owner-effect-broker-build.sh)
    NEED_BROKER_BUILD=1
    ;;
  .github/workflows/owner-effect-fixture.yml | apps/cli/tests/broker_bootstrap.rs | apps/cli/src/main.rs | apps/cli/Cargo.toml)
    NEED_BROKER_BUILD=1
    ;;
  crates/authority/src/broker/* | crates/authority/Cargo.toml | Cargo.toml | Cargo.lock)
    NEED_BROKER_BUILD=1
    ;;
  esac
done <<EOF
$CHANGED
EOF

# Authority-plane graph gate: cargo metadata + tree is not cheap enough
# for every session. Run it when the capability/authority manifests, the
# plane script, or the capability verifier sources move.
NEED_AUTHORITY_PLANE=0
while IFS= read -r f; do
  case "$f" in
  scripts/check-owner-effect-authority-plane.sh | scripts/tests/check-owner-effect-authority-plane.sh)
    NEED_AUTHORITY_PLANE=1
    ;;
  crates/capability/Cargo.toml | crates/authority/Cargo.toml | Cargo.toml | Cargo.lock)
    NEED_AUTHORITY_PLANE=1
    ;;
  crates/capability/src/* | crates/authority/src/lib.rs)
    NEED_AUTHORITY_PLANE=1
    ;;
  esac
done <<EOF
$CHANGED
EOF

# any runtime-crate change also runs the cross-crate security invariants
if [ -n "$PKGS" ]; then
  add_pkg "sovereign-adversarial-tests"
fi

# ---- always-on cheap gates ----------------------------------------------
# These run even when the tree is clean: "nothing changed" is not a reason to
# exit 0 without checking anything, and the cost is seconds.
if [ "${GATE_SELFTEST_RUNNING:-0}" != "1" ]; then
  run_step "gate-self-test" env GATE_SELFTEST_RUNNING=1 \
    ./scripts/tests/gate_portability_test.sh
  # The owner-effect runners exist to stop a vacuous cargo success from being
  # read as a pass. A checked runner nobody checks is worth nothing, so their
  # self-tests run here, on every session, not only when someone remembers.
  run_step "owner-effect-runner-selftests" env GATE_SELFTEST_RUNNING=1 \
    ./scripts/tests/run-owner-effect-tests.sh
  run_step "synthetic-owner-effect-soak-selftest" env GATE_SELFTEST_RUNNING=1 \
    ./scripts/tests/run-synthetic-owner-effect-soak.sh
  run_step "owner-effect-regression-selftest" env GATE_SELFTEST_RUNNING=1 \
    ./scripts/tests/run-owner-effect-regression.sh
  # The same-target broker build is two clean CARGO_TARGET_DIR compiles;
  # that is not cheap enough for every session. Its self-test (stub cargo,
  # no rustc) is, and is what keeps the gate from rotting into a script
  # nobody runs. The real build is queued below when fixture sources move.
  run_step "owner-effect-broker-build-selftest" env GATE_SELFTEST_RUNNING=1 \
    ./scripts/tests/check-owner-effect-broker-build.sh
  # Same shape as the broker-build self-test: stub cargo, no rustc. The
  # real metadata/tree walk is queued below when plane sources move.
  run_step "owner-effect-authority-plane-selftest" env GATE_SELFTEST_RUNNING=1 \
    ./scripts/tests/check-owner-effect-authority-plane.sh
  # The supervisor MAC is what separates the fixture broker's parent from any
  # other local process. A silent dependency bump changes that code path
  # without changing a line here, so the reviewed graph is checked every run.
  run_step "owner-effect-crypto-profile" ./scripts/check-owner-effect-crypto-profile.sh
  # Clippy below runs on default features, so every fixture-gated module —
  # the whole broker and the whole owner crate — would otherwise never be
  # linted at all. Lint them under their own feature.
  run_step "clippy(owner-effect-fixture)" cargo clippy \
    -p sovereign-authority -p sovereign-owner -p sovereign-cli \
    -p sovereign-artifact -p sovereign-capability \
    -p sovereign-synthetic-owner-effect --all-targets \
    --features owner-effect-fixture --locked -- -D warnings
  run_step "owner-effect-profile-builds" ./scripts/check-owner-effect-profile-builds.sh
  # A real address reaching the fixture would put it in an unencrypted redb
  # file. Cheap to scan for, and the way it happens is someone pasting one in
  # while debugging and not taking it out.
  run_step "owner-effect-canaries" ./scripts/check-owner-effect-canaries.sh
  # Each secret-bearing type has a hand-written Debug and a test for it. This
  # is what keeps that true for the next one, which will otherwise reach for
  # derive because every other struct in the file has it.
  run_step "owner-effect-value-free" ./scripts/check-owner-effect-value-free.sh
  # The whole fixture is behind non-default features, which is an arrangement
  # rather than a guarantee: one crate naming the feature in its defaults puts
  # all of it in the release binary, and nothing about a normal day would show
  # it. This checks the artefact.
  run_step "owner-effect-boundary" ./scripts/check-owner-effect-boundary.sh
  # v2 upper crate: not a sovereign-cli dependency, lock-before-redb, one
  # listener. Distinct from the v1 feature-exclusion gate above.
  run_step "synthetic-owner-effect-boundary" env SKIP_RELEASE_BUILD=1 \
    ./scripts/check-synthetic-owner-effect-boundary.sh
  # Code gates catch code. This catches the other way the boundary erodes:
  # a sentence that is nearly right gets quoted, and later a decision rests
  # on it.
  run_step "owner-effect-documentation" ./scripts/check-owner-effect-documentation.sh
  # Every registered row, through the checked runner. A row naming a test that
  # no longer exists fails here rather than rotting, and CI runs the same
  # command against a clean checkout.
  run_step "owner-effect-manifest" ./scripts/run-owner-effect-tests.sh --all
  # A separate workspace, so nothing else in this gate builds it.
  run_step "owner-webauthn-adapter" cargo test \
    --manifest-path fixtures/owner-webauthn/Cargo.toml --locked
  run_step "qualify-vault-v2-selftest" env GATE_SELFTEST_RUNNING=1 \
    ./scripts/tests/qualify_vault_v2_test.sh
fi
if [ "${GATE_SELFTEST_RUNNING:-0}" != "1" ] && [ "$NEED_BROKER_BUILD" -eq 1 ]; then
  run_step "owner-effect-broker-build" ./scripts/check-owner-effect-broker-build.sh
fi
if [ "${GATE_SELFTEST_RUNNING:-0}" != "1" ] && [ "$NEED_AUTHORITY_PLANE" -eq 1 ]; then
  run_step "owner-effect-authority-plane" ./scripts/check-owner-effect-authority-plane.sh
fi
run_step "file-size" ./scripts/check-file-size.sh
run_step "fmt" cargo fmt --all --check

# ---- scoped clippy + tests ----------------------------------------------
RAN=""
if [ "$FULL" -eq 1 ]; then
  run_step "clippy(workspace)" cargo clippy --workspace --all-targets --locked -- -D warnings
  run_step "test(workspace)" cargo test --workspace --locked
  RAN="workspace"
elif [ -n "$PKGS" ]; then
  P_FLAGS=()
  for p in $PKGS; do P_FLAGS+=(-p "$p"); done
  run_step "clippy(scoped)" cargo clippy "${P_FLAGS[@]}" --all-targets --locked -- -D warnings
  run_step "test(scoped)" cargo test "${P_FLAGS[@]}" --locked
  RAN="${PKGS# }"
fi

QUALIFY_NOTE=""
NEED_VAULT_QUALIFY=0
if [ "$FULL" -eq 1 ]; then
  NEED_VAULT_QUALIFY=1
else
  case " $PKGS " in
  *" sovereign-vault-v2-engine "*) NEED_VAULT_QUALIFY=1 ;;
  esac
fi
if [ "$NEED_VAULT_QUALIFY" -eq 1 ]; then
  if rustc -vV 2>/dev/null | grep -q '^host: x86_64-unknown-linux-gnu'; then
    run_step "qualify-vault-v2(full)" ./scripts/qualify-vault-v2.sh full
    QUALIFY_NOTE=", vault-v2 qualification ok"
  else
    QUALIFY_NOTE=", SKIPPED qualify-vault-v2 (host is not x86_64-unknown-linux-gnu; CI covers it)"
  fi
fi

# ---- frontend type-check (environment-dependent: needs npx + network) ----
FE_NOTE=""
if [ "$FRONTEND" -eq 1 ] || [ "$FULL" -eq 1 ]; then
  if command -v npx >/dev/null 2>&1; then
    run_step "tsc(frontend)" env npm_config_ignore_scripts=true \
      npx -y -p typescript@5.5.4 tsc -p apps/cli/assets/tsconfig.json
    FE_NOTE=", frontend tsc ok"
  else
    # explicit skip, never silent: CI still runs this check on push
    FE_NOTE=", SKIPPED frontend tsc (npx not installed in this environment; CI covers it)"
  fi
fi

# The cheap gates above are unconditional, so reaching here without them is a
# bug in this script rather than a possible state — check anyway, since the
# whole point of this file is that a gate must not be able to lie.
case "$STEPS_RUN" in
*" file-size "* | *" file-size") ;;
*)
  echo "GATE BUG: reached the summary without running file-size" >&2
  exit 1
  ;;
esac

GATE_STATE=completed
if [ -z "$RAN" ] && [ "$FRONTEND" -eq 0 ]; then
  echo "test_changed: ALL GREEN — steps:$STEPS_RUN — no cargo test scope in this change set (full log: $LOG)"
else
  echo "test_changed: ALL GREEN — steps:$STEPS_RUN — scope: ${RAN:-frontend-only}${FE_NOTE}${QUALIFY_NOTE} (full log: $LOG)"
fi

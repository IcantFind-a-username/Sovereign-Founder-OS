#!/usr/bin/env bash
# RFC 0005 Program 1A Task 5 — isolated native key-store qualification on one
# exact host triple. Requires SFO_VAULT_PLATFORM_NAMESPACE=sfo-ci:<run>:<attempt>.
set -euo pipefail

if [ -z "${BASH_VERSINFO:-}" ] || [ "${BASH_VERSINFO[0]}" -lt 3 ] ||
  { [ "${BASH_VERSINFO[0]}" -eq 3 ] && [ "${BASH_VERSINFO[1]}" -lt 2 ]; }; then
  echo "run-vault-platform-qualifier: need bash 3.2+" >&2
  exit 3
fi

EXPECTED_TRIPLE="${1:-}"
if [ -z "$EXPECTED_TRIPLE" ]; then
  echo "usage: $0 <expected-rust-host-triple>" >&2
  exit 2
fi

case "$EXPECTED_TRIPLE" in
x86_64-unknown-linux-gnu | aarch64-apple-darwin | x86_64-pc-windows-msvc) ;;
*)
  echo "run-vault-platform-qualifier: triple not admitted by Program 1A Task 5: $EXPECTED_TRIPLE" >&2
  exit 2
  ;;
esac

ROOT=$(cd "$(dirname "$0")/.." && pwd)
cd "$ROOT" || exit 1

host=$(rustc -vV 2>/dev/null | sed -n 's/^host: //p' | tr -d '[:space:]')
if [ "$host" != "$EXPECTED_TRIPLE" ]; then
  echo "run-vault-platform-qualifier: HOST ($host) != EXPECTED ($EXPECTED_TRIPLE)" >&2
  exit 1
fi

if [ -z "${SFO_VAULT_PLATFORM_NAMESPACE:-}" ]; then
  echo "run-vault-platform-qualifier: SFO_VAULT_PLATFORM_NAMESPACE is required (sfo-ci:run:attempt)" >&2
  exit 1
fi
case "$SFO_VAULT_PLATFORM_NAMESPACE" in
sfo-ci:*) ;;
*)
  echo "run-vault-platform-qualifier: namespace must start with sfo-ci:" >&2
  exit 1
  ;;
esac

export SFO_VAULT_PLATFORM_ADMITTED_TARGET="$EXPECTED_TRIPLE"

run_qualification_cargo() {
  ./scripts/qualify-vault-v2.sh cargo "$@"
}

run_platform_tests() {
  run_qualification_cargo test -p sovereign-vault-v2-engine --bin sovereign-vault-v2-engine \
    --features platform-qualifier platform_qualification --frozen --offline \
    -- --nocapture --test-threads 1
}

run_linux_durability() {
  run_qualification_cargo test -p sovereign-vault-v2-engine --bin sovereign-vault-v2-engine \
    engine::storage::tests --frozen --offline -- --nocapture
}

run_inner() {
  run_platform_tests
  if [ "$EXPECTED_TRIPLE" = "x86_64-unknown-linux-gnu" ]; then
    run_linux_durability
  fi
  ./scripts/check-vault-v2-non-activation.sh
}

on_qualification_failure() {
  local log="$ROOT/.harness/qualify-vault-v2.log"
  if [ -f "$log" ]; then
    echo "run-vault-platform-qualifier: qualify-vault-v2.log (last 80 lines):" >&2
    tail -n 80 "$log" >&2
  fi
  exit 1
}

if [ "$EXPECTED_TRIPLE" = "x86_64-unknown-linux-gnu" ] && [ "${SFO_PLATFORM_KEYRING_READY:-}" != "1" ]; then
  if command -v apt-get >/dev/null 2>&1; then
    sudo apt-get update -qq
    sudo DEBIAN_FRONTEND=noninteractive apt-get install -y -qq \
      dbus dbus-x11 gnome-keyring libsecret-1-0 >/dev/null
  fi
  export SFO_PLATFORM_KEYRING_READY=1
  eval "$(dbus-launch --sh-syntax)"
  # Same unlock pattern as keyring 4.1.5 upstream CI (login keyring must be writable).
  gnome-keyring-daemon --components=secrets --daemonize --unlock <<< 'sfo-ci-platform-qual'
fi

if [ "$EXPECTED_TRIPLE" = "aarch64-apple-darwin" ] && [ "${SFO_PLATFORM_KEYCHAIN_READY:-}" != "1" ]; then
  export SFO_PLATFORM_KEYCHAIN_READY=1
  if command -v security >/dev/null 2>&1; then
    login_keychain="${HOME}/Library/Keychains/login.keychain-db"
    if [ ! -f "$login_keychain" ]; then
      login_keychain="${HOME}/Library/Keychains/login.keychain"
    fi
    if [ -f "$login_keychain" ]; then
      security unlock-keychain -p "" "$login_keychain" 2>/dev/null || true
      security set-keychain-settings -t 3600 -u "$login_keychain" 2>/dev/null || true
    fi
  fi
fi

run_inner || on_qualification_failure

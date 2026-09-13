#!/usr/bin/env bash
# Program 1A Task 6 — fast guards that vault-v2 stays non-product.
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
cd "$ROOT"

engine_toml="crates/vault-v2-engine/Cargo.toml"
grep -q 'publish = false' "$engine_toml" || {
  echo "check-vault-v2-non-activation: missing publish = false in $engine_toml" >&2
  exit 1
}

if grep -q 'sovereign-vault-v2-engine' apps/cli/Cargo.toml; then
  echo "check-vault-v2-non-activation: sovereign-cli must not depend on vault-v2-engine" >&2
  exit 1
fi

for phrase in "encrypted at rest" E2EE "production-ready"; do
  if grep -qi "$phrase" docs/security/vault-v2-verification.md; then
    echo "check-vault-v2-non-activation: forbidden claim in evidence ledger: $phrase" >&2
    exit 1
  fi
done

if rustc -vV 2>/dev/null | grep -q '^host: x86_64-unknown-linux-gnu'; then
  ./scripts/qualify-vault-v2.sh cargo test -p sovereign-vault-v2-engine --test non_activation \
    --frozen --offline
else
  cargo test -p sovereign-vault-v2-engine --test non_activation --locked
fi

echo "check-vault-v2-non-activation: OK"

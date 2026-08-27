#!/usr/bin/env bash
set -euo pipefail

ACTIONLINT_VERSION="1.7.12"
ACTIONLINT_LINUX_X86_64_SHA256="8aca8db96f1b94770f1b0d72b6dddcb1ebb8123cb3712530b08cc387b349a3d8"

if [[ "$(uname -s)" != "Linux" || "$(uname -m)" != "x86_64" ]]; then
  echo "pinned actionlint bootstrap supports Linux x86_64 only" >&2
  exit 1
fi

tool_dir="$(mktemp -d)"
trap 'rm -rf -- "$tool_dir"' EXIT
archive="$tool_dir/actionlint.tar.gz"
url="https://github.com/rhysd/actionlint/releases/download/v${ACTIONLINT_VERSION}/actionlint_${ACTIONLINT_VERSION}_linux_amd64.tar.gz"

curl --proto '=https' --tlsv1.2 --fail --silent --show-error --location \
  "$url" --output "$archive"
echo "$ACTIONLINT_LINUX_X86_64_SHA256  $archive" | sha256sum --check --status
tar --no-same-owner -xzf "$archive" -C "$tool_dir" actionlint
"$tool_dir/actionlint"

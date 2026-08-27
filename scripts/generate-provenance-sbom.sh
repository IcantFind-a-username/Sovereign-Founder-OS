#!/usr/bin/env bash
set -euo pipefail

SYFT_VERSION="1.51.0"
SYFT_LINUX_AMD64_SHA256="2a2e837a2c8d59ec9af5472ee22d3b04ee463c4e44476ecf993fd1e5ab6ebc7f"

if [[ $# -ne 2 ]]; then
  echo "usage: $0 INPUT_Cargo.lock OUTPUT.spdx.json" >&2
  exit 2
fi
if [[ "$(uname -s)" != "Linux" || "$(uname -m)" != "x86_64" ]]; then
  echo "pinned Syft bootstrap supports Linux x86_64 only" >&2
  exit 1
fi

input="$1"
output="$2"
if [[ ! -f "$input" || -L "$input" || "$(basename -- "$input")" != "Cargo.lock" ]]; then
  echo "SBOM input must be a regular non-symlink named Cargo.lock" >&2
  exit 1
fi
if [[ -e "$output" || -L "$output" ]]; then
  echo "refusing to overwrite SBOM output: $output" >&2
  exit 1
fi

tool_dir="$(mktemp -d)"
trap 'rm -rf -- "$tool_dir"' EXIT
archive="$tool_dir/syft.tar.gz"
url="https://github.com/anchore/syft/releases/download/v${SYFT_VERSION}/syft_${SYFT_VERSION}_linux_amd64.tar.gz"
input_dir="$(dirname -- "$(realpath -- "$input")")"
output="$(realpath -m -- "$output")"

curl --proto '=https' --tlsv1.2 --fail --silent --show-error --location \
  "$url" --output "$archive"
echo "$SYFT_LINUX_AMD64_SHA256  $archive" | sha256sum --check --status
tar --no-same-owner -xzf "$archive" -C "$tool_dir" syft
(
  cd "$input_dir"
  "$tool_dir/syft" scan file:Cargo.lock --output "spdx-json=$output"
)

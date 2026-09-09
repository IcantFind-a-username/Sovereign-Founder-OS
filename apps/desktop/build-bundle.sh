#!/usr/bin/env bash
# Build the desktop app bundle.
#
# The shell does not contain the runtime: it launches the audited `sovereign`
# binary as a child. So the bundle needs that binary, built from the core
# workspace and staged under the target triple name Tauri expects.
#
#   ./apps/desktop/build-bundle.sh            # .app + .dmg
#   ./apps/desktop/build-bundle.sh --debug    # faster, unoptimized
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

# Plain strings, not arrays: under `set -u` on bash 3.2 (the stock macOS
# shell) expanding an empty array is an "unbound variable" error, and each
# flag here is a single word with no spaces.
PROFILE=release
CARGO_PROFILE_FLAG=--release
TAURI_FLAG=
DMG=
for arg in "$@"; do
  case "$arg" in
  --debug)
    PROFILE=debug
    CARGO_PROFILE_FLAG=
    TAURI_FLAG=--debug
    ;;
  --dmg) DMG=1 ;;
  *)
    echo "unknown argument: $arg" >&2
    exit 2
    ;;
  esac
done

PRODUCT="Sovereign Founder OS"
TRIPLE=$(rustc -vV | awk '/^host:/ {print $2}')
echo "==> building the runtime for $TRIPLE ($PROFILE)"
# shellcheck disable=SC2086  # deliberately word-split: the flag may be empty
cargo build -p sovereign-cli $CARGO_PROFILE_FLAG --locked

echo "==> staging the runtime as the bundle sidecar"
mkdir -p apps/desktop/binaries
cp "target/$PROFILE/sovereign" "apps/desktop/binaries/sovereign-$TRIPLE"
chmod +x "apps/desktop/binaries/sovereign-$TRIPLE"

echo "==> bundling the desktop shell"
cd apps/desktop
# shellcheck disable=SC2086  # deliberately word-split: the flag may be empty
npx --yes @tauri-apps/cli@2 build $TAURI_FLAG

if [ -n "$DMG" ]; then
  # Built with hdiutil rather than Tauri's bundler on purpose: that one styles
  # the disk-image window through Finder, and the AppleScript step hangs
  # forever unless the calling shell already holds Automation access to
  # Finder. hdiutil needs no permission and produces the same drag-to-install
  # image: the app plus a shortcut to /Applications.
  APP_PATH="target/$PROFILE/bundle/macos/$PRODUCT.app"
  DMG_PATH="target/$PROFILE/bundle/dmg/$PRODUCT.dmg"
  STAGE=$(mktemp -d)
  trap 'rm -rf "$STAGE"' EXIT
  echo "==> building the disk image"
  cp -R "$APP_PATH" "$STAGE/"
  ln -s /Applications "$STAGE/Applications"
  mkdir -p "$(dirname "$DMG_PATH")"
  rm -f "$DMG_PATH"
  hdiutil create -volname "$PRODUCT" -srcfolder "$STAGE" -ov -format UDZO \
    -quiet "$DMG_PATH"
fi

echo
echo "Bundles:"
find target -maxdepth 4 \( -name '*.dmg' -o -name '*.app' \) -print 2>/dev/null | sed 's/^/  /'
echo
echo "The bundle is unsigned: macOS quarantines a downloaded copy until you"
echo "right-click -> Open once, or clear the flag with"
echo "  xattr -dr com.apple.quarantine '<path to the .app>'"

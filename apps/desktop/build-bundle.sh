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

APP_BUNDLE="target/$PROFILE/bundle/macos/$PRODUCT.app"

# Seal the bundle with an ad-hoc signature.
#
# Without this the only signature present is the one the linker put on the
# inner executable, so the bundle reports "code has no resources but signature
# indicates they must be present": Info.plist is unbound and no resources are
# sealed. Gatekeeper treats that as a broken signature and tells the user the
# app is *damaged*, which is both alarming and untrue. Signing inner binaries
# first and the bundle last is the supported order; `--deep` is deprecated for
# signing.
#
# Ad-hoc is not Developer ID: a downloaded copy still needs one right-click ->
# Open, or the quarantine flag cleared. It only makes the bundle internally
# valid so the failure is the honest "unidentified developer" one.
echo "==> ad-hoc signing the bundle"
codesign --force --sign - --timestamp=none "$APP_BUNDLE/Contents/MacOS/sovereign"
codesign --force --sign - --timestamp=none "$APP_BUNDLE"
codesign --verify --deep --strict --verbose=2 "$APP_BUNDLE"

if [ -n "$DMG" ]; then
  # Built with hdiutil rather than Tauri's bundler on purpose: that one styles
  # the disk-image window through Finder, and the AppleScript step hangs
  # forever unless the calling shell already holds Automation access to
  # Finder. hdiutil needs no permission and produces the same drag-to-install
  # image: the app plus a shortcut to /Applications.
  DMG_PATH="target/$PROFILE/bundle/dmg/$PRODUCT.dmg"
  STAGE=$(mktemp -d)
  trap 'rm -rf "$STAGE"' EXIT
  echo "==> building the disk image"
  # `ditto`, not `cp -R`: it preserves the extended attributes and resource
  # layout a signed bundle depends on. `cp -R` can invalidate the signature
  # it just took to produce.
  ditto "$APP_BUNDLE" "$STAGE/$PRODUCT.app"
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
echo "Ad-hoc signed, not notarized. Drag the app into /Applications, then clear"
echo "the quarantine flag on a copy that arrived over the network:"
echo "  xattr -dr com.apple.quarantine '/Applications/$PRODUCT.app'"
echo "(macOS 15 and later removed the right-click -> Open bypass; the other way"
echo " is System Settings > Privacy & Security > Open Anyway.)"

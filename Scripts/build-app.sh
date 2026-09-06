#!/bin/bash
# Builds Vigil.app from the Swift package. No Xcode project needed.
#   Scripts/build-app.sh [debug|release]
set -euo pipefail
cd "$(dirname "$0")/.."
CONFIG="${1:-release}"
APP_NAME="Vigil"
OUT_DIR="dist"
APP="$OUT_DIR/$APP_NAME.app"

swift build -c "$CONFIG" 2>&1 | tail -n 5
BIN="$(swift build -c "$CONFIG" --show-bin-path)/$APP_NAME"

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BIN" "$APP/Contents/MacOS/$APP_NAME"
cp Resources/Info.plist "$APP/Contents/Info.plist"
if [ -f Resources/AppIcon.icns ]; then cp Resources/AppIcon.icns "$APP/Contents/Resources/AppIcon.icns"; fi
# SwiftPM resource bundle (hook script template etc.)
BUNDLE_DIR="$(dirname "$BIN")/${APP_NAME}_${APP_NAME}.bundle"
if [ -d "$BUNDLE_DIR" ]; then cp -R "$BUNDLE_DIR" "$APP/Contents/Resources/"; fi
echo "APPL????" > "$APP/Contents/PkgInfo"

# Ad-hoc signature so notifications and login items work locally.
codesign --force --deep --sign - "$APP" >/dev/null 2>&1 || true
echo "Built $APP"

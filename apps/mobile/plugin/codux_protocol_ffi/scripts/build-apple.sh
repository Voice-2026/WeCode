#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
PLUGIN_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
find_repo_root() {
  local dir="$PLUGIN_DIR"
  while [[ "$dir" != "/" ]]; do
    if [[ -f "$dir/Cargo.toml" ]] && grep -q '^\[workspace\]' "$dir/Cargo.toml"; then
      printf '%s\n' "$dir"
      return 0
    fi
    dir="$(dirname "$dir")"
  done
  return 1
}
REPO_ROOT="$(find_repo_root)"
PLATFORM_NAME="${PLATFORM_NAME:-macosx}"
ARCHS="${ARCHS:-arm64}"
CONFIGURATION="${CONFIGURATION:-Release}"

PROFILE_FLAG="--release"
TARGET_DIR="release"
if [[ "$CONFIGURATION" == "Debug" ]]; then
  PROFILE_FLAG=""
  TARGET_DIR="debug"
fi

case "$PLATFORM_NAME" in
  iphoneos)
    TARGET="aarch64-apple-ios"
    OUT_DIR="$PLUGIN_DIR/ios/Frameworks"
    ;;
  iphonesimulator)
    if [[ "$ARCHS" == *"x86_64"* ]]; then
      TARGET="x86_64-apple-ios"
    else
      TARGET="aarch64-apple-ios-sim"
    fi
    OUT_DIR="$PLUGIN_DIR/ios/Frameworks"
    ;;
  macosx)
    if [[ "$ARCHS" == *"x86_64"* ]]; then
      TARGET="x86_64-apple-darwin"
    else
      TARGET="aarch64-apple-darwin"
    fi
    OUT_DIR="$PLUGIN_DIR/macos/Frameworks"
    ;;
  *)
    echo "Unsupported Apple platform: $PLATFORM_NAME" >&2
    exit 2
    ;;
esac

cd "$REPO_ROOT"
rustup target add "$TARGET" >/dev/null
cargo build -p codux-protocol-ffi --target "$TARGET" $PROFILE_FLAG
mkdir -p "$OUT_DIR"
cp "$REPO_ROOT/target/$TARGET/$TARGET_DIR/libcodux_protocol_ffi.a" \
  "$OUT_DIR/libcodux_protocol_ffi.a"

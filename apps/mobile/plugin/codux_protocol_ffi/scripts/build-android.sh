#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
REPO_ROOT="$(cd "$PLUGIN_DIR/../../../.." && pwd)"
SDK_ROOT="${ANDROID_SDK_ROOT:-${ANDROID_HOME:-$HOME/Library/Android/sdk}}"

if [[ -z "${ANDROID_NDK_HOME:-}" ]]; then
  if [[ -d "$SDK_ROOT/ndk" ]]; then
    ANDROID_NDK_HOME="$(find "$SDK_ROOT/ndk" -mindepth 1 -maxdepth 1 -type d | sort -V | tail -1)"
    export ANDROID_NDK_HOME
  fi
fi

if [[ -z "${ANDROID_NDK_HOME:-}" || ! -d "$ANDROID_NDK_HOME" ]]; then
  echo "ANDROID_NDK_HOME is not set and no NDK was found under $SDK_ROOT/ndk" >&2
  exit 2
fi

if ! command -v cargo-ndk >/dev/null 2>&1; then
  echo "cargo-ndk is required. Install with: cargo install cargo-ndk" >&2
  exit 2
fi

cd "$REPO_ROOT"
rm -f \
  "$PLUGIN_DIR/android/src/main/jniLibs/arm64-v8a/libc++_shared.so" \
  "$PLUGIN_DIR/android/src/main/jniLibs/arm64-v8a/libghostty-vt.so.0"

cargo ndk \
  -t arm64-v8a \
  -o "$PLUGIN_DIR/android/src/main/jniLibs" \
  build -p codux-protocol-ffi --release

copy_ghostty_vt() {
  local target="$1"
  local abi="$2"
  local ghostty_lib
  ghostty_lib="$(
    find "$REPO_ROOT/target/$target/release/build" \
      -path '*/out/android-link-lib/libghostty-vt.so' \
      -size +1M \
      -print \
      | sort \
      | tail -1
  )"
  if [[ -z "$ghostty_lib" || ! -f "$ghostty_lib" ]]; then
    echo "libghostty-vt.so not found for $target under $REPO_ROOT/target" >&2
    exit 2
  fi
  cp "$ghostty_lib" "$PLUGIN_DIR/android/src/main/jniLibs/$abi/libghostty-vt.so"
}

copy_ghostty_vt "aarch64-linux-android" "arm64-v8a"

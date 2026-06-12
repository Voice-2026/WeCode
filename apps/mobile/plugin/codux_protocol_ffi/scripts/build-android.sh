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
cargo ndk \
  -t arm64-v8a \
  -o "$PLUGIN_DIR/android/src/main/jniLibs" \
  build -p codux-protocol-ffi --release

copy_libcxx_shared() {
  local android_triple="$1"
  local abi="$2"
  local src="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt"
  local host_dir
  host_dir="$(find "$src" -mindepth 1 -maxdepth 1 -type d | sort | head -1)"
  local libcxx="$host_dir/sysroot/usr/lib/$android_triple/libc++_shared.so"
  if [[ ! -f "$libcxx" ]]; then
    echo "libc++_shared.so not found for $android_triple under $ANDROID_NDK_HOME" >&2
    exit 2
  fi
  cp "$libcxx" "$PLUGIN_DIR/android/src/main/jniLibs/$abi/libc++_shared.so"
}

copy_libcxx_shared "aarch64-linux-android" "arm64-v8a"

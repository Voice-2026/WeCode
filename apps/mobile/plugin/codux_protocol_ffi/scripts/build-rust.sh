#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
REPO_ROOT="$(cd "$PLUGIN_DIR/../../../.." && pwd)"
TARGET="${1:-}"
PROFILE="${PROFILE:-release}"

if [[ -z "$TARGET" ]]; then
  echo "usage: build-rust.sh <rust-target|android>" >&2
  exit 2
fi

if [[ "$TARGET" == "android" ]]; then
  "$SCRIPT_DIR/build-android.sh"
  exit 0
fi

if [[ "$TARGET" == "apple" ]]; then
  "$SCRIPT_DIR/build-apple.sh"
  exit 0
fi

PROFILE_FLAG=""
TARGET_DIR="debug"
if [[ "$PROFILE" == "release" ]]; then
  PROFILE_FLAG="--release"
  TARGET_DIR="release"
fi

cargo build -p codux-protocol-ffi --target "$TARGET" $PROFILE_FLAG
echo "$REPO_ROOT/target/$TARGET/$TARGET_DIR"

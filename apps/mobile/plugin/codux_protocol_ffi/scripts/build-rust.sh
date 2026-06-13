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

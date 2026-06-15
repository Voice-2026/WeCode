#!/bin/sh
set -eu

root_dir="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
cd "$root_dir"

if [ "$(uname -s)" = "Darwin" ]; then
    cargo build -p codux
    dev_bin="$root_dir/target/debug/Codux Dev"
    cp "$root_dir/target/debug/codux" "$dev_bin"
    exec "$dev_bin" "$@"
fi

exec cargo run -p codux -- "$@"

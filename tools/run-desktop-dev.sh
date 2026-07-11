#!/bin/sh
set -eu

root_dir="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
cd "$root_dir"

normalize_path_var() {
    var_name="$1"
    eval "value=\${$var_name:-}"
    [ -n "$value" ] || return 0

    normalized=""
    old_ifs="$IFS"
    IFS=:
    for entry in $value; do
        [ -n "$entry" ] || continue
        case ":$normalized:" in
            *:"$entry":*) ;;
            *)
                if [ -n "$normalized" ]; then
                    normalized="$normalized:$entry"
                else
                    normalized="$entry"
                fi
                ;;
        esac
    done
    IFS="$old_ifs"

    export "$var_name=$normalized"
}

normalize_path_var PKG_CONFIG_PATH

if [ "$(uname -s)" = "Darwin" ]; then
    cargo build -p wecode
    dev_bin="$root_dir/target/debug/WeCode Dev"
    cp "$root_dir/target/debug/wecode" "$dev_bin"
    exec "$dev_bin" "$@"
fi

exec cargo run -p wecode -- "$@"

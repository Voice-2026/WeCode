set shell := ["sh", "-eu", "-c"]

default:
    just --list

desktop *args:
    cargo run -p codux -- {{args}}

server *args:
    cargo run -p codux-server -- {{args}}

agent *args:
    cargo run -p codux-agent -- {{args}}

mobile *args:
    cd apps/mobile && \
    set -- {{args}}; \
    platform="${1:-android}"; \
    case "$platform" in \
      android|ios) shift || true ;; \
      *) platform="" ;; \
    esac; \
    if [ -n "$platform" ]; then \
        device_id="$(flutter devices --machine | ruby -rjson -e 'platform = ARGV[0]; devices = JSON.parse(STDIN.read); device = devices.find { |item| item["isSupported"] && item["targetPlatform"].to_s.start_with?(platform) }; print(device ? device["id"] : "")' "$platform")"; \
        if [ -n "$device_id" ]; then \
            echo "Using $platform device: $device_id"; \
            flutter run -d "$device_id" "$@"; \
        else \
            echo "No $platform device found. Falling back to flutter run."; \
            flutter run "$@"; \
        fi; \
    else \
        flutter run "$@"; \
    fi

check:
    cargo check --workspace
    cd apps/mobile && flutter analyze

test:
    cargo test --workspace
    cd apps/mobile && flutter test

ffi:
    cargo build -p codux-protocol-ffi

smoke:
    cargo run -p codux-agent -- --pty-smoke
    cargo run -p codux-agent -- --transport-smoke

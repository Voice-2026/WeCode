# Codux Shared Crates

This directory contains cross-platform Rust crates used by desktop, mobile FFI, server, and headless targets. App-specific UI and packaging belongs in `../apps`.

## Crates

| Crate | Role |
| --- | --- |
| `codux-protocol` | v3.1 protocol constants, relay envelope DTOs, transport candidates, capabilities, resource subscription helpers, and terminal buffer payload rules. |
| `codux-remote-transport` | Shared transport layer: WebSocket relay, WebRTC host/controller direct and fallback paths, local memory transport, URL/STUN normalization, and transport factory rules. |
| `codux-protocol-ffi` | C ABI consumed by Flutter. Exposes protocol helpers, controller transport handles, terminal-core session state, and output sequencing. |
| `codux-runtime-core` | Shared runtime-domain payload rules and subscription router for host.info, project, file, Git, worktree, upload, and terminal domains. |
| `codux-terminal-core` | Platform-neutral terminal traits and remote terminal state: sequence, baseline restore, cache trimming, and held-live replay. |
| `codux-terminal-pty` | Shared local PTY driver built on `portable_pty`, suitable for headless hosts and future desktop delegation. |

## Rules

- Shared behavior goes here before app code grows duplicate logic.
- Crates must not depend on GPUI, Flutter, or app-specific UI state.
- Protocol and transport crates must not own business runtime state.
- Terminal core must not launch processes; process launching belongs in `codux-terminal-pty`.
- FFI APIs should expose opaque handles and JSON payloads where ownership crosses Dart/Rust boundaries.

## Commands

```bash
cargo test --workspace
cargo test -p codux-protocol
cargo test -p codux-remote-transport
cargo test -p codux-protocol-ffi
cargo test -p codux-runtime-core
cargo test -p codux-terminal-core
cargo test -p codux-terminal-pty
```

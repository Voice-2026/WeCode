# WeCode Shared Crates

This directory contains cross-platform Rust crates used by desktop, mobile FFI, server, and headless targets. App-specific UI and packaging belongs in `../apps`.

## Crates

| Crate | Role |
| --- | --- |
| `wecode-protocol` | v3.2 protocol constants, relay envelope DTOs, transport candidates, capabilities, resource subscription helpers, and terminal buffer payload rules. |
| `wecode-remote-transport` | Shared transport layer: Iroh host/controller links, local memory transport, relay URL normalization for pairing tickets, and transport factory rules. |
| `wecode-protocol-ffi` | C ABI consumed by Flutter. Exposes protocol helpers, controller transport handles, terminal-core session state, and output sequencing. |
| `wecode-runtime-core` | Shared runtime-domain payload rules and subscription router for host.info, project, file, Git, worktree, upload, and terminal domains. |
| `wecode-terminal-core` | Platform-neutral terminal traits and remote terminal state: sequence, baseline restore, cache trimming, and held-live replay. |
| `wecode-terminal-pty` | Shared local PTY driver built on `portable_pty`, suitable for headless hosts and future desktop delegation. |

## Rules

- Shared behavior goes here before app code grows duplicate logic.
- Crates must not depend on GPUI, Flutter, or app-specific UI state.
- Protocol and transport crates must not own business runtime state.
- Protocol changes must update `../docs/protocol.md`.
- Terminal core must not launch processes; process launching belongs in `wecode-terminal-pty`.
- FFI APIs should expose opaque handles and JSON payloads where ownership crosses Dart/Rust boundaries.

## Commands

```bash
cargo test --workspace
cargo test -p wecode-protocol
cargo test -p wecode-remote-transport
cargo test -p wecode-protocol-ffi
cargo test -p wecode-runtime-core
cargo test -p wecode-terminal-core
cargo test -p wecode-terminal-pty
```

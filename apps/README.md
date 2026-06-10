# Codux Apps

This directory contains runnable Codux applications. Shared protocol, runtime, terminal, and transport code lives in `../crates`; app directories should only own product-specific UI, packaging, platform integration, or process entry points.

## Applications

| Path | Role | Runtime |
| --- | --- | --- |
| `desktop/` | Rust + GPUI desktop app. Owns the primary UI, local workspace orchestration, AI CLI sessions, local terminal adapter, and host-side remote runtime. | Rust |
| `mobile/` | Flutter mobile controller. Connects to a Codux host, renders remote runtime state, and sends user intent. | Flutter + Rust FFI |
| `agent/` | Headless controlled-agent entry point. Uses shared protocol, transport, runtime, and PTY crates without GPUI. | Rust |
| `server/` | Rust v3 relay service for pairing tickets, signaling, and WebSocket fallback routing. | Rust |
| `relay-server/` | Go relay service kept during migration for legacy deployment compatibility. | Go |

## Commands

Run from the repository root:

```bash
just desktop
just mobile android
just mobile ios
just server
just agent -- --version
just test
```

Use app-local commands only when working inside that app's native toolchain, such as `flutter test` in `apps/mobile` or `go test ./...` in `apps/relay-server`.

## Ownership Rules

- UI code belongs in an app directory.
- Shared protocol names, payload shapes, transport rules, terminal state, and reusable runtime models belong in `../crates`.
- Do not duplicate WebSocket/WebRTC URL selection, terminal sequence handling, or remote PTY restore logic in app code.
- Keep generated build output out of version control.
